use crate::auth::AuthVendedor;
use crate::db::{self, DbPool};
use crate::errors::ApiResult;
use crate::services::facturacion_services::{self as svc, FacturacionInput};
use actix_web::{web, HttpResponse};
use serde_json::json;

/// Datos de facturación de la organización (solo dueño).
pub async fn obtener(user: AuthVendedor, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    user.exigir_owner()?;
    let org = user.id;
    let f = db::run(&pool, move |conn| svc::get(conn, org)).await?;
    Ok(HttpResponse::Ok().json(json!({ "facturacion": f })))
}

/// Crea o actualiza los datos de facturación (solo dueño).
pub async fn guardar(user: AuthVendedor, body: web::Json<FacturacionInput>, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    user.exigir_owner()?;
    let org = user.id;
    let f = db::run(&pool, move |conn| svc::upsert(conn, org, body.into_inner())).await?;
    Ok(HttpResponse::Ok().json(f))
}
