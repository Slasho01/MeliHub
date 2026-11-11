//! Cuentas de Falabella Seller Center por vendedor. La `api_key` se guarda **cifrada**
//! (`crypto::cifrar`) y se descifra sólo al momento de firmar una llamada.

use crate::errors::{ApiError, ApiResult};
use crate::models::{FalabellaCuenta, NewFalabellaCuenta, NewFalabellaPublicacion};
use crate::schema::{falabella_cuentas, falabella_publicaciones, productos};
use crate::services::common::now;
use diesel::prelude::*;
use uuid::Uuid;

/// Registra (o actualiza) que un producto quedó publicado en una cuenta de Falabella,
/// para la sincronización continua de precio/stock.
pub fn registrar_publicacion(conn: &mut PgConnection, producto_id: Uuid, cuenta_id: Uuid, seller_sku: &str, operador: &str) -> ApiResult<()> {
    diesel::insert_into(falabella_publicaciones::table)
        .values(NewFalabellaPublicacion {
            producto_id,
            cuenta_id,
            seller_sku: seller_sku.to_string(),
            operador: operador.to_string(),
        })
        .on_conflict((falabella_publicaciones::producto_id, falabella_publicaciones::cuenta_id))
        .do_update()
        .set(falabella_publicaciones::seller_sku.eq(seller_sku))
        .execute(conn)?;
    Ok(())
}

/// Para una cuenta Falabella, los productos publicados con su precio/stock ACTUAL del hub
/// (para empujar `ProductUpdate`). Devuelve (seller_sku, precio, stock, operador).
pub fn publicaciones_para_sync(conn: &mut PgConnection, cuenta_id: Uuid) -> ApiResult<Vec<(String, i64, i32, String)>> {
    falabella_publicaciones::table
        .inner_join(productos::table)
        .filter(falabella_publicaciones::cuenta_id.eq(cuenta_id))
        .select((falabella_publicaciones::seller_sku, productos::precio, productos::stock, falabella_publicaciones::operador))
        .load::<(String, i64, i32, String)>(conn)
        .map_err(ApiError::from)
}

/// Todas las cuentas Falabella (para el job de sincronización).
pub fn todas_cuentas(conn: &mut PgConnection) -> ApiResult<Vec<FalabellaCuenta>> {
    falabella_cuentas::table
        .filter(falabella_cuentas::activa.eq(true))
        .select(FalabellaCuenta::as_select())
        .load(conn)
        .map_err(ApiError::from)
}

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
