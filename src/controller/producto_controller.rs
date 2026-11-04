use crate::auth::AuthVendedor;
use crate::db::{self, DbPool};
use crate::errors::ApiResult;
use crate::services::inventario_services::{self as inv, ConfigUpdate, InventarioUpdate};
use crate::services::producto_services::ProductoFiltro;
use actix_web::{web, HttpResponse};
use uuid::Uuid;

/// Catálogo unificado con ganancia/margen calculados por canal.
pub async fn listar(
    user: AuthVendedor,
    filtro: web::Query<ProductoFiltro>,
    pool: web::Data<DbPool>,
) -> ApiResult<HttpResponse> {
    let (vid, admin) = (user.id, user.es_admin());
    let f = filtro.into_inner();
    let items = db::run(&pool, move |conn| inv::listar(conn, vid, admin, f)).await?;
    Ok(HttpResponse::Ok().json(items))
}

/// Fija el costo y/o el precio JarStore de un producto.
pub async fn set_inventario(
    path: web::Path<Uuid>,
    user: AuthVendedor,
    body: web::Json<InventarioUpdate>,
    pool: web::Data<DbPool>,
) -> ApiResult<HttpResponse> {
    let (pid, vid, admin) = (path.into_inner(), user.id, user.es_admin());
    let b = body.into_inner();
    let p = db::run(&pool, move |conn| inv::set_inventario(conn, pid, vid, admin, b)).await?;
    Ok(HttpResponse::Ok().json(p))
}

pub async fn get_config(user: AuthVendedor, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    let vid = user.id;
    let c = db::run(&pool, move |conn| inv::config(conn, vid)).await?;
    Ok(HttpResponse::Ok().json(c))
}

pub async fn set_config(
    user: AuthVendedor,
    body: web::Json<ConfigUpdate>,
    pool: web::Data<DbPool>,
) -> ApiResult<HttpResponse> {
    let vid = user.id;
    let b = body.into_inner();
    let c = db::run(&pool, move |conn| inv::set_config(conn, vid, b)).await?;
    Ok(HttpResponse::Ok().json(c))
}
