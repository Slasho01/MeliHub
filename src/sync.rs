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
pub async fn access_token_valido(pool: &web::Data<DbPool>, client: &MlClient, cuenta: &MlCuenta) -> ApiResult<String> {
    if cuenta.token_expira_at > now() + Duration::seconds(60) {
        return Ok(cuenta.access_token.clone());
    }
    let tokens = client.refresh(&cuenta.refresh_token).await?;
    let expira = now() + Duration::seconds(tokens.expires_in.max(0));
    let refresh = if tokens.refresh_token.is_empty() { cuenta.refresh_token.clone() } else { tokens.refresh_token.clone() };
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
    let mut cat_cache: std::collections::HashMap<String, Option<String>> = std::collections::HashMap::new();
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
                        let name = client.get_category(&token, &cat_id).await;
                        cat_cache.insert(cat_id.clone(), name);
                    }
                    it.categoria_nombre = cat_cache.get(&cat_id).cloned().flatten();
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
            it.categoria_nombre = client.get_category(&token, &cat_id).await;
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
