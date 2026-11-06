use crate::auth::AuthVendedor;
use crate::db::{self, DbPool};
use crate::errors::ApiResult;
use crate::services::api_key_services::{self as svc, NuevaKey};
use actix_web::{web, HttpResponse};
use serde_json::json;
use uuid::Uuid;

/// Crea una API key. Devuelve la clave en claro UNA sola vez.
pub async fn crear(user: AuthVendedor, body: web::Json<NuevaKey>, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    user.exigir_owner()?;
    let vid = user.id;
    let b = body.into_inner();
    let (registro, clave) = db::run(&pool, move |conn| svc::generar(conn, vid, b)).await?;
    let (actor, det) = (user.actor_id, format!("{} ({})", registro.nombre, registro.canal));
    let _ = db::run(&pool, move |conn| crate::services::auditoria_services::log(conn, vid, actor, "apikey_crear".into(), det)).await;
    Ok(HttpResponse::Created().json(json!({ "api_key": clave, "registro": registro })))
}

pub async fn listar(user: AuthVendedor, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    let vid = user.id;
    Ok(HttpResponse::Ok().json(db::run(&pool, move |conn| svc::listar(conn, vid)).await?))
}

pub async fn revocar(path: web::Path<Uuid>, user: AuthVendedor, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    user.exigir_owner()?;
    let (id, vid, admin) = (path.into_inner(), user.id, user.es_admin());
    db::run(&pool, move |conn| svc::revocar(conn, id, vid, admin)).await?;
    let actor = user.actor_id;
    let _ = db::run(&pool, move |conn| crate::services::auditoria_services::log(conn, vid, actor, "apikey_revocar".into(), id.to_string())).await;
    Ok(HttpResponse::NoContent().finish())
}
