//! Kits / combos: definir productos compuestos y ver su disponibilidad.

use crate::auth::AuthVendedor;
use crate::db::{self, DbPool};
use crate::errors::ApiResult;
use crate::ml::MlClient;
use crate::services::kit_services;
use actix_web::{web, HttpResponse};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct ComponenteReq {
    pub producto_id: Uuid,
    pub cantidad: i32,
}

#[derive(Deserialize)]
pub struct DefinirReq {
    pub kit_producto_id: Uuid,
    pub componentes: Vec<ComponenteReq>,
}

/// Lista los kits del vendedor con componentes y disponibilidad calculada.
pub async fn listar(user: AuthVendedor, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    let vid = user.id;
    let kits = db::run(&pool, move |conn| kit_services::kits_de(conn, vid)).await?;
    Ok(HttpResponse::Ok().json(json!({ "kits": kits })))
}

/// Define (o redefine) los componentes de un kit y empuja el stock resultante a ML.
pub async fn definir(
    user: AuthVendedor,
    body: web::Json<DefinirReq>,
    pool: web::Data<DbPool>,
    client: web::Data<MlClient>,
) -> ApiResult<HttpResponse> {
    user.exigir_edicion()?;
    let DefinirReq { kit_producto_id, componentes } = body.into_inner();
    let comps: Vec<(Uuid, i32)> = componentes.into_iter().map(|c| (c.producto_id, c.cantidad)).collect();
    let vid = user.id;
    let venta = db::run(&pool, move |conn| kit_services::definir_kit(conn, vid, kit_producto_id, comps)).await?;
    if let Some(v) = venta {
        crate::sync::empujar_stock_ml(&pool, &client, &v).await;
    }
    Ok(HttpResponse::Ok().json(json!({ "ok": true })))
}

/// Deshace un kit (quita sus componentes).
pub async fn eliminar(user: AuthVendedor, path: web::Path<Uuid>, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    user.exigir_edicion()?;
    let (vid, kid) = (user.id, path.into_inner());
    db::run(&pool, move |conn| kit_services::eliminar_kit(conn, vid, kid)).await?;
    Ok(HttpResponse::Ok().json(json!({ "ok": true })))
}
