use crate::auth::{crear_token, AuthVendedor};
use crate::config::Config;
use crate::db::{self, DbPool};
use crate::errors::ApiResult;
use crate::services::auth_services::{self as svc, LoginRequest, RegistroRequest};
use actix_web::{web, HttpResponse};
use serde_json::json;

pub async fn registrar(
    body: web::Json<RegistroRequest>,
    pool: web::Data<DbPool>,
    cfg: web::Data<Config>,
) -> ApiResult<HttpResponse> {
    let v = db::run(&pool, move |conn| svc::registrar(conn, body.into_inner())).await?;
    let token = crear_token(v.id, &v.rol, &cfg.jwt_secret)?;
    Ok(HttpResponse::Created().json(json!({ "token": token, "vendedor": v })))
}

pub async fn login(
    body: web::Json<LoginRequest>,
    pool: web::Data<DbPool>,
    cfg: web::Data<Config>,
) -> ApiResult<HttpResponse> {
    let v = db::run(&pool, move |conn| svc::login(conn, body.into_inner())).await?;
    let token = crear_token(v.id, &v.rol, &cfg.jwt_secret)?;
    Ok(HttpResponse::Ok().json(json!({ "token": token, "vendedor": v })))
}

pub async fn me(user: AuthVendedor, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    let v = db::run(&pool, move |conn| svc::get(conn, user.id)).await?;
    Ok(HttpResponse::Ok().json(v))
}
