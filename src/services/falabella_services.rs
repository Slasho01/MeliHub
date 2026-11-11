//! Cuentas de Falabella Seller Center por vendedor. La `api_key` se guarda **cifrada**
//! (`crypto::cifrar`) y se descifra sólo al momento de firmar una llamada.

use crate::errors::{ApiError, ApiResult};
use crate::models::{FalabellaCuenta, NewFalabellaCuenta, NewFalabellaPublicacion};
use crate::schema::{falabella_cuentas, falabella_publicaciones, falabella_ventas, productos};
use crate::services::common::now;
use diesel::prelude::*;
use uuid::Uuid;

/// Registra (o actualiza) que un producto quedó publicado en una cuenta de Falabella,
/// para la sincronización continua de precio/stock.
pub fn registrar_publicacion(conn: &mut PgConnection, producto_id: Uuid, cuenta_id: Uuid, seller_sku: &str, operador: &str, categoria: &str, marca: &str, atributos: serde_json::Value) -> ApiResult<()> {
    let cat = (!categoria.trim().is_empty()).then(|| categoria.trim().to_string());
    let mar = (!marca.trim().is_empty()).then(|| marca.trim().to_string());
    diesel::insert_into(falabella_publicaciones::table)
        .values(NewFalabellaPublicacion {
            producto_id,
            cuenta_id,
            seller_sku: seller_sku.to_string(),
            operador: operador.to_string(),
            categoria: cat.clone(),
            marca: mar.clone(),
            atributos: atributos.clone(),
        })
        .on_conflict((falabella_publicaciones::producto_id, falabella_publicaciones::cuenta_id))
        .do_update()
        .set((
            falabella_publicaciones::seller_sku.eq(seller_sku),
            falabella_publicaciones::categoria.eq(cat),
            falabella_publicaciones::marca.eq(mar),
            falabella_publicaciones::atributos.eq(atributos),
        ))
        .execute(conn)?;
    Ok(())
}

/// Mapeo guardado (categoría, marca, operador, atributos) — para republicar sin re-elegir.
pub fn mapeo_de(conn: &mut PgConnection, producto_id: Uuid, cuenta_id: Uuid) -> ApiResult<Option<(Option<String>, Option<String>, String, serde_json::Value)>> {
    falabella_publicaciones::table
        .filter(falabella_publicaciones::producto_id.eq(producto_id))
        .filter(falabella_publicaciones::cuenta_id.eq(cuenta_id))
        .select((falabella_publicaciones::categoria, falabella_publicaciones::marca, falabella_publicaciones::operador, falabella_publicaciones::atributos))
        .first(conn)
        .optional()
        .map_err(ApiError::from)
}

/// Para una cuenta Falabella, los productos publicados con su precio/stock ACTUAL del hub
/// (para empujar `ProductUpdate`). Devuelve (seller_sku, precio, stock, operador).
pub fn publicaciones_para_sync(conn: &mut PgConnection, cuenta_id: Uuid) -> ApiResult<Vec<(String, i64, i32, String)>> {
    let rows = falabella_publicaciones::table
        .inner_join(productos::table)
        .filter(falabella_publicaciones::cuenta_id.eq(cuenta_id))
        .select((falabella_publicaciones::seller_sku, productos::precio, productos::precio_falabella, productos::stock, falabella_publicaciones::operador))
        .load::<(String, i64, Option<i64>, i32, String)>(conn)?;
    // Usa el precio propio de Falabella si está; si no, el de ML.
    Ok(rows.into_iter().map(|(sku, precio, pf, stock, op)| (sku, pf.filter(|&x| x > 0).unwrap_or(precio), stock, op)).collect())
}

/// Marca una orden de Falabella como procesada. Devuelve `true` si era NUEVA (hay que
/// descontar stock) y `false` si ya estaba (evita descontar dos veces).
pub fn marcar_venta_si_nueva(conn: &mut PgConnection, order_id: &str, cuenta_id: Uuid) -> ApiResult<bool> {
    let n = diesel::insert_into(falabella_ventas::table)
        .values((falabella_ventas::order_id.eq(order_id), falabella_ventas::cuenta_id.eq(cuenta_id)))
        .on_conflict_do_nothing()
        .execute(conn)?;
    Ok(n > 0)
}

/// Operador (facl/fape/faco) con que se publicó un producto en una cuenta (default facl).
pub fn operador_de(conn: &mut PgConnection, producto_id: Uuid, cuenta_id: Uuid) -> String {
    falabella_publicaciones::table
        .filter(falabella_publicaciones::producto_id.eq(producto_id))
        .filter(falabella_publicaciones::cuenta_id.eq(cuenta_id))
        .select(falabella_publicaciones::operador)
        .first::<String>(conn)
        .optional()
        .ok()
        .flatten()
        .unwrap_or_else(|| "facl".to_string())
}

/// Producto del hub que corresponde a un SellerSku publicado en una cuenta Falabella.
pub fn producto_de_sku(conn: &mut PgConnection, cuenta_id: Uuid, seller_sku: &str) -> ApiResult<Option<Uuid>> {
    falabella_publicaciones::table
        .filter(falabella_publicaciones::cuenta_id.eq(cuenta_id))
        .filter(falabella_publicaciones::seller_sku.eq(seller_sku))
        .select(falabella_publicaciones::producto_id)
        .first(conn)
        .optional()
        .map_err(ApiError::from)
}

/// Publicaciones locales de una cuenta: (producto_id, seller_sku, título) — para la vista
/// de estado (se cruza con GetProducts para el QC).
pub fn listar_publicaciones(conn: &mut PgConnection, cuenta_id: Uuid) -> ApiResult<Vec<(Uuid, String, String)>> {
    falabella_publicaciones::table
        .inner_join(productos::table)
        .filter(falabella_publicaciones::cuenta_id.eq(cuenta_id))
        .select((falabella_publicaciones::producto_id, falabella_publicaciones::seller_sku, productos::titulo))
        .load::<(Uuid, String, String)>(conn)
        .map_err(ApiError::from)
}

/// Cuenta Falabella por id, sin filtrar por vendedor (uso interno: webhook/jobs).
pub fn cuenta_por_id(conn: &mut PgConnection, id: Uuid) -> ApiResult<Option<FalabellaCuenta>> {
    falabella_cuentas::table.find(id).select(FalabellaCuenta::as_select()).first(conn).optional().map_err(ApiError::from)
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
    // Falabella (marketplace) solo en Pro/Business. El Free debe mejorar su plan.
    crate::services::saas_services::permitir_marketplace(conn, vendedor_id)?;
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
