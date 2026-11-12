//! Shopify como canal: conectar la tienda (verifica el token), listar/desconectar, y publicar
//! productos del hub en la tienda.

use crate::auth::AuthVendedor;
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
    pub access_token: String,
    #[serde(default)]
    pub nombre: Option<String>,
}

pub async fn conectar(user: AuthVendedor, body: web::Json<ConectarShopify>, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    user.exigir_edicion()?;
    let b = body.into_inner();
    // Verifica la conexión (token + dominio) antes de guardar.
    ShopifyClient::new()
        .verificar(&b.shop_domain, &b.access_token)
        .await
        .map_err(|e| ApiError::bad_request(format!("No se pudo conectar a Shopify (revisa dominio y token): {e}")))?;
    let (vid, dom, tok, nom) = (user.id, b.shop_domain, b.access_token, b.nombre);
    let cuenta = db::run(&pool, move |conn| shopify_services::conectar(conn, vid, &dom, &tok, nom)).await?;
    Ok(HttpResponse::Ok().json(cuenta))
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
