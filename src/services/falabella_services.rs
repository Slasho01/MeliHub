//! Cuentas de Falabella Seller Center por vendedor. La `api_key` se guarda **cifrada**
//! (`crypto::cifrar`) y se descifra sólo al momento de firmar una llamada.

use crate::errors::{ApiError, ApiResult};
use crate::models::{FalabellaCuenta, NewFalabellaCuenta};
use crate::schema::falabella_cuentas;
use crate::services::common::now;
use diesel::prelude::*;
use uuid::Uuid;

/// Conecta (o actualiza) la cuenta Falabella del vendedor. Upsert por (vendedor_id, user_id).
pub fn conectar(conn: &mut PgConnection, vendedor_id: Uuid, user_id: &str, api_key: &str, nickname: Option<String>) -> ApiResult<FalabellaCuenta> {
    let nuevo = NewFalabellaCuenta {
        vendedor_id,
        user_id: user_id.trim().to_string(),
        api_key: crate::crypto::cifrar(api_key.trim()),
        nickname,
    };
    diesel::insert_into(falabella_cuentas::table)
        .values(&nuevo)
        .on_conflict((falabella_cuentas::vendedor_id, falabella_cuentas::user_id))
        .do_update()
        .set((
            falabella_cuentas::api_key.eq(&nuevo.api_key),
            falabella_cuentas::nickname.eq(&nuevo.nickname),
            falabella_cuentas::activa.eq(true),
            falabella_cuentas::updated_at.eq(now()),
        ))
        .returning(FalabellaCuenta::as_returning())
        .get_result(conn)
        .map_err(ApiError::from)
}

pub fn listar(conn: &mut PgConnection, vendedor_id: Uuid) -> ApiResult<Vec<FalabellaCuenta>> {
    falabella_cuentas::table
        .filter(falabella_cuentas::vendedor_id.eq(vendedor_id))
        .select(FalabellaCuenta::as_select())
        .order(falabella_cuentas::created_at.desc())
        .load(conn)
        .map_err(ApiError::from)
}

/// Cuenta con la `api_key` TODAVÍA cifrada (el que llame descifra para firmar).
pub fn get(conn: &mut PgConnection, cuenta_id: Uuid, vendedor_id: Uuid) -> ApiResult<FalabellaCuenta> {
    falabella_cuentas::table
        .filter(falabella_cuentas::id.eq(cuenta_id))
        .filter(falabella_cuentas::vendedor_id.eq(vendedor_id))
        .select(FalabellaCuenta::as_select())
        .first(conn)
        .optional()?
        .ok_or_else(|| ApiError::not_found("Cuenta de Falabella"))
}

pub fn desconectar(conn: &mut PgConnection, cuenta_id: Uuid, vendedor_id: Uuid) -> ApiResult<()> {
    let n = diesel::delete(
        falabella_cuentas::table
            .filter(falabella_cuentas::id.eq(cuenta_id))
            .filter(falabella_cuentas::vendedor_id.eq(vendedor_id)),
    )
    .execute(conn)?;
    if n == 0 {
        return Err(ApiError::not_found("Cuenta de Falabella"));
    }
    Ok(())
}
