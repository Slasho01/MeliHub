use crate::config::Config;
use crate::db::{self, DbPool};
use crate::errors::{ApiError, ApiResult};
use crate::ml::MlClient;
use crate::models::ApiKey;
use crate::services::{api_key_services, ml_services, pub_services};
use actix_web::{web, HttpRequest, HttpResponse};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

/// Resuelve la API key del header `X-API-Key`.
async fn key_de(req: &HttpRequest, pool: &web::Data<DbPool>) -> ApiResult<ApiKey> {
    let raw = req
        .headers()
        .get("X-API-Key")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| ApiError::unauthorized("Falta el header X-API-Key"))?;
    db::run(pool, move |conn| api_key_services::autenticar(conn, &raw)).await
}

/// GET /api/pub/productos — catálogo del vendedor (precio del canal de la key).
pub async fn productos(req: HttpRequest, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    let key = key_de(&req, &pool).await?;
    let items = db::run(&pool, move |conn| pub_services::productos(conn, &key)).await?;
    Ok(HttpResponse::Ok().json(items))
}

#[derive(Deserialize)]
pub struct VentaRequest {
    pub producto_id: Uuid,
    pub cantidad: i32,
}

/// POST /api/pub/ventas — descuenta stock por una venta y lo empuja a Mercado Libre.
pub async fn reportar_venta(
    req: HttpRequest,
    body: web::Json<VentaRequest>,
    pool: web::Data<DbPool>,
    client: web::Data<MlClient>,
    cfg: web::Data<Config>,
) -> ApiResult<HttpResponse> {
    let key = key_de(&req, &pool).await?;
    let VentaRequest { producto_id, cantidad } = body.into_inner();
    let kvid = key.vendedor_id;
    let venta = db::run(&pool, move |conn| pub_services::reportar_venta(conn, &key, producto_id, cantidad)).await?;

    // Empujar el nuevo stock a Mercado Libre (best-effort: la venta ya ocurrió en la plataforma).
    let mut ml_actualizado = false;
    if cfg.ml.configurado() {
        let cuenta_id = venta.ml_cuenta_id;
        match db::run(&pool, move |conn| ml_services::get_cuenta(conn, cuenta_id, kvid, true)).await {
            Ok(cuenta) => match crate::sync::access_token_valido(&pool, &client, &cuenta).await {
                Ok(token) => match client.update_stock(&token, &venta.ml_item_id, venta.nuevo_stock).await {
                    Ok(()) => ml_actualizado = true,
                    Err(e) => log::warn!("No se pudo actualizar stock en ML ({}): {e}", venta.ml_item_id),
                },
                Err(e) => log::warn!("Token ML no disponible para {}: {e}", venta.ml_item_id),
            },
            Err(e) => log::warn!("Cuenta ML no encontrada para la venta: {e}"),
        }
    }
    Ok(HttpResponse::Ok().json(json!({
        "producto_id": producto_id,
        "nuevo_stock": venta.nuevo_stock,
        "ml_actualizado": ml_actualizado,
    })))
}
