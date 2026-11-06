use crate::auth::AuthVendedor;
use crate::db::{self, DbPool};
use crate::errors::ApiResult;
use crate::services::auditoria_services as svc;
use actix_web::{web, HttpResponse};

/// Registro de actividad de la organización (solo dueño).
pub async fn listar(user: AuthVendedor, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    user.exigir_owner()?;
    let org = user.id;
    let items = db::run(&pool, move |conn| svc::listar(conn, org, 200)).await?;
    Ok(HttpResponse::Ok().json(items))
}
