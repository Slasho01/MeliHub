//! Endpoints del Marketplace de Cencosud (Paris). Requiere plan con marketplaces (Pro+),
//! que ya valida `paris_services::conectar` al conectar la cuenta.

use crate::auth::AuthVendedor;
use crate::db::{self, DbPool};
use crate::errors::{ApiError, ApiResult};
use crate::models::ParisCuenta;
use crate::paris::ParisClient;
use crate::services::paris_push_services as push;
use crate::services::paris_services as svc;
use actix_web::{web, HttpResponse};
use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct ConectarReq {
    pub api_key: String,
    #[serde(default)]
    pub nombre: Option<String>,
    /// Por defecto conecta al entorno de pruebas (staging).
    #[serde(default = "verdadero")]
    pub sandbox: bool,
}
fn verdadero() -> bool {
    true
}

/// Por ahora Paris está disponible solo para el administrador de la plataforma.
fn exigir_admin(user: &AuthVendedor) -> ApiResult<()> {
    if !user.es_admin() {
        return Err(ApiError::forbidden("Por ahora Paris está disponible solo para el administrador."));
    }
    Ok(())
}

/// Obtiene la cuenta validando que sea del vendedor autenticado.
async fn cuenta_de(pool: &web::Data<DbPool>, org: Uuid, id: Uuid) -> ApiResult<ParisCuenta> {
    db::run(pool, move |conn| {
        let c = svc::cuenta_por_id(conn, id)?.ok_or_else(|| ApiError::not_found("Cuenta de Paris"))?;
        if c.vendedor_id != org {
            return Err(ApiError::forbidden("Esa cuenta de Paris no es tuya"));
        }
        Ok(c)
    })
    .await
}

pub async fn conectar(user: AuthVendedor, body: web::Json<ConectarReq>, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    user.exigir_owner()?;
    exigir_admin(&user)?;
    let ConectarReq { api_key, nombre, sandbox } = body.into_inner();
    let org = user.id;
    let cuenta = db::run(&pool, move |conn| svc::conectar(conn, org, &api_key, nombre, sandbox)).await?;
    Ok(HttpResponse::Created().json(cuenta))
}

pub async fn listar(user: AuthVendedor, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    exigir_admin(&user)?;
    let org = user.id;
    let cuentas = db::run(&pool, move |conn| svc::listar(conn, org)).await?;
    Ok(HttpResponse::Ok().json(cuentas))
}

pub async fn desconectar(user: AuthVendedor, path: web::Path<Uuid>, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    user.exigir_owner()?;
    exigir_admin(&user)?;
    let (org, id) = (user.id, path.into_inner());
    db::run(&pool, move |conn| svc::desconectar(conn, id, org)).await?;
    Ok(HttpResponse::Ok().json(serde_json::json!({ "ok": true })))
}

// ----------------- Taxonomía -----------------

pub async fn familias(user: AuthVendedor, path: web::Path<Uuid>, pool: web::Data<DbPool>, client: web::Data<ParisClient>) -> ApiResult<HttpResponse> {
    exigir_admin(&user)?;
    let cuenta = cuenta_de(&pool, user.id, path.into_inner()).await?;
    let key = crate::crypto::descifrar(&cuenta.api_key);
    Ok(HttpResponse::Ok().json(client.get_families(cuenta.sandbox, &key).await?))
}

pub async fn categorias(user: AuthVendedor, path: web::Path<(Uuid, String)>, pool: web::Data<DbPool>, client: web::Data<ParisClient>) -> ApiResult<HttpResponse> {
    let (id, family) = path.into_inner();
    exigir_admin(&user)?;
    let cuenta = cuenta_de(&pool, user.id, id).await?;
    let key = crate::crypto::descifrar(&cuenta.api_key);
    Ok(HttpResponse::Ok().json(client.get_categories(cuenta.sandbox, &key, &family).await?))
}

pub async fn atributos(user: AuthVendedor, path: web::Path<(Uuid, String)>, pool: web::Data<DbPool>, client: web::Data<ParisClient>) -> ApiResult<HttpResponse> {
    let (id, family_id) = path.into_inner();
    exigir_admin(&user)?;
    let cuenta = cuenta_de(&pool, user.id, id).await?;
    let key = crate::crypto::descifrar(&cuenta.api_key);
    Ok(HttpResponse::Ok().json(client.get_attributes(cuenta.sandbox, &key, &family_id).await?))
}

// ----------------- Productos / variantes -----------------

#[derive(Deserialize)]
pub struct AttrReq {
    pub code: String,
    pub value: String,
}
fn pares(a: &[AttrReq]) -> Vec<(String, String)> {
    a.iter().map(|x| (x.code.clone(), x.value.clone())).collect()
}

#[derive(Deserialize)]
pub struct PublicarReq {
    pub name: String,
    pub ref_product: String,
    pub family_code: String,
    pub precio: i64,
    /// Categoría (subcategoría) de la familia — obligatoria en Paris.
    pub categoria_id: String,
    #[serde(default)]
    pub oferta: Option<i64>,
    #[serde(default)]
    pub atributos: Vec<AttrReq>,
    /// URLs de imágenes (Paris exige al menos una).
    #[serde(default)]
    pub imagenes: Vec<String>,
    /// Producto del hub (opcional): si viene, se guarda el mapeo para el sync continuo.
    #[serde(default)]
    pub producto_id: Option<Uuid>,
}

/// Crea un producto en Paris armando el cuerpo con el builder.
pub async fn publicar(user: AuthVendedor, path: web::Path<Uuid>, body: web::Json<PublicarReq>, pool: web::Data<DbPool>, client: web::Data<ParisClient>) -> ApiResult<HttpResponse> {
    user.exigir_edicion()?;
    exigir_admin(&user)?;
    let cuenta = cuenta_de(&pool, user.id, path.into_inner()).await?;
    let key = crate::crypto::descifrar(&cuenta.api_key);
    let r = body.into_inner();
    let b = push::build_product(&r.name, &r.ref_product, &r.family_code, &r.categoria_id, r.precio, r.oferta, &pares(&r.atributos), &r.imagenes);
    let resp = client.product_create(cuenta.sandbox, &key, &b).await?;
    // Guarda el mapeo (producto hub ↔ cuenta/sku) para que el job sincronice el stock.
    if let Some(pid) = r.producto_id {
        let paris_sku = resp
            .get("sku")
            .and_then(|v| v.as_str())
            .or_else(|| resp.pointer("/data/sku").and_then(|v| v.as_str()))
            .map(|s| s.to_string());
        let (cid, sku) = (cuenta.id, r.ref_product.clone());
        let _ = db::run(&pool, move |conn| crate::services::paris_services::registrar_publicacion(conn, pid, cid, &sku, paris_sku)).await;
    }
    Ok(HttpResponse::Ok().json(resp))
}

/// Actualización avanzada (cuerpo JSON crudo, para campos puntuales).
pub async fn actualizar_producto(user: AuthVendedor, path: web::Path<(Uuid, String)>, body: web::Json<Value>, pool: web::Data<DbPool>, client: web::Data<ParisClient>) -> ApiResult<HttpResponse> {
    user.exigir_edicion()?;
    let (id, sku) = path.into_inner();
    exigir_admin(&user)?;
    let cuenta = cuenta_de(&pool, user.id, id).await?;
    let key = crate::crypto::descifrar(&cuenta.api_key);
    Ok(HttpResponse::Ok().json(client.product_update(cuenta.sandbox, &key, &sku, &body.into_inner()).await?))
}

#[derive(Deserialize)]
pub struct VarianteReq {
    pub name: String,
    pub product_sku: String,
    pub ref_variant: String,
    #[serde(default)]
    pub atributos: Vec<AttrReq>,
}

pub async fn variante(user: AuthVendedor, path: web::Path<Uuid>, body: web::Json<VarianteReq>, pool: web::Data<DbPool>, client: web::Data<ParisClient>) -> ApiResult<HttpResponse> {
    user.exigir_edicion()?;
    exigir_admin(&user)?;
    let cuenta = cuenta_de(&pool, user.id, path.into_inner()).await?;
    let key = crate::crypto::descifrar(&cuenta.api_key);
    let r = body.into_inner();
    let b = push::build_variante(&r.name, &r.product_sku, &r.ref_variant, &pares(&r.atributos));
    Ok(HttpResponse::Ok().json(client.variant_create(cuenta.sandbox, &key, &b).await?))
}

// ----------------- Precio / stock -----------------

#[derive(Deserialize)]
pub struct PrecioReq {
    pub precio: i64,
    #[serde(default)]
    pub oferta: Option<i64>,
}

pub async fn precio(user: AuthVendedor, path: web::Path<(Uuid, String)>, body: web::Json<PrecioReq>, pool: web::Data<DbPool>, client: web::Data<ParisClient>) -> ApiResult<HttpResponse> {
    user.exigir_edicion()?;
    let (id, sku) = path.into_inner();
    exigir_admin(&user)?;
    let cuenta = cuenta_de(&pool, user.id, id).await?;
    let key = crate::crypto::descifrar(&cuenta.api_key);
    let r = body.into_inner();
    let b = push::build_precio(r.precio, r.oferta);
    Ok(HttpResponse::Ok().json(client.update_price(cuenta.sandbox, &key, &sku, &b).await?))
}

#[derive(Deserialize)]
pub struct StockItem {
    pub sku_seller: String,
    pub cantidad: i32,
}
#[derive(Deserialize)]
pub struct StockReq {
    pub items: Vec<StockItem>,
}

pub async fn stock(user: AuthVendedor, path: web::Path<Uuid>, body: web::Json<StockReq>, pool: web::Data<DbPool>, client: web::Data<ParisClient>) -> ApiResult<HttpResponse> {
    user.exigir_edicion()?;
    exigir_admin(&user)?;
    let cuenta = cuenta_de(&pool, user.id, path.into_inner()).await?;
    let key = crate::crypto::descifrar(&cuenta.api_key);
    let items: Vec<(String, i32)> = body.into_inner().items.into_iter().map(|i| (i.sku_seller, i.cantidad)).collect();
    let b = push::build_stock(&items);
    Ok(HttpResponse::Ok().json(client.update_stock_sku_seller(cuenta.sandbox, &key, &b).await?))
}

// ----------------- Órdenes -----------------

#[derive(Deserialize)]
pub struct OrdenesQuery {
    #[serde(default)]
    pub query: Option<String>,
}

pub async fn ordenes(user: AuthVendedor, path: web::Path<Uuid>, q: web::Query<OrdenesQuery>, pool: web::Data<DbPool>, client: web::Data<ParisClient>) -> ApiResult<HttpResponse> {
    exigir_admin(&user)?;
    let cuenta = cuenta_de(&pool, user.id, path.into_inner()).await?;
    let key = crate::crypto::descifrar(&cuenta.api_key);
    let query = q.into_inner().query.unwrap_or_default();
    Ok(HttpResponse::Ok().json(client.get_orders(cuenta.sandbox, &key, &query).await?))
}

pub async fn orden(user: AuthVendedor, path: web::Path<(Uuid, String)>, pool: web::Data<DbPool>, client: web::Data<ParisClient>) -> ApiResult<HttpResponse> {
    let (id, numero) = path.into_inner();
    exigir_admin(&user)?;
    let cuenta = cuenta_de(&pool, user.id, id).await?;
    let key = crate::crypto::descifrar(&cuenta.api_key);
    Ok(HttpResponse::Ok().json(client.get_order(cuenta.sandbox, &key, &numero).await?))
}

pub async fn sub_ordenes(user: AuthVendedor, path: web::Path<Uuid>, q: web::Query<OrdenesQuery>, pool: web::Data<DbPool>, client: web::Data<ParisClient>) -> ApiResult<HttpResponse> {
    exigir_admin(&user)?;
    let cuenta = cuenta_de(&pool, user.id, path.into_inner()).await?;
    let key = crate::crypto::descifrar(&cuenta.api_key);
    let query = q.into_inner().query.unwrap_or_default();
    Ok(HttpResponse::Ok().json(client.get_sub_orders(cuenta.sandbox, &key, &query).await?))
}
