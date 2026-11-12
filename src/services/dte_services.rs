//! Cuentas de facturación electrónica (DTE / SII) por vendedor. Multi-tenant: cada seller
//! conecta su proveedor (LibreDTE) con su token y RUT para emitir boletas/facturas.

use crate::errors::{ApiError, ApiResult};
use crate::models::{DteCuenta, NewDteCuenta};
use crate::schema::dte_cuentas;
use diesel::prelude::*;
use uuid::Uuid;

pub fn conectar(conn: &mut PgConnection, vendedor_id: Uuid, api_token: &str, rut_emisor: &str, razon_social: Option<String>, certificacion: bool) -> ApiResult<DteCuenta> {
    let api_token = api_token.trim();
    let rut = rut_emisor.trim();
    if api_token.is_empty() {
        return Err(ApiError::bad_request("El token de LibreDTE es obligatorio"));
    }
    if rut.is_empty() {
        return Err(ApiError::bad_request("El RUT del emisor es obligatorio"));
    }
    let nuevo = NewDteCuenta {
        vendedor_id,
        proveedor: "libredte".to_string(),
        api_token: crate::crypto::cifrar(api_token),
        rut_emisor: rut.to_string(),
        razon_social: razon_social.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()),
        certificacion,
    };
    diesel::insert_into(dte_cuentas::table)
        .values(&nuevo)
        .returning(DteCuenta::as_returning())
        .get_result(conn)
        .map_err(ApiError::from)
}

pub fn listar(conn: &mut PgConnection, vendedor_id: Uuid) -> ApiResult<Vec<DteCuenta>> {
    dte_cuentas::table
        .filter(dte_cuentas::vendedor_id.eq(vendedor_id))
        .select(DteCuenta::as_select())
        .order(dte_cuentas::created_at.desc())
        .load(conn)
        .map_err(ApiError::from)
}

pub fn cuenta_por_id(conn: &mut PgConnection, id: Uuid, vendedor_id: Uuid) -> ApiResult<DteCuenta> {
    dte_cuentas::table
        .find(id)
        .filter(dte_cuentas::vendedor_id.eq(vendedor_id))
        .select(DteCuenta::as_select())
        .first(conn)
        .map_err(|_| ApiError::not_found("Cuenta de facturación no encontrada"))
}

pub fn desconectar(conn: &mut PgConnection, id: Uuid, vendedor_id: Uuid) -> ApiResult<()> {
    let n = diesel::delete(dte_cuentas::table.find(id).filter(dte_cuentas::vendedor_id.eq(vendedor_id))).execute(conn)?;
    if n == 0 {
        return Err(ApiError::not_found("Cuenta de facturación no encontrada"));
    }
    Ok(())
}
