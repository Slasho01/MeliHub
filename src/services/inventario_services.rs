use crate::errors::{ApiError, ApiResult};
use crate::models::{ConfigGanancias, Producto};
use crate::schema::{config_ganancias, ml_cuentas, productos};
use crate::services::common::now;
use crate::services::producto_services::{self, ProductoFiltro};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ----------------------------- Config de ganancias -----------------------------

/// Devuelve la config del vendedor, creándola con valores por defecto si no existe.
pub fn config(conn: &mut PgConnection, vendedor_id: Uuid) -> ApiResult<ConfigGanancias> {
    if let Some(c) = config_ganancias::table.find(vendedor_id).select(ConfigGanancias::as_select()).first(conn).optional()? {
        return Ok(c);
    }
    let def = ConfigGanancias {
        vendedor_id,
        precios_con_iva: true,
        iva_pct: 19.0,
        comision_ml_pct: 13.0,
        comision_ml_fija: 0,
        comision_mp_pct: 3.5,
        comision_mp_fija: 0,
        updated_at: now(),
        margen_objetivo: 30.0,
        redondeo: 0,
    };
    diesel::insert_into(config_ganancias::table)
        .values(&def)
        .on_conflict_do_nothing()
        .execute(conn)?;
    config_ganancias::table.find(vendedor_id).select(ConfigGanancias::as_select()).first(conn).map_err(ApiError::from)
}

#[derive(Deserialize)]
pub struct ConfigUpdate {
    pub precios_con_iva: Option<bool>,
    pub iva_pct: Option<f64>,
    pub comision_ml_pct: Option<f64>,
    pub comision_ml_fija: Option<i64>,
    pub comision_mp_pct: Option<f64>,
    pub comision_mp_fija: Option<i64>,
    pub margen_objetivo: Option<f64>,
    pub redondeo: Option<i64>,
}

pub fn set_config(conn: &mut PgConnection, vendedor_id: Uuid, u: ConfigUpdate) -> ApiResult<ConfigGanancias> {
    let mut c = config(conn, vendedor_id)?;
    if let Some(v) = u.precios_con_iva { c.precios_con_iva = v; }
    if let Some(v) = u.iva_pct { c.iva_pct = v.clamp(0.0, 100.0); }
    if let Some(v) = u.comision_ml_pct { c.comision_ml_pct = v.clamp(0.0, 100.0); }
    if let Some(v) = u.comision_ml_fija { c.comision_ml_fija = v.max(0); }
    if let Some(v) = u.comision_mp_pct { c.comision_mp_pct = v.clamp(0.0, 100.0); }
    if let Some(v) = u.comision_mp_fija { c.comision_mp_fija = v.max(0); }
    if let Some(v) = u.margen_objetivo { c.margen_objetivo = v.clamp(0.0, 95.0); }
    if let Some(v) = u.redondeo { c.redondeo = v.clamp(0, 999); }
    c.updated_at = now();
    diesel::update(config_ganancias::table.find(vendedor_id)).set(&c).execute(conn)?;
    Ok(c)
}

// ----------------------------- Editar inventario -----------------------------

#[derive(Deserialize)]
pub struct InventarioUpdate {
    pub costo: Option<i64>,
    pub precio_jarstore: Option<i64>,
}

/// Fija el costo y/o el precio JarStore de un producto (verifica propiedad).
pub fn set_inventario(conn: &mut PgConnection, producto_id: Uuid, vendedor_id: Uuid, es_admin: bool, u: InventarioUpdate) -> ApiResult<Producto> {
    // Verificar que el producto pertenezca al vendedor (vía su cuenta ML).
    let dueno: Option<Uuid> = productos::table
        .inner_join(ml_cuentas::table)
        .filter(productos::id.eq(producto_id))
        .select(ml_cuentas::vendedor_id)
        .first(conn)
        .optional()?;
    let dueno = dueno.ok_or_else(|| ApiError::not_found("Producto no encontrado"))?;
    if !es_admin && dueno != vendedor_id {
        return Err(ApiError::forbidden("Ese producto no te pertenece"));
    }
    diesel::update(productos::table.find(producto_id))
        .set((
            productos::costo.eq(u.costo.filter(|c| *c >= 0)),
            productos::precio_jarstore.eq(u.precio_jarstore.filter(|p| *p >= 0)),
        ))
        .returning(Producto::as_returning())
        .get_result(conn)
        .map_err(ApiError::from)
}

// ----------------------------- Cálculo de ganancias -----------------------------

#[derive(Serialize)]
pub struct Ganancia {
    pub precio: i64,
    pub neto: i64,
    pub iva: i64,
    pub comision: i64,
    pub ganancia: i64,
    pub margen: f64, // % sobre el precio
}

/// Ganancia estimada de un canal. `costo` se asume NETO (sin IVA).
fn calcular(precio: i64, costo: i64, com_pct: f64, com_fija: i64, cfg: &ConfigGanancias) -> Ganancia {
    let div = if cfg.precios_con_iva { 1.0 + cfg.iva_pct / 100.0 } else { 1.0 };
    let p = precio as f64;
    let neto = p / div;
    let iva = p - neto;
    let comision = p * com_pct / 100.0 + com_fija as f64;
    let ganancia = neto - costo as f64 - comision;
    let margen = if precio > 0 { ganancia / p * 100.0 } else { 0.0 };
    Ganancia {
        precio,
        neto: neto.round() as i64,
        iva: iva.round() as i64,
        comision: comision.round() as i64,
        ganancia: ganancia.round() as i64,
        margen: (margen * 10.0).round() / 10.0,
    }
}

#[derive(Serialize)]
pub struct ProductoInventario {
    #[serde(flatten)]
    pub producto: Producto,
    /// Ganancia en Mercado Libre (usa el precio de ML `precio`).
    pub ml: Option<Ganancia>,
    /// Ganancia en JarStore (usa `precio_jarstore`).
    pub jarstore: Option<Ganancia>,
    /// Precio JarStore sugerido para alcanzar el margen objetivo (si hay costo).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub precio_sugerido: Option<i64>,
}

/// Precio (canal JarStore) que alcanza el margen objetivo para un costo dado.
fn sugerir_precio(costo: i64, cfg: &ConfigGanancias) -> Option<i64> {
    let div = if cfg.precios_con_iva { 1.0 + cfg.iva_pct / 100.0 } else { 1.0 };
    let m = cfg.margen_objetivo / 100.0;
    let denom = 1.0 / div - cfg.comision_mp_pct / 100.0 - m;
    if denom <= 0.0 {
        return None; // margen inalcanzable con esas comisiones/IVA
    }
    let precio = (costo as f64 + cfg.comision_mp_fija as f64) / denom;
    if !precio.is_finite() || precio <= 0.0 {
        return None;
    }
    Some(redondear(precio.ceil() as i64, cfg.redondeo))
}

/// Redondeo psicológico: el menor número ≥ `precio` que termina en `redondeo` (ej. 990).
fn redondear(precio: i64, redondeo: i64) -> i64 {
    if redondeo <= 0 {
        return precio;
    }
    let m = 1000;
    let base = (precio / m) * m + redondeo;
    if base >= precio {
        base
    } else {
        base + m
    }
}

/// Lista los productos con la ganancia calculada por canal.
pub fn listar(conn: &mut PgConnection, vendedor_id: Uuid, es_admin: bool, filtro: ProductoFiltro) -> ApiResult<Vec<ProductoInventario>> {
    let cfg = config(conn, vendedor_id)?;
    let productos = producto_services::listar(conn, vendedor_id, es_admin, filtro)?;
    Ok(productos
        .into_iter()
        .map(|p| {
            let costo = p.costo.unwrap_or(0);
            let ml = if p.precio > 0 { Some(calcular(p.precio, costo, cfg.comision_ml_pct, cfg.comision_ml_fija, &cfg)) } else { None };
            let jarstore = p.precio_jarstore.filter(|v| *v > 0).map(|pj| calcular(pj, costo, cfg.comision_mp_pct, cfg.comision_mp_fija, &cfg));
            let precio_sugerido = if costo > 0 { sugerir_precio(costo, &cfg) } else { None };
            ProductoInventario { producto: p, ml, jarstore, precio_sugerido }
        })
        .collect())
}
