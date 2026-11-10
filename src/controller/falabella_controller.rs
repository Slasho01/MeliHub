//! Conexión de cuentas de Falabella Seller Center. Al conectar se validan las credenciales
//! con una llamada de solo lectura (GetBrands) ANTES de guardarlas (cifradas).

use crate::auth::AuthVendedor;
use crate::db::{self, DbPool};
use crate::errors::{ApiError, ApiResult};
use crate::falabella::FalabellaClient;
use crate::services::falabella_services as svc;
use actix_web::{web, HttpResponse};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct ConectarReq {
    pub user_id: String,
    pub api_key: String,
    #[serde(default)]
    pub nickname: Option<String>,
}

pub async fn conectar(
    user: AuthVendedor,
    body: web::Json<ConectarReq>,
    pool: web::Data<DbPool>,
    client: web::Data<FalabellaClient>,
) -> ApiResult<HttpResponse> {
    user.exigir_owner()?;
    let ConectarReq { user_id, api_key, nickname } = body.into_inner();
    let user_id = user_id.trim().to_string();
    let api_key = api_key.trim().to_string();
    if user_id.is_empty() || api_key.is_empty() {
        return Err(ApiError::bad_request("Ingresa el UserID (email) y la API key de Falabella Seller Center."));
    }
    // Valida credenciales contra Falabella (lectura) antes de guardar nada.
    client.get_brands(&user_id, &api_key).await?;
    let vid = user.id;
    let nick = nickname.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    let cuenta = db::run(&pool, move |conn| svc::conectar(conn, vid, &user_id, &api_key, nick)).await?;
    Ok(HttpResponse::Created().json(cuenta))
}

pub async fn listar(user: AuthVendedor, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    let vid = user.id;
    Ok(HttpResponse::Ok().json(db::run(&pool, move |conn| svc::listar(conn, vid)).await?))
}

pub async fn desconectar(path: web::Path<Uuid>, user: AuthVendedor, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    user.exigir_owner()?;
    let (cid, vid) = (path.into_inner(), user.id);
    db::run(&pool, move |conn| svc::desconectar(conn, cid, vid)).await?;
    Ok(HttpResponse::NoContent().finish())
}

#[derive(Deserialize)]
pub struct CuentaQuery {
    pub cuenta: Uuid,
}

/// Árbol de categorías de Falabella (para mapear la categoría del producto).
pub async fn categorias(user: AuthVendedor, q: web::Query<CuentaQuery>, pool: web::Data<DbPool>, client: web::Data<FalabellaClient>) -> ApiResult<HttpResponse> {
    let (vid, cid) = (user.id, q.cuenta);
    let cuenta = db::run(&pool, move |conn| svc::get(conn, cid, vid)).await?;
    let api_key = crate::crypto::descifrar(&cuenta.api_key);
    Ok(HttpResponse::Ok().json(client.get_category_tree(&cuenta.user_id, &api_key).await?))
}

#[derive(Deserialize)]
pub struct AtributosQuery {
    pub cuenta: Uuid,
    pub categoria: String,
}

/// Atributos (obligatorios/opcionales) de una categoría Falabella.
pub async fn atributos(user: AuthVendedor, q: web::Query<AtributosQuery>, pool: web::Data<DbPool>, client: web::Data<FalabellaClient>) -> ApiResult<HttpResponse> {
    let (vid, cid, cat) = (user.id, q.cuenta, q.categoria.clone());
    let cuenta = db::run(&pool, move |conn| svc::get(conn, cid, vid)).await?;
    let api_key = crate::crypto::descifrar(&cuenta.api_key);
    Ok(HttpResponse::Ok().json(client.get_category_attributes(&cuenta.user_id, &api_key, &cat).await?))
}

#[derive(Deserialize)]
pub struct AtributoKV {
    pub nombre: String,
    pub valor: String,
}

#[derive(Deserialize)]
pub struct PublicarReq {
    pub producto_id: Uuid,
    pub cuenta_id: Uuid,
    pub categoria: String,
    pub marca: String,
    #[serde(default)]
    pub variation: Option<String>,
    #[serde(default)]
    pub operador: Option<String>,
    #[serde(default)]
    pub atributos: Vec<AtributoKV>,
}

/// Publica (o actualiza) un producto del hub en Falabella con su categoría/atributos.
pub async fn publicar(user: AuthVendedor, body: web::Json<PublicarReq>, pool: web::Data<DbPool>, client: web::Data<FalabellaClient>) -> ApiResult<HttpResponse> {
    user.exigir_edicion()?;
    let PublicarReq { producto_id, cuenta_id, categoria, marca, variation, operador, atributos } = body.into_inner();
    let (vid, admin) = (user.id, user.es_admin());
    let (producto, cuenta) = db::run(&pool, move |conn| {
        let (p, _) = crate::services::producto_services::get_con_cuenta(conn, producto_id, vid, admin)?;
        let c = svc::get(conn, cuenta_id, vid)?;
        Ok::<_, ApiError>((p, c))
    })
    .await?;
    let api_key = crate::crypto::descifrar(&cuenta.api_key);
    let atributos: Vec<(String, String)> = atributos.into_iter().map(|a| (a.nombre, a.valor)).collect();
    let xml = crate::services::falabella_push_services::build_product_xml(
        &producto,
        &categoria,
        &marca,
        variation.as_deref().unwrap_or(""),
        operador.as_deref().unwrap_or(""),
        &atributos,
    );
    Ok(HttpResponse::Ok().json(client.product_create(&cuenta.user_id, &api_key, xml).await?))
}
