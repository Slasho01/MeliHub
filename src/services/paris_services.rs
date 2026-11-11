//! Persistencia de cuentas del Marketplace de Cencosud (Paris).

use crate::errors::{ApiError, ApiResult};
use crate::models::{NewParisCuenta, ParisCuenta};
use crate::schema::paris_cuentas;
use diesel::prelude::*;
use uuid::Uuid;

/// Conecta (o actualiza) una cuenta de Paris. Gated: solo planes con marketplaces (Pro+).
pub fn conectar(conn: &mut PgConnection, vendedor_id: Uuid, api_key: &str, nombre: Option<String>, sandbox: bool) -> ApiResult<ParisCuenta> {
    crate::services::saas_services::permitir_marketplace(conn, vendedor_id)?;
    let api_key = api_key.trim();
    if api_key.is_empty() {
        return Err(ApiError::bad_request("La API key de Paris es obligatoria"));
    }
    let nuevo = NewParisCuenta {
        vendedor_id,
        nombre,
        api_key: crate::crypto::cifrar(api_key),
        sandbox,
    };
    diesel::insert_into(paris_cuentas::table)
        .values(&nuevo)
        .returning(ParisCuenta::as_returning())
        .get_result(conn)
        .map_err(ApiError::from)
}

pub fn listar(conn: &mut PgConnection, vendedor_id: Uuid) -> ApiResult<Vec<ParisCuenta>> {
    paris_cuentas::table
        .filter(paris_cuentas::vendedor_id.eq(vendedor_id))
        .select(ParisCuenta::as_select())
        .order(paris_cuentas::created_at.desc())
        .load(conn)
        .map_err(ApiError::from)
}

pub fn cuenta_por_id(conn: &mut PgConnection, id: Uuid) -> ApiResult<Option<ParisCuenta>> {
    paris_cuentas::table.find(id).select(ParisCuenta::as_select()).first(conn).optional().map_err(ApiError::from)
}

/// Todas las cuentas (para los jobs de sincronización/órdenes).
pub fn todas_cuentas(conn: &mut PgConnection) -> ApiResult<Vec<ParisCuenta>> {
    paris_cuentas::table
        .filter(paris_cuentas::activa.eq(true))
        .select(ParisCuenta::as_select())
        .load(conn)
        .map_err(ApiError::from)
}

pub fn desconectar(conn: &mut PgConnection, cuenta_id: Uuid, vendedor_id: Uuid) -> ApiResult<()> {
    let n = diesel::delete(
        paris_cuentas::table
            .filter(paris_cuentas::id.eq(cuenta_id))
            .filter(paris_cuentas::vendedor_id.eq(vendedor_id)),
    )
    .execute(conn)?;
    if n == 0 {
        return Err(ApiError::not_found("Cuenta de Paris"));
    }
    Ok(())
}
