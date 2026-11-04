use crate::auth::AuthVendedor;
use crate::db::{self, DbPool};
use crate::errors::ApiResult;
use crate::services::api_key_services::{self as svc, NuevaKey};
use actix_web::{web, HttpResponse};
use serde_json::json;
use uuid::Uuid;

/// Crea una API key. Devuelve la clave en claro UNA sola vez.
pub async fn crear(user: AuthVendedor, body: web::Json<NuevaKey>, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    let vid = user.id;
    let b = body.into_inner();
    let (registro, clave) = db::run(&pool, move |conn| svc::generar(conn, vid, b)).await?;
    Ok(HttpResponse::Created().json(json!({ "api_key": clave, "registro": registro })))
}

pub async fn listar(user: AuthVendedor, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    let vid = user.id;
    Ok(HttpResponse::Ok().json(db::run(&pool, move |conn| svc::listar(conn, vid)).await?))
}

pub async fn revocar(path: web::Path<Uuid>, user: AuthVendedor, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    let (id, vid, admin) = (path.into_inner(), user.id, user.es_admin());
    db::run(&pool, move |conn| svc::revocar(conn, id, vid, admin)).await?;
    Ok(HttpResponse::NoContent().finish())
}
