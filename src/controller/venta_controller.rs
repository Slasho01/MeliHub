use crate::auth::AuthVendedor;
use crate::config::Config;
use crate::db::{self, DbPool};
use crate::errors::{ApiError, ApiResult};
use crate::ml::MlClient;
use crate::services::{ml_services, venta_services as venta};
use actix_web::{web, HttpResponse};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

/// Trae las órdenes recientes de todas las cuentas ML del vendedor.
pub async fn sincronizar(user: AuthVendedor, pool: web::Data<DbPool>, client: web::Data<MlClient>, cfg: web::Data<Config>) -> ApiResult<HttpResponse> {
    if !cfg.ml.configurado() {
        return Err(ApiError::bad_request("Mercado Libre no está configurado."));
    }
    let (vid, admin) = (user.id, user.es_admin());
    let cuentas = db::run(&pool, move |conn| ml_services::listar_cuentas(conn, vid, admin)).await?;
    let (mut total, mut errores) = (0usize, 0usize);
    for cuenta in &cuentas {
        match crate::sync::sincronizar_ordenes(&pool, &client, cuenta).await {
            Ok(n) => total += n,
            Err(e) => {
                errores += 1;
                log::warn!("No se pudieron sincronizar órdenes de la cuenta {}: {e}", cuenta.id);
            }
        }
    }
    Ok(HttpResponse::Ok().json(json!({ "sincronizadas": total, "cuentas": cuentas.len(), "errores": errores })))
}

#[derive(Deserialize)]
pub struct DashQuery {
    pub dias: Option<i64>,
}

/// Dashboard de ventas con ganancia real (últimos N días, 30 por defecto).
pub async fn dashboard(user: AuthVendedor, q: web::Query<DashQuery>, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    let (vid, dias) = (user.id, q.into_inner().dias.unwrap_or(30));
    Ok(HttpResponse::Ok().json(db::run(&pool, move |conn| venta::dashboard(conn, vid, dias)).await?))
}

#[derive(Deserialize)]
pub struct ListQuery {
    pub cuenta_id: Option<Uuid>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Listado de órdenes del vendedor (más recientes primero).
pub async fn listar(user: AuthVendedor, q: web::Query<ListQuery>, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    let vid = user.id;
    let ListQuery { cuenta_id, limit, offset } = q.into_inner();
    let items = db::run(&pool, move |conn| venta::listar(conn, vid, cuenta_id, limit.unwrap_or(50), offset.unwrap_or(0))).await?;
    Ok(HttpResponse::Ok().json(items))
}
