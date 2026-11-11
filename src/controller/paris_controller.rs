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

/// Elige el tipo de precio de `/v2/price-types` (prefiere "normal/lista"): devuelve (id, nombre).
fn pick_store_price(v: &Value) -> Option<(String, String)> {
    let results = v.get("results").and_then(|r| r.as_array())?;
    let chosen = results
        .iter()
        .find(|x| {
            x.get("name").and_then(|n| n.as_str()).map(|n| {
                let n = n.to_lowercase();
                n.contains("normal") || n.contains("lista") || n.contains("list")
            }).unwrap_or(false)
        })
        .or_else(|| results.first())?;
    let id = chosen.get("id").and_then(|i| i.as_str())?.to_string();
    let name = chosen.get("name").and_then(|n| n.as_str()).unwrap_or("Normal").to_string();
    Some((id, name))
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

pub async fn atributos_variante(user: AuthVendedor, path: web::Path<(Uuid, String)>, pool: web::Data<DbPool>, client: web::Data<ParisClient>) -> ApiResult<HttpResponse> {
    let (id, family_id) = path.into_inner();
    exigir_admin(&user)?;
    let cuenta = cuenta_de(&pool, user.id, id).await?;
    let key = crate::crypto::descifrar(&cuenta.api_key);
    Ok(HttpResponse::Ok().json(client.get_variant_attributes(cuenta.sandbox, &key, &family_id).await?))
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
    /// Stock (quantity de la variante base).
    #[serde(default)]
    pub stock: Option<i32>,
    #[serde(default)]
    pub atributos: Vec<AttrReq>,
    /// Atributos de la VARIANTE (color/talla…), requeridos por la familia.
    #[serde(default)]
    pub variante_atributos: Vec<AttrReq>,
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
    // 1) Crear el producto SIN precio (el precio inline en v2 crashea en Paris; se setea aparte).
    let b = push::build_product(&r.name, &r.ref_product, &r.family_code, &r.categoria_id, r.precio, r.stock.unwrap_or(0), &pares(&r.atributos), &pares(&r.variante_atributos), &r.imagenes, None, None);
    let resp = client.product_create(cuenta.sandbox, &key, &b).await?;
    // SKU que asignó Paris (para el precio y el mapeo).
    let paris_sku = resp.get("sku").and_then(|v| v.as_str()).or_else(|| resp.get("sellerSku").and_then(|v| v.as_str())).map(|s| s.to_string());
    // 2) Precio aparte (best-effort: si falla, el producto igual quedó creado).
    let mut precio_ok = false;
    if let Some(psku) = &paris_sku {
        let store_price = client.get_price_types(cuenta.sandbox, &key).await.ok().and_then(|v| pick_store_price(&v));
        let (sp_id, sp_type) = match &store_price { Some((i, n)) => (Some(i.as_str()), Some(n.as_str())), None => (None, None) };
        if sp_id.is_some() {
            let pb = serde_json::json!({ "prices": push::build_precio(r.precio, sp_id, sp_type) });
            precio_ok = client.update_price(cuenta.sandbox, &key, psku, &pb).await.is_ok();
        }
    }
    // 3) Mapeo para el sync continuo.
    if let Some(pid) = r.producto_id {
        let (cid, sku, psku) = (cuenta.id, r.ref_product.clone(), paris_sku.clone());
        let _ = db::run(&pool, move |conn| crate::services::paris_services::registrar_publicacion(conn, pid, cid, &sku, psku)).await;
    }
    Ok(HttpResponse::Ok().json(serde_json::json!({ "producto": resp, "precio_ok": precio_ok })))
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

pub async fn variante(user: AuthVendedor, path: web::Path<Uuid>, body: web::Json<Value>, pool: web::Data<DbPool>, client: web::Data<ParisClient>) -> ApiResult<HttpResponse> {
    user.exigir_edicion()?;
    exigir_admin(&user)?;
    let cuenta = cuenta_de(&pool, user.id, path.into_inner()).await?;
    let key = crate::crypto::descifrar(&cuenta.api_key);
    Ok(HttpResponse::Ok().json(client.variant_create(cuenta.sandbox, &key, &body.into_inner()).await?))
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
    let store_price = client.get_price_types(cuenta.sandbox, &key).await.ok().and_then(|v| pick_store_price(&v));
    let (sp_id, sp_type) = match &store_price { Some((i, n)) => (Some(i.as_str()), Some(n.as_str())), None => (None, None) };
    let b = serde_json::json!({ "prices": push::build_precio(r.precio, sp_id, sp_type) });
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
