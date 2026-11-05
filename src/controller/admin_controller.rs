use crate::auth::AuthVendedor;
use crate::db::{self, DbPool};
use crate::errors::{ApiError, ApiResult};
use crate::services::admin_services as adm;
use actix_web::{web, HttpResponse};
use serde::Deserialize;
use uuid::Uuid;

fn exigir_admin(user: &AuthVendedor) -> ApiResult<()> {
    if !user.es_admin() {
        return Err(ApiError::forbidden("Solo el administrador de la plataforma puede acceder aquí"));
    }
    Ok(())
}

/// Lista de vendedores (clientes) con su uso.
pub async fn vendedores(user: AuthVendedor, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    exigir_admin(&user)?;
    Ok(HttpResponse::Ok().json(db::run(&pool, adm::listar_vendedores).await?))
}

/// Métricas globales de la plataforma.
pub async fn metricas(user: AuthVendedor, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    exigir_admin(&user)?;
    Ok(HttpResponse::Ok().json(db::run(&pool, adm::metricas).await?))
}

#[derive(Deserialize)]
pub struct PlanBody {
    pub plan: String,
}

/// Cambia el plan de un vendedor.
pub async fn set_plan(user: AuthVendedor, path: web::Path<Uuid>, body: web::Json<PlanBody>, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    exigir_admin(&user)?;
    let (id, plan) = (path.into_inner(), body.into_inner().plan);
    Ok(HttpResponse::Ok().json(db::run(&pool, move |conn| adm::set_plan(conn, id, &plan)).await?))
}

#[derive(Deserialize)]
pub struct EstadoBody {
    pub activo: bool,
}

/// Suspende o reactiva a un vendedor.
pub async fn set_estado(user: AuthVendedor, path: web::Path<Uuid>, body: web::Json<EstadoBody>, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    exigir_admin(&user)?;
    let (id, activo) = (path.into_inner(), body.into_inner().activo);
    if id == user.id && !activo {
        return Err(ApiError::bad_request("No puedes suspender tu propia cuenta"));
    }
    Ok(HttpResponse::Ok().json(db::run(&pool, move |conn| adm::set_activo(conn, id, activo)).await?))
}
