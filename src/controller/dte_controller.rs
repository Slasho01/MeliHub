//! Facturación electrónica (DTE / SII): conexión de la cuenta del proveedor (LibreDTE) por
//! vendedor. La emisión de boletas/facturas se agrega encima de esta conexión.

use crate::auth::AuthVendedor;
use crate::db::{self, DbPool};
use crate::errors::ApiResult;
use crate::services::dte_services;
use actix_web::{web, HttpResponse};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct ConectarDte {
    pub api_token: String,
    pub rut_emisor: String,
    pub razon_social: Option<String>,
    #[serde(default)]
    pub certificacion: Option<bool>,
}

pub async fn conectar(user: AuthVendedor, body: web::Json<ConectarDte>, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    user.exigir_owner()?;
    let vid = user.id;
    let b = body.into_inner();
    let cert = b.certificacion.unwrap_or(true);
    let c = db::run(&pool, move |conn| dte_services::conectar(conn, vid, &b.api_token, &b.rut_emisor, b.razon_social, cert)).await?;
    Ok(HttpResponse::Ok().json(c))
}

pub async fn listar(user: AuthVendedor, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    let vid = user.id;
    let l = db::run(&pool, move |conn| dte_services::listar(conn, vid)).await?;
    Ok(HttpResponse::Ok().json(l))
}

pub async fn desconectar(user: AuthVendedor, path: web::Path<Uuid>, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    user.exigir_owner()?;
    let (vid, id) = (user.id, path.into_inner());
    db::run(&pool, move |conn| dte_services::desconectar(conn, id, vid)).await?;
    Ok(HttpResponse::Ok().json(json!({ "ok": true })))
}
