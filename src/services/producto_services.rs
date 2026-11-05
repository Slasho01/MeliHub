use crate::errors::{ApiError, ApiResult};
use crate::ml::MlItem;
use crate::models::{MlCuenta, Producto, UpsertProducto};
use crate::schema::{ml_cuentas, productos};
use crate::services::common::now;
use diesel::prelude::*;
use serde::Deserialize;
use uuid::Uuid;

/// Stock que el vendedor realmente puede despachar: el de su **bodega (depósito)**.
/// El de FULL (meli_facility) lo gestiona Mercado Libre y no cuenta.
fn stock_vendible(it: &MlItem) -> i32 {
    if let Some(d) = it.deposito_stock {
        return d.clamp(0, i32::MAX as i64) as i32;
    }
    let full = it
        .shipping
        .get("logistic_type")
        .and_then(|l| l.as_str())
        .map(|t| t.eq_ignore_ascii_case("fulfillment"))
        .unwrap_or(false);
    if full {
        0
    } else {
        it.available_quantity.unwrap_or(0).clamp(0, i32::MAX as i64) as i32
    }
}

/// Convierte un ítem de Mercado Libre al registro local.
fn a_registro(cuenta_id: Uuid, it: &MlItem) -> UpsertProducto {
    UpsertProducto {
        ml_cuenta_id: cuenta_id,
        ml_item_id: it.id.clone(),
        titulo: it.title.chars().take(500).collect(),
        precio: it.price.map(|p| p.round() as i64).unwrap_or(0),
        moneda: it.currency_id.clone().unwrap_or_else(|| "CLP".into()),
        stock: stock_vendible(it),
        estado: it.status.clone().unwrap_or_else(|| "active".into()),
        permalink: it.permalink.clone(),
        thumbnail: it.thumbnail.clone(),
        categoria_ml: it.category_id.clone(),
        variantes: it.variations.clone(),
        datos: serde_json::to_value(it).unwrap_or_else(|_| serde_json::json!({})),
        sincronizado_at: now(),
    }
}

/// Inserta o actualiza las publicaciones de una cuenta. Devuelve cuántas.
pub fn upsert_items(conn: &mut PgConnection, cuenta_id: Uuid, items: &[MlItem]) -> ApiResult<usize> {
    let mut n = 0;
    for it in items {
        let rec = a_registro(cuenta_id, it);
        diesel::insert_into(productos::table)
            .values(&rec)
            .on_conflict((productos::ml_cuenta_id, productos::ml_item_id))
            .do_update()
            .set(&rec)
            .execute(conn)?;
        n += 1;
    }
    Ok(n)
}

#[derive(Deserialize, Default)]
pub struct ProductoFiltro {
    pub cuenta_id: Option<Uuid>,
    pub q: Option<String>,
}

/// Catálogo unificado del vendedor (o de todos, si es admin).
pub fn listar(
    conn: &mut PgConnection,
    vendedor_id: Uuid,
    es_admin: bool,
    filtro: ProductoFiltro,
) -> ApiResult<Vec<Producto>> {
    let mut q = productos::table.inner_join(ml_cuentas::table).into_boxed();
    if !es_admin {
        q = q.filter(ml_cuentas::vendedor_id.eq(vendedor_id));
    }
    if let Some(cid) = filtro.cuenta_id {
        q = q.filter(productos::ml_cuenta_id.eq(cid));
    }
    if let Some(txt) = filtro.q.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        q = q.filter(productos::titulo.ilike(format!("%{txt}%")));
    }
    q.select(Producto::as_select())
        .order(productos::sincronizado_at.desc())
        .limit(500)
        .load(conn)
        .map_err(ApiError::from)
}
