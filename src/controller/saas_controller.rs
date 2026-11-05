use crate::auth::AuthVendedor;
use crate::db::{self, DbPool};
use crate::errors::{ApiError, ApiResult};
use crate::services::saas_services as saas;
use actix_web::{web, HttpResponse};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

/// Plan actual del vendedor, uso y catálogo de planes.
pub async fn mi_plan(user: AuthVendedor, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    let vid = user.id;
    Ok(HttpResponse::Ok().json(db::run(&pool, move |conn| saas::mi_plan(conn, vid)).await?))
}

#[derive(Deserialize)]
pub struct NotifQuery {
    pub solo_no_leidas: Option<bool>,
}

/// Notificaciones del vendedor + conteo de no leídas.
pub async fn notificaciones(user: AuthVendedor, q: web::Query<NotifQuery>, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    let vid = user.id;
    let solo = q.into_inner().solo_no_leidas.unwrap_or(false);
    let (items, no_leidas) = db::run(&pool, move |conn| {
        let lista = saas::listar(conn, vid, solo, 100)?;
        let nl = saas::no_leidas(conn, vid)?;
        Ok::<_, ApiError>((lista, nl))
    })
    .await?;
    Ok(HttpResponse::Ok().json(json!({ "items": items, "no_leidas": no_leidas })))
}

#[derive(Deserialize)]
pub struct MarcarBody {
    /// ids a marcar; si viene vacío/omitido, marca todas.
    pub ids: Option<Vec<Uuid>>,
}

pub async fn marcar_leidas(user: AuthVendedor, body: web::Json<MarcarBody>, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    let vid = user.id;
    let ids = body.into_inner().ids.filter(|v| !v.is_empty());
    let n = db::run(&pool, move |conn| saas::marcar_leidas(conn, vid, ids)).await?;
    Ok(HttpResponse::Ok().json(json!({ "marcadas": n })))
}
