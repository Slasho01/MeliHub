//! Facturación electrónica (DTE / SII): conexión de la cuenta del proveedor (LibreDTE) por
//! vendedor. La emisión de boletas/facturas se agrega encima de esta conexión.

use crate::auth::AuthVendedor;
use crate::db::{self, DbPool};
use crate::dte::{construir_documento, DteClient, LineaDte, Receptor};
use crate::errors::{ApiError, ApiResult};
use crate::services::dte_services;
use actix_web::{web, HttpResponse};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct ConectarDte {
    pub api_token: String,
    pub rut_emisor: String,
    pub razon_social: Option<String>,
    #[serde(default)]
    pub certificacion: Option<bool>,
}

pub async fn conectar(user: AuthVendedor, body: web::Json<ConectarDte>, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    user.exigir_owner()?;
    let vid = user.id;
    let b = body.into_inner();
    let cert = b.certificacion.unwrap_or(true);
    let c = db::run(&pool, move |conn| dte_services::conectar(conn, vid, &b.api_token, &b.rut_emisor, b.razon_social, cert)).await?;
    Ok(HttpResponse::Ok().json(c))
}

pub async fn listar(user: AuthVendedor, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    let vid = user.id;
    let l = db::run(&pool, move |conn| dte_services::listar(conn, vid)).await?;
    Ok(HttpResponse::Ok().json(l))
}

pub async fn desconectar(user: AuthVendedor, path: web::Path<Uuid>, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    user.exigir_owner()?;
    let (vid, id) = (user.id, path.into_inner());
    db::run(&pool, move |conn| dte_services::desconectar(conn, id, vid)).await?;
    Ok(HttpResponse::Ok().json(json!({ "ok": true })))
}

#[derive(Deserialize)]
pub struct EmitirReq {
    pub cuenta_id: Uuid,
    /// 39 = boleta electrónica, 33 = factura electrónica.
    pub tipo: i32,
    #[serde(default)]
    pub receptor: Receptor,
    pub detalle: Vec<LineaDte>,
    #[serde(default)]
    pub descuento_pct: f64,
}

/// Emite una boleta/factura en el SII vía LibreDTE, a partir de los datos del cotizador/venta.
pub async fn emitir(user: AuthVendedor, body: web::Json<EmitirReq>, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    user.exigir_edicion()?;
    let r = body.into_inner();
    if r.detalle.is_empty() {
        return Err(ApiError::bad_request("El documento no tiene líneas."));
    }
    if r.tipo != 39 && r.tipo != 33 {
        return Err(ApiError::bad_request("Tipo de documento no soportado (39 boleta o 33 factura)."));
    }
    let (vid, cuenta_id) = (user.id, r.cuenta_id);
    let cuenta = db::run(&pool, move |conn| dte_services::cuenta_por_id(conn, cuenta_id, vid)).await?;
    let token = crate::crypto::descifrar(&cuenta.api_token);
    let fecha = crate::services::common::now().format("%Y-%m-%d").to_string();
    let documento = construir_documento(r.tipo, &fecha, &cuenta.rut_emisor, &r.receptor, &r.detalle, r.descuento_pct);
    let client = DteClient::new();
    let resp = client.emitir(&token, &documento, r.tipo, &cuenta.rut_emisor, &r.receptor.rut).await?;
    let folio = resp.get("folio").and_then(|v| v.as_i64());
    let pdf = folio.map(|f| client.pdf_url(&cuenta.rut_emisor, r.tipo, f));
    Ok(HttpResponse::Ok().json(json!({ "ok": true, "folio": folio, "pdf": pdf, "resultado": resp })))
}
