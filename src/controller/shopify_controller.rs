//! Shopify como canal: conectar la tienda (verifica el token), listar/desconectar, y publicar
//! productos del hub en la tienda.

use crate::auth::AuthVendedor;
use crate::config::Config;
use crate::db::{self, DbPool};
use crate::errors::{ApiError, ApiResult};
use crate::models::Producto;
use crate::schema::{ml_cuentas, productos};
use crate::services::shopify_services;
use crate::shopify::{build_producto, ShopifyClient};
use actix_web::{web, HttpResponse};
use diesel::prelude::*;
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

fn id_str(v: Option<&Value>) -> Option<String> {
    v.and_then(|x| x.as_i64().map(|n| n.to_string()).or_else(|| x.as_str().map(|s| s.to_string())))
}

/// Imágenes del producto (desde datos.pictures), alta calidad y https.
fn imagenes_de(p: &Producto) -> Vec<String> {
    let mut urls = Vec::new();
    if let Some(arr) = p.datos.get("pictures").and_then(|x| x.as_array()) {
        for pic in arr {
            if let Some(u) = pic.get("secure_url").and_then(|v| v.as_str()).or_else(|| pic.get("url").and_then(|v| v.as_str())) {
                let u = u.replace("http://", "https://");
                if !u.is_empty() && !urls.contains(&u) {
                    urls.push(u);
                }
            }
        }
    }
    if urls.is_empty() {
        if let Some(t) = &p.thumbnail {
            urls.push(t.replace("http://", "https://"));
        }
    }
    urls
}

#[derive(Deserialize)]
pub struct ConectarShopify {
    pub shop_domain: String,
}

/// Inicia el OAuth: devuelve la URL de autorización de Shopify (el front redirige ahí).
pub async fn conectar(user: AuthVendedor, body: web::Json<ConectarShopify>, pool: web::Data<DbPool>, cfg: web::Data<Config>) -> ApiResult<HttpResponse> {
    user.exigir_owner()?;
    if !cfg.shopify_configurado() {
        return Err(ApiError::bad_request("Shopify no está configurado en el servidor (faltan SHOPIFY_API_KEY/SHOPIFY_API_SECRET)."));
    }
    let shop = crate::shopify::dominio(&body.into_inner().shop_domain);
    if !shop.ends_with(".myshopify.com") {
        return Err(ApiError::bad_request("Dominio inválido (debe terminar en .myshopify.com)."));
    }
    let vid = user.id;
    let estado = db::run(&pool, move |conn| {
        crate::services::saas_services::permitir_marketplace(conn, vid)?;
        crate::services::ml_services::crear_estado(conn, vid)
    })
    .await?;
    let redirect = format!("{}/api/shopify/callback", cfg.app_url);
    let mut url = reqwest::Url::parse(&format!("https://{shop}/admin/oauth/authorize")).map_err(|_| ApiError::internal("URL de Shopify inválida"))?;
    url.query_pairs_mut()
        .append_pair("client_id", &cfg.shopify_api_key)
        .append_pair("scope", &cfg.shopify_scopes)
        .append_pair("redirect_uri", &redirect)
        .append_pair("state", &estado);
    Ok(HttpResponse::Ok().json(json!({ "authorize_url": url.to_string() })))
}

#[derive(Deserialize)]
pub struct CallbackShopify {
    pub code: Option<String>,
    pub shop: Option<String>,
    pub state: Option<String>,
}

/// Callback del OAuth: valida el estado, intercambia el code por el token y guarda la tienda.
pub async fn callback(q: web::Query<CallbackShopify>, pool: web::Data<DbPool>, cfg: web::Data<Config>) -> ApiResult<HttpResponse> {
    if !cfg.shopify_configurado() {
        return Err(ApiError::bad_request("Shopify no está configurado."));
    }
    let q = q.into_inner();
    let (code, shop, estado) = match (q.code, q.shop, q.state) {
        (Some(c), Some(s), Some(e)) => (c, s, e),
        _ => return Ok(pagina(&cfg.app_url, false, "Faltan parámetros de autorización.")),
    };
    let vendedor_id = match db::run(&pool, move |conn| crate::services::ml_services::consumir_estado(conn, &estado)).await {
        Ok(v) => v,
        Err(_) => return Ok(pagina(&cfg.app_url, false, "La autorización expiró o es inválida. Inténtalo de nuevo.")),
    };
    let token = match ShopifyClient::new().exchange_code(&shop, &cfg.shopify_api_key, &cfg.shopify_api_secret, &code).await {
        Ok(t) => t,
        Err(e) => return Ok(pagina(&cfg.app_url, false, &format!("No se pudo obtener el token: {e}"))),
    };
    let shop2 = crate::shopify::dominio(&shop);
    if let Err(e) = db::run(&pool, move |conn| shopify_services::conectar(conn, vendedor_id, &shop2, &token, None)).await {
        return Ok(pagina(&cfg.app_url, false, &format!("No se pudo guardar la tienda: {e}")));
    }
    Ok(pagina(&cfg.app_url, true, "¡Tienda de Shopify conectada!"))
}

fn pagina(app_url: &str, ok: bool, msg: &str) -> HttpResponse {
    let (titulo, color) = if ok { ("✅ Listo", "#2aa76a") } else { ("⚠️ Error", "#d93025") };
    let esc = |s: &str| s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
    let html = format!(
        "<!doctype html><html lang=\"es\"><meta charset=\"utf-8\"><meta http-equiv=\"refresh\" content=\"3;url={}/shopify\"><body style=\"font-family:system-ui,sans-serif;background:#0f1420;color:#e8edf5;display:grid;place-items:center;height:100vh;margin:0\"><div style=\"text-align:center;max-width:420px;padding:20px\"><h2 style=\"color:{color}\">{titulo}</h2><p>{}</p><p style=\"color:#8a93a6\">Volviendo a MeliHub…</p></div></body></html>",
        esc(app_url), esc(msg)
    );
    HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html)
}

pub async fn listar(user: AuthVendedor, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    let vid = user.id;
    let l = db::run(&pool, move |conn| shopify_services::listar(conn, vid)).await?;
    Ok(HttpResponse::Ok().json(l))
}

pub async fn desconectar(user: AuthVendedor, path: web::Path<Uuid>, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    user.exigir_edicion()?;
    let (vid, id) = (user.id, path.into_inner());
    db::run(&pool, move |conn| shopify_services::desconectar(conn, id, vid)).await?;
    Ok(HttpResponse::Ok().json(json!({ "ok": true })))
}

/// Órdenes de una tienda Shopify (para la vista de despachos).
pub async fn ordenes(user: AuthVendedor, path: web::Path<Uuid>, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    let (vid, id) = (user.id, path.into_inner());
    let cuenta = db::run(&pool, move |conn| shopify_services::cuenta_por_id(conn, id, vid)).await?;
    let token = crate::crypto::descifrar(&cuenta.access_token);
    let v = ShopifyClient::new().get_orders(&cuenta.shop_domain, &token).await?;
    Ok(HttpResponse::Ok().json(v))
}

#[derive(Deserialize)]
pub struct PublicarShopify {
    pub cuenta_id: Uuid,
    pub producto_id: Uuid,
}

pub async fn publicar(user: AuthVendedor, body: web::Json<PublicarShopify>, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    user.exigir_edicion()?;
    let vid = user.id;
    let PublicarShopify { cuenta_id, producto_id } = body.into_inner();
    let (cuenta, p) = db::run(&pool, move |conn| {
        let cuenta = shopify_services::cuenta_por_id(conn, cuenta_id, vid)?;
        let p: Producto = productos::table
            .inner_join(ml_cuentas::table)
            .filter(ml_cuentas::vendedor_id.eq(vid))
            .filter(productos::id.eq(producto_id))
            .select(Producto::as_select())
            .first(conn)
            .map_err(|_| ApiError::not_found("Producto no encontrado"))?;
        Ok::<_, ApiError>((cuenta, p))
    })
    .await?;

    let token = crate::crypto::descifrar(&cuenta.access_token);
    let client = ShopifyClient::new();
    let precio = p.precio_jarstore.filter(|v| *v > 0).unwrap_or(p.precio);
    let desc = p.datos.get("descripcion").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let imgs = imagenes_de(&p);
    let cuerpo = build_producto(&p.titulo, &desc, precio, &p.ml_item_id, p.stock, &imgs);
    let resp = client.crear_producto(&cuenta.shop_domain, &token, &cuerpo).await?;

    let prod = resp.get("product");
    let shopify_product_id = id_str(prod.and_then(|x| x.get("id")));
    let variant = prod.and_then(|x| x.get("variants")).and_then(|a| a.as_array()).and_then(|a| a.first());
    let variant_id = id_str(variant.and_then(|v| v.get("id")));
    let inventory_item_id = id_str(variant.and_then(|v| v.get("inventory_item_id")));

    // Stock: fijar en la ubicación principal (best-effort).
    let mut stock_ok = false;
    if let Some(inv_id) = &inventory_item_id {
        if let Ok(Some(loc)) = client.ubicacion_principal(&cuenta.shop_domain, &token).await {
            stock_ok = client.set_inventario(&cuenta.shop_domain, &token, inv_id, loc, p.stock).await.is_ok();
        }
    }
    // Mapeo para el sync.
    if let Some(spid) = &shopify_product_id {
        let (spid2, vid2, iid2) = (spid.clone(), variant_id.clone(), inventory_item_id.clone());
        let _ = db::run(&pool, move |conn| shopify_services::registrar_publicacion(conn, producto_id, cuenta_id, &spid2, vid2, iid2)).await;
    }
    Ok(HttpResponse::Ok().json(json!({ "producto": resp, "stock_ok": stock_ok })))
}
