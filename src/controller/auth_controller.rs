use crate::auth::{crear_token, AuthVendedor};
use crate::config::Config;
use crate::db::{self, DbPool};
use crate::errors::ApiResult;
use crate::services::auth_services::{self as svc, LoginRequest, PasswordUpdate, PerfilUpdate, RegistroRequest};
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
    req: actix_web::HttpRequest,
    body: web::Json<LoginRequest>,
    pool: web::Data<DbPool>,
    cfg: web::Data<Config>,
    limiter: web::Data<crate::auth::LoginLimiter>,
) -> ApiResult<HttpResponse> {
    // IP real (considera X-Forwarded-For detrás de ngrok/Render).
    let ip = req.connection_info().realip_remote_addr().unwrap_or("desconocida").to_string();
    if !limiter.permitido(&ip) {
        return Ok(HttpResponse::TooManyRequests()
            .json(json!({ "error": "Demasiados intentos. Espera unos minutos e intenta de nuevo." })));
    }
    match db::run(&pool, move |conn| svc::login(conn, body.into_inner())).await {
        Ok(v) => {
            limiter.exito(&ip);
            let token = crear_token(v.id, &v.rol, &cfg.jwt_secret)?;
            Ok(HttpResponse::Ok().json(json!({ "token": token, "vendedor": v })))
        }
        Err(e) => {
            limiter.fallo(&ip);
            Err(e)
        }
    }
}

pub async fn me(user: AuthVendedor, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    let v = db::run(&pool, move |conn| svc::get(conn, user.id)).await?;
    Ok(HttpResponse::Ok().json(v))
}

/// Actualiza el perfil (nombre/email) del vendedor autenticado.
pub async fn actualizar_perfil(user: AuthVendedor, body: web::Json<PerfilUpdate>, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    let vid = user.id;
    let v = db::run(&pool, move |conn| svc::actualizar_perfil(conn, vid, body.into_inner())).await?;
    Ok(HttpResponse::Ok().json(v))
}

/// Cambia la contraseña del vendedor autenticado.
pub async fn cambiar_password(user: AuthVendedor, body: web::Json<PasswordUpdate>, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    let vid = user.id;
    db::run(&pool, move |conn| svc::cambiar_password(conn, vid, body.into_inner())).await?;
    Ok(HttpResponse::Ok().json(json!({ "ok": true })))
}
