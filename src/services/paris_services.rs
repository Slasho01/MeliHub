//! Persistencia de cuentas del Marketplace de Cencosud (Paris).

use crate::errors::{ApiError, ApiResult};
use crate::models::{NewParisCuenta, NewParisPublicacion, NewParisVenta, ParisCuenta};
use crate::schema::{paris_cuentas, paris_publicaciones, paris_ventas, productos};
use diesel::prelude::*;
use std::collections::HashMap;
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

// ----------------- Mapeo de publicaciones y ventas -----------------

/// Un producto publicado, con su stock y precio actuales, para empujar a Paris.
pub struct SyncItem {
    pub sku_seller: String,
    pub stock: i32,
    pub precio: i64,
    pub paris_sku: Option<String>,
}

/// Registra (upsert por cuenta+sku) el producto publicado en Paris.
pub fn registrar_publicacion(conn: &mut PgConnection, producto_id: Uuid, cuenta_id: Uuid, sku_seller: &str, paris_sku: Option<String>) -> ApiResult<()> {
    let nuevo = NewParisPublicacion { producto_id, cuenta_id, sku_seller: sku_seller.to_string(), paris_sku };
    diesel::insert_into(paris_publicaciones::table)
        .values(&nuevo)
        .on_conflict((paris_publicaciones::cuenta_id, paris_publicaciones::sku_seller))
        .do_update()
        .set(paris_publicaciones::producto_id.eq(producto_id))
        .execute(conn)?;
    Ok(())
}

/// Productos publicados de una cuenta + su stock actual (para el sync continuo).
pub fn publicaciones_para_sync(conn: &mut PgConnection, cuenta_id: Uuid) -> ApiResult<Vec<SyncItem>> {
    let pubs: Vec<(String, Uuid, Option<String>)> = paris_publicaciones::table
        .filter(paris_publicaciones::cuenta_id.eq(cuenta_id))
        .select((paris_publicaciones::sku_seller, paris_publicaciones::producto_id, paris_publicaciones::paris_sku))
        .load(conn)?;
    if pubs.is_empty() {
        return Ok(vec![]);
    }
    let ids: Vec<Uuid> = pubs.iter().map(|(_, pid, _)| *pid).collect();
    let datos: Vec<(Uuid, i32, i64)> = productos::table
        .filter(productos::id.eq_any(&ids))
        .select((productos::id, productos::stock, productos::precio))
        .load(conn)?;
    let map: HashMap<Uuid, (i32, i64)> = datos.into_iter().map(|(id, st, pr)| (id, (st, pr))).collect();
    Ok(pubs
        .into_iter()
        .filter_map(|(sku, pid, psku)| map.get(&pid).map(|&(st, pr)| SyncItem { sku_seller: sku, stock: st, precio: pr, paris_sku: psku }))
        .collect())
}

/// producto_id asociado a un sku_seller en una cuenta (para aplicar ventas de Paris).
pub fn producto_de_sku(conn: &mut PgConnection, cuenta_id: Uuid, sku_seller: &str) -> ApiResult<Option<Uuid>> {
    paris_publicaciones::table
        .filter(paris_publicaciones::cuenta_id.eq(cuenta_id))
        .filter(paris_publicaciones::sku_seller.eq(sku_seller))
        .select(paris_publicaciones::producto_id)
        .first(conn)
        .optional()
        .map_err(ApiError::from)
}

/// Marca una orden como procesada; true si es NUEVA (no estaba registrada).
pub fn marcar_venta_si_nueva(conn: &mut PgConnection, order_number: &str, cuenta_id: Uuid) -> ApiResult<bool> {
    let n = diesel::insert_into(paris_ventas::table)
        .values(NewParisVenta { cuenta_id, order_number: order_number.to_string() })
        .on_conflict((paris_ventas::cuenta_id, paris_ventas::order_number))
        .do_nothing()
        .execute(conn)?;
    Ok(n > 0)
}
