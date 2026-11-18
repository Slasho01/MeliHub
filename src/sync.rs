//! Lógica de sincronización reutilizable por el controlador (manual), el
//! webhook (en vivo) y el job periódico (red de seguridad).

use crate::db::{self, DbPool};
use crate::errors::ApiResult;
use crate::ml::MlClient;
use crate::models::MlCuenta;
use crate::services::common::now;
use crate::services::{ml_services as svc, producto_services as prod_svc, venta_services as venta_svc};
use actix_web::web;
use chrono::Duration;

/// Devuelve un access_token válido, renovándolo (y persistiéndolo) si expira pronto.
/// Empuja a Mercado Libre el nuevo stock tras una venta externa (p. ej. Falabella),
/// para mantener sincronizados todos los canales. Best-effort.
pub async fn empujar_stock_ml(pool: &web::Data<DbPool>, client: &MlClient, venta: &crate::services::pub_services::VentaAplicada) -> bool {
    let cid = venta.ml_cuenta_id;
    let cuenta = match db::run(pool, move |conn| svc::cuenta_por_id(conn, cid)).await {
        Ok(Some(c)) => c,
        _ => return false,
    };
    let Ok(token) = access_token_valido(pool, client, &cuenta).await else { return false };
    let res = match &venta.user_product_id {
        Some(upid) => client.update_deposito_stock(&token, upid, venta.nuevo_stock).await,
        None => client.update_stock(&token, &venta.ml_item_id, venta.nuevo_stock).await,
    };
    match res {
        Ok(()) => true,
        Err(e) => {
            log::warn!("No se pudo empujar stock a ML ({}): {e}", venta.ml_item_id);
            let cid = venta.ml_cuenta_id;
            let _ = db::run(pool, move |conn| {
                use crate::schema::ml_cuentas;
                use diesel::prelude::*;
                if let Ok(vid) = ml_cuentas::table.find(cid).select(ml_cuentas::vendedor_id).first::<uuid::Uuid>(conn) {
                    crate::services::saas_services::notificar_una_vez(conn, vid, "sync_error", "No se pudo actualizar el stock en Mercado Libre", Some("Revisa la conexión de tu cuenta de ML (quizá haya que reconectarla). Mientras no se resuelva, el stock en ML puede quedar desactualizado y causar sobreventa.".into()));
                }
                Ok::<(), crate::errors::ApiError>(())
            })
            .await;
            false
        }
    }
}

/// Empuja el stock (y precio) de UN producto a todos los canales donde está publicado
/// (Falabella, Paris, Shopify). Se usa tras un webhook de ML para propagar en segundos,
/// sin esperar al ciclo periódico. Best-effort: los errores se loguean y no cortan.
pub async fn empujar_producto_a_canales(
    pool: &web::Data<DbPool>,
    producto_id: uuid::Uuid,
    fala: &crate::falabella::FalabellaClient,
    paris: &crate::paris::ParisClient,
) {
    // Falabella (stock + precio).
    if let Ok(rows) = db::run(pool, move |conn| crate::services::falabella_services::sync_de_producto(conn, producto_id)).await {
        for (cuenta, sku, precio, stock, op) in rows {
            let api_key = crate::crypto::descifrar(&cuenta.api_key);
            let xml = crate::services::falabella_push_services::build_stock_precio_xml(&sku, precio, stock, &op);
            if let Err(e) = fala.product_update(&cuenta.user_id, &api_key, xml).await {
                log::warn!("Webhook push Falabella ({sku}): {e}");
            }
        }
    }
    // Paris (stock por variante).
    if let Ok(rows) = db::run(pool, move |conn| crate::services::paris_services::sync_de_producto(conn, producto_id)).await {
        for (cuenta, psku, stock) in rows {
            let api_key = crate::crypto::descifrar(&cuenta.api_key);
            let body = crate::services::paris_push_services::build_stock(&[(format!("{psku}-1"), stock)]);
            if let Err(e) = paris.update_stock_sku_seller(cuenta.sandbox, &api_key, &body).await {
                log::warn!("Webhook push Paris ({psku}): {e}");
            }
        }
    }
    // Shopify (stock + precio).
    if let Ok(rows) = db::run(pool, move |conn| crate::services::shopify_services::sync_de_producto(conn, producto_id)).await {
        let client = crate::shopify::ShopifyClient::new();
        for (cuenta, vid, iid, precio, stock) in rows {
            let token = crate::crypto::descifrar(&cuenta.access_token);
            if let Some(v) = &vid {
                let _ = client.actualizar_variante_precio(&cuenta.shop_domain, &token, v, precio).await;
            }
            if let Some(inv) = &iid {
                if let Ok(Some(l)) = client.ubicacion_principal(&cuenta.shop_domain, &token).await {
                    let _ = client.set_inventario(&cuenta.shop_domain, &token, inv, l, stock).await;
                }
            }
        }
    }
}

pub async fn access_token_valido(pool: &web::Data<DbPool>, client: &MlClient, cuenta: &MlCuenta) -> ApiResult<String> {
    if cuenta.token_expira_at > now() + Duration::seconds(60) {
        return Ok(crate::crypto::descifrar(&cuenta.access_token));
    }
    let refresh_plano = crate::crypto::descifrar(&cuenta.refresh_token);
    let tokens = match client.refresh(&refresh_plano).await {
        Ok(t) => t,
        Err(e) => {
            // Token/refresh muerto: la cuenta quedó desconectada → nada se sincroniza. Avisar
            // (in-app + email, una sola vez por cada episodio gracias al dedup).
            let vid = cuenta.vendedor_id;
            let nick = cuenta.nickname.clone().unwrap_or_default();
            let correo: Option<String> = db::run(pool, move |conn| {
                let cuerpo = format!(
                    "No pudimos renovar la sesión de tu cuenta de Mercado Libre{}. Ve a «Cuentas» y vuelve a conectarla para que el stock y las ventas se sigan sincronizando (si no, el stock puede desactualizarse y causar sobreventa).",
                    if nick.is_empty() { String::new() } else { format!(" ({nick})") }
                );
                let nueva = crate::services::saas_services::notificar_una_vez(conn, vid, "reconectar", "Reconecta tu cuenta de Mercado Libre", Some(cuerpo));
                let email = if nueva {
                    use crate::schema::vendedores;
                    use diesel::prelude::*;
                    vendedores::table.find(vid).select(vendedores::email).first::<String>(conn).ok()
                } else {
                    None
                };
                Ok::<Option<String>, crate::errors::ApiError>(email)
            })
            .await
            .ok()
            .flatten();
            if let Some(to) = correo {
                let app = std::env::var("APP_URL").unwrap_or_default();
                let cuerpo = format!(
                    "<h2 style=\"margin:0 0 12px;color:#13233b;font-size:21px\">Reconecta tu cuenta de Mercado Libre</h2>\
                     <p>Tu sesión de Mercado Libre expiró y no pudimos renovarla. Mientras esté desconectada, <b>el stock y las ventas no se sincronizan</b> (riesgo de sobreventa).</p>{}",
                    crate::email::boton("Ir a Cuentas", &format!("{app}/cuentas"))
                );
                let html = crate::email::plantilla(&cuerpo);
                let _ = crate::email::enviar_sistema(pool, &to, "Reconecta tu cuenta de Mercado Libre · Ventoki", &html).await;
            }
            return Err(e);
        }
    };
    let expira = now() + Duration::seconds(tokens.expires_in.max(0));
    let refresh = if tokens.refresh_token.is_empty() { refresh_plano } else { tokens.refresh_token.clone() };
    let access = tokens.access_token.clone();
    let cuenta_id = cuenta.id;
    let (a, r) = (access.clone(), refresh.clone());
    db::run(pool, move |conn| svc::actualizar_tokens(conn, cuenta_id, &a, &r, expira)).await?;
    Ok(access)
}

/// Importa/actualiza TODAS las publicaciones de una cuenta. Devuelve cuántas.
pub async fn sincronizar_cuenta(pool: &web::Data<DbPool>, client: &MlClient, cuenta: &MlCuenta) -> ApiResult<usize> {
    let token = access_token_valido(pool, client, cuenta).await?;
    let mut offset = 0i64;
    let mut total = 0usize;
    // Cachés: nombres de categoría y stock de depósito (varios ítems comparten).
    let mut cat_cache: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    let mut dep_cache: std::collections::HashMap<String, Option<i64>> = std::collections::HashMap::new();
    loop {
        let search = client.list_item_ids(&token, cuenta.ml_user_id, offset).await?;
        if search.results.is_empty() {
            break;
        }
        for chunk in search.results.chunks(20) {
            let mut items = client.get_items(&token, chunk).await?;
            // La descripción y la categoría son endpoints aparte: los traemos por ítem (best-effort).
            for it in items.iter_mut() {
                it.descripcion = client.get_description(&token, &it.id).await;
                if let Some(cat_id) = it.category_id.clone() {
                    if !cat_cache.contains_key(&cat_id) {
                        let ruta = client.get_category_ruta(&token, &cat_id).await;
                        cat_cache.insert(cat_id.clone(), ruta);
                    }
                    if let Some(ruta) = cat_cache.get(&cat_id) {
                        it.categoria_nombre = ruta.last().cloned();
                        it.categoria_ruta = ruta.clone();
                    }
                }
                if let Some(upid) = it.user_product_id.clone() {
                    if !dep_cache.contains_key(&upid) {
                        let d = client.get_deposito_stock(&token, &upid).await;
                        dep_cache.insert(upid.clone(), d);
                    }
                    it.deposito_stock = dep_cache.get(&upid).cloned().flatten();
                }
            }
            let cuenta_id = cuenta.id;
            total += db::run(pool, move |conn| prod_svc::upsert_items(conn, cuenta_id, &items)).await?;
        }
        offset += search.results.len() as i64;
        if offset >= search.paging.total || search.results.len() < 50 {
            break;
        }
    }
    let cuenta_id = cuenta.id;
    db::run(pool, move |conn| svc::marcar_sync(conn, cuenta_id)).await?;
    Ok(total)
}

/// Actualiza una sola publicación (usado por el webhook).
pub async fn refrescar_item(pool: &web::Data<DbPool>, client: &MlClient, cuenta: &MlCuenta, item_id: &str) -> ApiResult<usize> {
    let token = access_token_valido(pool, client, cuenta).await?;
    let mut items = client.get_items(&token, &[item_id.to_string()]).await?;
    for it in items.iter_mut() {
        it.descripcion = client.get_description(&token, &it.id).await;
        if let Some(cat_id) = it.category_id.clone() {
            let ruta = client.get_category_ruta(&token, &cat_id).await;
            it.categoria_nombre = ruta.last().cloned();
            it.categoria_ruta = ruta;
        }
        if let Some(upid) = it.user_product_id.clone() {
            it.deposito_stock = client.get_deposito_stock(&token, &upid).await;
        }
    }
    let cuenta_id = cuenta.id;
    db::run(pool, move |conn| prod_svc::upsert_items(conn, cuenta_id, &items)).await
}

/// Importa las órdenes (ventas) recientes de una cuenta. Devuelve cuántas.
pub async fn sincronizar_ordenes(pool: &web::Data<DbPool>, client: &MlClient, cuenta: &MlCuenta) -> ApiResult<usize> {
    let token = access_token_valido(pool, client, cuenta).await?;
    let mut offset = 0i64;
    let mut total = 0usize;
    loop {
        let search = client.search_orders(&token, cuenta.ml_user_id, offset).await?;
        let results = search.get("results").and_then(|r| r.as_array()).cloned().unwrap_or_default();
        if results.is_empty() {
            break;
        }
        let cuenta_id = cuenta.id;
        total += db::run(pool, move |conn| venta_svc::upsert_ordenes(conn, cuenta_id, &results)).await?;
        offset += 50;
        let paging_total = search.get("paging").and_then(|p| p.get("total")).and_then(|t| t.as_i64()).unwrap_or(0);
        if offset >= paging_total || offset >= 1000 {
            break; // tope de seguridad (hasta ~1000 órdenes por sync)
        }
    }
    Ok(total)
}
