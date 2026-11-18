use crate::auth::AuthVendedor;
use crate::db::{self, DbPool};
use crate::errors::{ApiError, ApiResult};
use crate::services::admin_services as adm;
use actix_web::{web, HttpResponse};
use serde::Deserialize;
use serde_json::json;
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

// ----------------------------- Suscripciones + boletas -----------------------------

/// Lista todas las suscripciones con sus boletas (solo admin).
pub async fn suscripciones(user: AuthVendedor, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    exigir_admin(&user)?;
    Ok(HttpResponse::Ok().json(db::run(&pool, adm::listar_suscripciones).await?))
}

#[derive(Deserialize)]
pub struct BoletaBody {
    pub periodo: String,
    pub nombre_archivo: String,
    pub archivo: String, // data:...;base64,...
}

/// Sube una boleta/factura para un vendedor (solo admin).
pub async fn subir_boleta(user: AuthVendedor, path: web::Path<Uuid>, body: web::Json<BoletaBody>, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    exigir_admin(&user)?;
    let vid = path.into_inner();
    let BoletaBody { periodo, nombre_archivo, archivo } = body.into_inner();
    if archivo.len() > 7_500_000 {
        return Err(ApiError::bad_request("El archivo es muy grande (máx. ~5 MB)."));
    }
    let id = db::run(&pool, move |conn| adm::guardar_boleta(conn, vid, &periodo, &nombre_archivo, &archivo)).await?;
    Ok(HttpResponse::Ok().json(json!({ "id": id })))
}

/// Descarga una boleta: devuelve nombre + data-URL para que el front la baje.
pub async fn descargar_boleta(user: AuthVendedor, path: web::Path<Uuid>, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    exigir_admin(&user)?;
    let id = path.into_inner();
    let (nombre_archivo, archivo) = db::run(&pool, move |conn| adm::boleta_archivo(conn, id)).await?;
    Ok(HttpResponse::Ok().json(json!({ "nombre_archivo": nombre_archivo, "archivo": archivo })))
}

/// Borra una boleta (solo admin).
pub async fn eliminar_boleta(user: AuthVendedor, path: web::Path<Uuid>, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    exigir_admin(&user)?;
    let id = path.into_inner();
    db::run(&pool, move |conn| adm::eliminar_boleta(conn, id)).await?;
    Ok(HttpResponse::Ok().json(json!({ "ok": true })))
}
