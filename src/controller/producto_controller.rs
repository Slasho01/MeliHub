use crate::auth::AuthVendedor;
use crate::config::Config;
use crate::db::{self, DbPool};
use crate::errors::{ApiError, ApiResult};
use crate::ml::MlClient;
use crate::services::inventario_services::{self as inv, ConfigUpdate, InventarioUpdate};
use crate::services::producto_services::{self as prod, ProductoFiltro};
use actix_web::{web, HttpResponse};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

fn exigir_ml(cfg: &Config) -> ApiResult<()> {
    if !cfg.ml.configurado() {
        return Err(ApiError::bad_request("Mercado Libre no está configurado."));
    }
    Ok(())
}

#[derive(Deserialize)]
pub struct EstadoBody {
    /// "active" (reactivar) | "paused" (pausar)
    pub estado: String,
}

/// Pausa o reactiva una publicación en Mercado Libre (y lo refleja en MeliHub).
pub async fn set_estado_ml(
    user: AuthVendedor,
    path: web::Path<Uuid>,
    body: web::Json<EstadoBody>,
    pool: web::Data<DbPool>,
    client: web::Data<MlClient>,
    cfg: web::Data<Config>,
) -> ApiResult<HttpResponse> {
    user.exigir_edicion()?;
    exigir_ml(&cfg)?;
    let estado = body.into_inner().estado;
    if estado != "active" && estado != "paused" {
        return Err(ApiError::bad_request("Estado inválido (usa active o paused)"));
    }
    let (pid, vid, admin) = (path.into_inner(), user.id, user.es_admin());
    let (producto, cuenta) = db::run(&pool, move |conn| prod::get_con_cuenta(conn, pid, vid, admin)).await?;
    let token = crate::sync::access_token_valido(&pool, &client, &cuenta).await?;
    client.set_status(&token, &producto.ml_item_id, &estado).await?;
    let (org, actor, det) = (user.id, user.actor_id, format!("{} → {}", producto.ml_item_id, estado));
    let p = db::run(&pool, move |conn| prod::aplicar_estado_local(conn, pid, &estado)).await?;
    let _ = db::run(&pool, move |conn| crate::services::auditoria_services::log(conn, org, actor, "estado_ml".into(), det)).await;
    Ok(HttpResponse::Ok().json(p))
}

#[derive(Deserialize)]
pub struct PrecioBody {
    pub precio: i64,
}

/// Empuja un nuevo precio a Mercado Libre (y lo refleja en MeliHub).
pub async fn set_precio_ml(
    user: AuthVendedor,
    path: web::Path<Uuid>,
    body: web::Json<PrecioBody>,
    pool: web::Data<DbPool>,
    client: web::Data<MlClient>,
    cfg: web::Data<Config>,
) -> ApiResult<HttpResponse> {
    user.exigir_edicion()?;
    exigir_ml(&cfg)?;
    let precio = body.into_inner().precio;
    if precio <= 0 {
        return Err(ApiError::bad_request("El precio debe ser mayor que 0"));
    }
    let (pid, vid, admin) = (path.into_inner(), user.id, user.es_admin());
    let (producto, cuenta) = db::run(&pool, move |conn| prod::get_con_cuenta(conn, pid, vid, admin)).await?;
    let token = crate::sync::access_token_valido(&pool, &client, &cuenta).await?;
    client.update_price(&token, &producto.ml_item_id, precio).await?;
    let p = db::run(&pool, move |conn| prod::aplicar_precio_local(conn, pid, precio)).await?;
    let (org, actor, det) = (user.id, user.actor_id, format!("{} → ${}", producto.ml_item_id, precio));
    let _ = db::run(&pool, move |conn| crate::services::auditoria_services::log(conn, org, actor, "precio_ml".into(), det)).await;
    Ok(HttpResponse::Ok().json(p))
}

#[derive(Deserialize)]
pub struct InvMasivo {
    pub ids: Vec<Uuid>,
    pub costo: Option<i64>,
    pub precio_jarstore: Option<i64>,
}

/// Historial de cambios (precio/stock/estado) de un producto.
pub async fn historial(user: AuthVendedor, path: web::Path<Uuid>, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    let (pid, vid, admin) = (path.into_inner(), user.id, user.es_admin());
    let items = db::run(&pool, move |conn| {
        prod::get_con_cuenta(conn, pid, vid, admin)?; // verifica propiedad
        prod::listar_historial(conn, pid, 100)
    })
    .await?;
    Ok(HttpResponse::Ok().json(items))
}

#[derive(Deserialize)]
pub struct AlertaQuery {
    pub umbral: Option<i32>,
}

/// Productos activos con stock (depósito) ≤ umbral (5 por defecto). Para alertas de quiebre.
pub async fn alertas(user: AuthVendedor, q: web::Query<AlertaQuery>, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    let (vid, admin) = (user.id, user.es_admin());
    let umbral = q.into_inner().umbral.unwrap_or(5).max(0);
    let items = db::run(&pool, move |conn| prod::stock_bajo(conn, vid, admin, umbral)).await?;
    Ok(HttpResponse::Ok().json(items))
}

/// Edición masiva de costo y/o precio JarStore.
pub async fn inventario_masivo(user: AuthVendedor, body: web::Json<InvMasivo>, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    user.exigir_edicion()?;
    let (vid, admin) = (user.id, user.es_admin());
    let InvMasivo { ids, costo, precio_jarstore } = body.into_inner();
    let n = db::run(&pool, move |conn| prod::set_inventario_masivo(conn, vid, admin, &ids, costo, precio_jarstore)).await?;
    Ok(HttpResponse::Ok().json(json!({ "afectados": n })))
}

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
    user.exigir_edicion()?;
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
    user.exigir_owner()?;
    let vid = user.id;
    let b = body.into_inner();
    let c = db::run(&pool, move |conn| inv::set_config(conn, vid, b)).await?;
    Ok(HttpResponse::Ok().json(c))
}
