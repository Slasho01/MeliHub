use crate::auth::AuthVendedor;
use crate::db::{self, DbPool};
use crate::errors::ApiResult;
use crate::services::producto_services::{self as svc, ProductoFiltro};
use actix_web::{web, HttpResponse};

/// Catálogo unificado (productos sincronizados desde todas las cuentas del vendedor).
pub async fn listar(
    user: AuthVendedor,
    filtro: web::Query<ProductoFiltro>,
    pool: web::Data<DbPool>,
) -> ApiResult<HttpResponse> {
    let (vid, admin) = (user.id, user.es_admin());
    let f = filtro.into_inner();
    let items = db::run(&pool, move |conn| svc::listar(conn, vid, admin, f)).await?;
    Ok(HttpResponse::Ok().json(items))
}
