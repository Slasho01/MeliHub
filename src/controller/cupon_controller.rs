use crate::auth::AuthVendedor;
use crate::db::{self, DbPool};
use crate::errors::{ApiError, ApiResult};
use crate::services::cupon_services as svc;
use actix_web::{web, HttpResponse};
use chrono::{NaiveDate, NaiveDateTime};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

fn exigir_admin(user: &AuthVendedor) -> ApiResult<()> {
    if user.es_admin() {
        Ok(())
    } else {
        Err(ApiError::forbidden("Solo el administrador de la plataforma puede gestionar cupones"))
    }
}

fn parse_fecha(s: &str) -> ApiResult<NaiveDateTime> {
    NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d")
        .ok()
        .and_then(|d| d.and_hms_opt(23, 59, 59))
        .ok_or_else(|| ApiError::bad_request("Fecha inválida (usa AAAA-MM-DD)"))
}

#[derive(Deserialize)]
pub struct ValidarBody {
    pub codigo: String,
}

/// Valida un cupón y devuelve su info, para previsualizar el descuento. Requiere login.
pub async fn validar(_user: AuthVendedor, body: web::Json<ValidarBody>, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    let codigo = body.into_inner().codigo;
    let d = db::run(&pool, move |conn| svc::info(conn, &codigo)).await?;
    Ok(HttpResponse::Ok().json(d))
}

// ----------------------------- Súper-admin -----------------------------

pub async fn listar(user: AuthVendedor, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    exigir_admin(&user)?;
    let cs = db::run(&pool, svc::listar).await?;
    Ok(HttpResponse::Ok().json(cs))
}

#[derive(Deserialize)]
pub struct CrearBody {
    pub codigo: String,
    pub tipo: String,
    pub valor: i64,
    #[serde(default)]
    pub vence: Option<String>,
    #[serde(default)]
    pub usos_max: Option<i32>,
}

pub async fn crear(user: AuthVendedor, body: web::Json<CrearBody>, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    exigir_admin(&user)?;
    let CrearBody { codigo, tipo, valor, vence, usos_max } = body.into_inner();
    let vence_dt = match vence.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(s) => Some(parse_fecha(s)?),
        None => None,
    };
    let c = db::run(&pool, move |conn| svc::crear(conn, &codigo, &tipo, valor, vence_dt, usos_max)).await?;
    Ok(HttpResponse::Ok().json(c))
}

#[derive(Deserialize)]
pub struct ActivoBody {
    pub activo: bool,
}

pub async fn set_activo(user: AuthVendedor, path: web::Path<Uuid>, body: web::Json<ActivoBody>, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    exigir_admin(&user)?;
    let (id, activo) = (path.into_inner(), body.into_inner().activo);
    db::run(&pool, move |conn| svc::set_activo(conn, id, activo)).await?;
    Ok(HttpResponse::Ok().json(json!({ "ok": true })))
}

pub async fn eliminar(user: AuthVendedor, path: web::Path<Uuid>, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    exigir_admin(&user)?;
    let id = path.into_inner();
    db::run(&pool, move |conn| svc::eliminar(conn, id)).await?;
    Ok(HttpResponse::Ok().json(json!({ "ok": true })))
}
