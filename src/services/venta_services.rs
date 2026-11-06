//! Ventas centralizadas: importa las órdenes de Mercado Libre y calcula la
//! **ganancia real** = venta − comisión real de ML (`sale_fee`) − costo (el que
//! cargaste en Inventario). Expone el listado y un dashboard agregado.

use crate::errors::{ApiError, ApiResult};
use crate::models::{MlCuenta, NewOrdenMl, OrdenMl, Producto};
use crate::schema::{ml_cuentas, ordenes_ml, productos};
use chrono::{Duration, NaiveDate, NaiveDateTime, Utc};
use diesel::prelude::*;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};
use uuid::Uuid;

fn parse_fecha(v: Option<&Value>) -> NaiveDateTime {
    v.and_then(|x| x.as_str())
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.naive_utc())
        .unwrap_or_else(|| Utc::now().naive_utc())
}

/// Costo cargado en Inventario para un ítem de esa cuenta (None si no se definió).
fn costo_de(conn: &mut PgConnection, cuenta_id: Uuid, item_id: &str) -> ApiResult<Option<i64>> {
    if item_id.is_empty() {
        return Ok(None);
    }
    let c: Option<Option<i64>> = productos::table
        .filter(productos::ml_cuenta_id.eq(cuenta_id))
        .filter(productos::ml_item_id.eq(item_id))
        .select(productos::costo)
        .first(conn)
        .optional()?;
    Ok(c.flatten())
}

/// Convierte una orden cruda de ML en registro local, calculando la ganancia real.
fn a_orden(conn: &mut PgConnection, cuenta_id: Uuid, o: &Value) -> ApiResult<NewOrdenMl> {
    let ml_order_id = o.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
    let estado = o.get("status").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
    let fecha = parse_fecha(o.get("date_created"));
    let moneda = o.get("currency_id").and_then(|v| v.as_str()).unwrap_or("CLP").to_string();
    let comprador = o
        .get("buyer")
        .and_then(|b| b.get("nickname"))
        .and_then(|v| v.as_str())
        .map(|s| s.chars().take(255).collect());

    let (mut venta, mut comision, mut costo) = (0i64, 0i64, 0i64);
    let mut items_out: Vec<Value> = Vec::new();
    if let Some(arr) = o.get("order_items").and_then(|v| v.as_array()) {
        for it in arr {
            let qty = it.get("quantity").and_then(|v| v.as_i64()).unwrap_or(0).max(0);
            let unit = it.get("unit_price").and_then(|v| v.as_f64()).unwrap_or(0.0).round() as i64;
            let fee_unit = it.get("sale_fee").and_then(|v| v.as_f64()).unwrap_or(0.0).round() as i64;
            let item_id = it.get("item").and_then(|i| i.get("id")).and_then(|v| v.as_str()).unwrap_or("");
            let titulo = it.get("item").and_then(|i| i.get("title")).and_then(|v| v.as_str()).unwrap_or("");
            let c = costo_de(conn, cuenta_id, item_id)?.unwrap_or(0);
            venta += unit * qty;
            comision += fee_unit * qty;
            costo += c * qty;
            items_out.push(json!({
                "ml_item_id": item_id,
                "titulo": titulo,
                "cantidad": qty,
                "precio_unitario": unit,
                "comision": fee_unit * qty,
                "costo_unitario": c,
            }));
        }
    }
    let ganancia = venta - comision - costo;
    let envio_id = o.get("shipping").and_then(|s| s.get("id")).and_then(|v| v.as_i64()).filter(|&id| id > 0);
    Ok(NewOrdenMl {
        ml_cuenta_id: cuenta_id,
        ml_order_id,
        estado,
        fecha,
        moneda,
        comprador,
        venta_total: venta,
        comision_total: comision,
        costo_total: costo,
        ganancia,
        items: json!(items_out),
        datos: o.clone(),
        envio_id,
    })
}

/// Inserta/actualiza las órdenes de una cuenta. Devuelve cuántas.
pub fn upsert_ordenes(conn: &mut PgConnection, cuenta_id: Uuid, orders: &[Value]) -> ApiResult<usize> {
    let mut n = 0;
    for o in orders {
        let rec = a_orden(conn, cuenta_id, o)?;
        if rec.ml_order_id == 0 {
            continue;
        }
        diesel::insert_into(ordenes_ml::table)
            .values(&rec)
            .on_conflict((ordenes_ml::ml_cuenta_id, ordenes_ml::ml_order_id))
            .do_update()
            .set(&rec)
            .execute(conn)?;
        n += 1;
    }
    Ok(n)
}

// ----------------------------- Consultas -----------------------------

/// Órdenes del vendedor (de todas sus cuentas, o una), más recientes primero.
pub fn listar(conn: &mut PgConnection, vendedor_id: Uuid, cuenta_id: Option<Uuid>, limit: i64, offset: i64) -> ApiResult<Vec<OrdenMl>> {
    let mut q = ordenes_ml::table
        .inner_join(ml_cuentas::table)
        .filter(ml_cuentas::vendedor_id.eq(vendedor_id))
        .into_boxed();
    if let Some(cid) = cuenta_id {
        q = q.filter(ordenes_ml::ml_cuenta_id.eq(cid));
    }
    Ok(q.order(ordenes_ml::fecha.desc())
        .limit(limit.clamp(1, 200))
        .offset(offset.max(0))
        .select(OrdenMl::as_select())
        .load(conn)?)
}

#[derive(Serialize)]
pub struct DiaVenta {
    pub dia: String,
    pub venta: i64,
    pub ganancia: i64,
    pub ordenes: i64,
}

#[derive(Serialize)]
pub struct TopProducto {
    pub ml_item_id: String,
    pub titulo: String,
    pub cantidad: i64,
    pub venta: i64,
}

#[derive(Serialize)]
pub struct Dashboard {
    pub dias: i64,
    pub ordenes: i64,
    pub venta_total: i64,
    pub ganancia_total: i64,
    pub comision_total: i64,
    pub costo_total: i64,
    pub ticket_promedio: i64,
    pub margen_pct: f64,
    pub serie: Vec<DiaVenta>,
    pub top_productos: Vec<TopProducto>,
    /// Productos activos con stock que NO vendieron en el período (hasta 10).
    pub sin_rotacion: Vec<ProductoSimple>,
    pub sin_rotacion_total: i64,
    /// Productos activos sin costo cargado (la ganancia no es confiable).
    pub sin_costo: i64,
}

#[derive(Serialize)]
pub struct ProductoSimple {
    pub ml_item_id: String,
    pub titulo: String,
    pub stock: i32,
}

/// Dashboard agregado de los últimos `dias` (excluye órdenes canceladas).
pub fn dashboard(conn: &mut PgConnection, vendedor_id: Uuid, dias: i64) -> ApiResult<Dashboard> {
    let dias = dias.clamp(1, 730);
    let desde = Utc::now().naive_utc() - Duration::days(dias);
    let ordenes: Vec<OrdenMl> = ordenes_ml::table
        .inner_join(ml_cuentas::table)
        .filter(ml_cuentas::vendedor_id.eq(vendedor_id))
        .filter(ordenes_ml::estado.ne("cancelled"))
        .filter(ordenes_ml::fecha.ge(desde))
        .select(OrdenMl::as_select())
        .load(conn)?;

    let n = ordenes.len() as i64;
    let venta: i64 = ordenes.iter().map(|o| o.venta_total).sum();
    let ganancia: i64 = ordenes.iter().map(|o| o.ganancia).sum();
    let comision: i64 = ordenes.iter().map(|o| o.comision_total).sum();
    let costo: i64 = ordenes.iter().map(|o| o.costo_total).sum();

    // Serie por día
    let mut por_dia: BTreeMap<NaiveDate, (i64, i64, i64)> = BTreeMap::new();
    for o in &ordenes {
        let e = por_dia.entry(o.fecha.date()).or_insert((0, 0, 0));
        e.0 += o.venta_total;
        e.1 += o.ganancia;
        e.2 += 1;
    }
    let serie = por_dia
        .into_iter()
        .map(|(d, (v, g, c))| DiaVenta { dia: d.format("%Y-%m-%d").to_string(), venta: v, ganancia: g, ordenes: c })
        .collect();

    // Top productos (por venta)
    let mut acc: HashMap<String, (String, i64, i64)> = HashMap::new();
    for o in &ordenes {
        if let Some(items) = o.items.as_array() {
            for it in items {
                let id = it.get("ml_item_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                if id.is_empty() {
                    continue;
                }
                let titulo = it.get("titulo").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let qty = it.get("cantidad").and_then(|v| v.as_i64()).unwrap_or(0);
                let precio = it.get("precio_unitario").and_then(|v| v.as_i64()).unwrap_or(0);
                let e = acc.entry(id).or_insert((titulo, 0, 0));
                e.1 += qty;
                e.2 += precio * qty;
            }
        }
    }
    let vendidos: std::collections::HashSet<String> = acc.keys().cloned().collect();
    let mut top: Vec<TopProducto> = acc
        .into_iter()
        .map(|(id, (titulo, cant, vta))| TopProducto { ml_item_id: id, titulo, cantidad: cant, venta: vta })
        .collect();
    top.sort_by(|a, b| b.venta.cmp(&a.venta));
    top.truncate(10);

    // Insights de inventario: sin rotación (activos con stock que no vendieron) y sin costo.
    let prods: Vec<Producto> = productos::table
        .inner_join(ml_cuentas::table)
        .filter(ml_cuentas::vendedor_id.eq(vendedor_id))
        .select(Producto::as_select())
        .load(conn)?;
    let mut sin_rot: Vec<ProductoSimple> = prods
        .iter()
        .filter(|p| p.estado == "active" && p.stock > 0 && !vendidos.contains(&p.ml_item_id))
        .map(|p| ProductoSimple { ml_item_id: p.ml_item_id.clone(), titulo: p.titulo.clone(), stock: p.stock })
        .collect();
    let sin_rotacion_total = sin_rot.len() as i64;
    sin_rot.sort_by(|a, b| b.stock.cmp(&a.stock));
    sin_rot.truncate(10);
    let sin_costo = prods.iter().filter(|p| p.estado == "active" && p.costo.unwrap_or(0) <= 0).count() as i64;

    Ok(Dashboard {
        dias,
        ordenes: n,
        venta_total: venta,
        ganancia_total: ganancia,
        comision_total: comision,
        costo_total: costo,
        ticket_promedio: if n > 0 { venta / n } else { 0 },
        margen_pct: if venta > 0 { (ganancia as f64 / venta as f64) * 100.0 } else { 0.0 },
        serie,
        top_productos: top,
        sin_rotacion: sin_rot,
        sin_rotacion_total,
        sin_costo,
    })
}

/// Órdenes del período (no canceladas) para exportar. Reusa el mismo filtro que el dashboard.
pub fn ordenes_periodo(conn: &mut PgConnection, vendedor_id: Uuid, dias: i64) -> ApiResult<Vec<OrdenMl>> {
    let dias = dias.clamp(1, 730);
    let desde = Utc::now().naive_utc() - Duration::days(dias);
    ordenes_ml::table
        .inner_join(ml_cuentas::table)
        .filter(ml_cuentas::vendedor_id.eq(vendedor_id))
        .filter(ordenes_ml::estado.ne("cancelled"))
        .filter(ordenes_ml::fecha.ge(desde))
        .order(ordenes_ml::fecha.desc())
        .select(OrdenMl::as_select())
        .load(conn)
        .map_err(crate::errors::ApiError::from)
}

// ----------------------------- Envíos / etiquetas -----------------------------

const ESTADOS_TERMINALES: [&str; 3] = ["delivered", "not_delivered", "cancelled"];

#[derive(Serialize)]
pub struct EnvioRow {
    pub id: Uuid,
    pub ml_order_id: i64,
    pub cuenta: String,
    pub comprador: Option<String>,
    pub fecha: NaiveDateTime,
    pub estado: String,
    pub envio_id: i64,
    pub productos: String,
    pub envio_estado: Option<String>,
    pub logistic_type: Option<String>,
}

fn resumen_items(items: &Value) -> String {
    items
        .as_array()
        .map(|arr| arr.iter().map(|it| {
            let q = it.get("cantidad").and_then(|v| v.as_i64()).unwrap_or(1);
            let t = it.get("titulo").and_then(|v| v.as_str()).unwrap_or("");
            format!("{q}× {t}")
        }).collect::<Vec<_>>().join(", "))
        .unwrap_or_default()
}

/// Lista de envíos (sin FULL) con estado, para mostrar en el panel.
pub fn envios(conn: &mut PgConnection, vendedor_id: Uuid, ini: NaiveDateTime, fin: NaiveDateTime) -> ApiResult<Vec<EnvioRow>> {
    let rows: Vec<(OrdenMl, Option<String>)> = ordenes_ml::table
        .inner_join(ml_cuentas::table)
        .filter(ml_cuentas::vendedor_id.eq(vendedor_id))
        .filter(ordenes_ml::envio_id.is_not_null())
        .filter(ordenes_ml::estado.ne("cancelled"))
        .filter(ordenes_ml::fecha.ge(ini))
        .filter(ordenes_ml::fecha.lt(fin))
        .filter(ordenes_ml::logistic_type.ne("fulfillment").or(ordenes_ml::logistic_type.is_null()))
        .order(ordenes_ml::fecha.desc())
        .select((OrdenMl::as_select(), ml_cuentas::nickname.nullable()))
        .load(conn)?;
    Ok(rows.into_iter().map(|(o, nick)| EnvioRow {
        id: o.id,
        ml_order_id: o.ml_order_id,
        cuenta: nick.unwrap_or_else(|| "Cuenta".to_string()),
        comprador: o.comprador,
        fecha: o.fecha,
        estado: o.estado,
        envio_id: o.envio_id.unwrap_or(0),
        productos: resumen_items(&o.items),
        envio_estado: o.envio_estado,
        logistic_type: o.logistic_type,
    }).collect())
}

/// Agrupa por cuenta los envíos imprimibles del rango (para descargar etiquetas con cada token).
pub fn envios_por_cuenta(conn: &mut PgConnection, vendedor_id: Uuid, ini: NaiveDateTime, fin: NaiveDateTime) -> ApiResult<Vec<(MlCuenta, Vec<i64>)>> {
    let pairs: Vec<(MlCuenta, Option<i64>)> = ordenes_ml::table
        .inner_join(ml_cuentas::table)
        .filter(ml_cuentas::vendedor_id.eq(vendedor_id))
        .filter(ordenes_ml::envio_id.is_not_null())
        .filter(ordenes_ml::estado.ne("cancelled"))
        .filter(ordenes_ml::fecha.ge(ini))
        .filter(ordenes_ml::fecha.lt(fin))
        .filter(ordenes_ml::logistic_type.ne("fulfillment").or(ordenes_ml::logistic_type.is_null()))
        .select((MlCuenta::as_select(), ordenes_ml::envio_id))
        .load(conn)?;
    let mut map: std::collections::HashMap<Uuid, (MlCuenta, Vec<i64>)> = std::collections::HashMap::new();
    for (cuenta, envio) in pairs {
        if let Some(e) = envio {
            map.entry(cuenta.id).or_insert_with(|| (cuenta.clone(), Vec::new())).1.push(e);
        }
    }
    Ok(map.into_values().collect())
}

/// Órdenes del rango que necesitan consultar su envío a ML. `todos=false` solo las sin
/// clasificar (logistic_type nulo); `todos=true` también las de estado no terminal (refresco).
pub fn para_enriquecer(conn: &mut PgConnection, vendedor_id: Uuid, ini: NaiveDateTime, fin: NaiveDateTime, todos: bool) -> ApiResult<Vec<(Uuid, MlCuenta, i64)>> {
    let rows: Vec<(Uuid, MlCuenta, Option<i64>, Option<String>, Option<String>)> = ordenes_ml::table
        .inner_join(ml_cuentas::table)
        .filter(ml_cuentas::vendedor_id.eq(vendedor_id))
        .filter(ordenes_ml::envio_id.is_not_null())
        .filter(ordenes_ml::estado.ne("cancelled"))
        .filter(ordenes_ml::fecha.ge(ini))
        .filter(ordenes_ml::fecha.lt(fin))
        .select((ordenes_ml::id, MlCuenta::as_select(), ordenes_ml::envio_id, ordenes_ml::logistic_type, ordenes_ml::envio_estado))
        .load(conn)?;
    Ok(rows.into_iter().filter_map(|(id, cuenta, envio, lt, est)| {
        let envio = envio?;
        let necesita = lt.is_none()
            || (todos && !est.as_deref().map(|e| ESTADOS_TERMINALES.contains(&e)).unwrap_or(false));
        necesita.then_some((id, cuenta, envio))
    }).collect())
}

/// Guarda el tipo logístico y el estado de envío de una orden.
pub fn set_envio_info(conn: &mut PgConnection, orden_id: Uuid, logistic_type: &str, estado: &str) -> ApiResult<()> {
    diesel::update(ordenes_ml::table.find(orden_id))
        .set((ordenes_ml::logistic_type.eq(logistic_type), ordenes_ml::envio_estado.eq(estado)))
        .execute(conn)?;
    Ok(())
}

/// Cuenta + envio_id de una orden puntual (verifica dueño y que no sea FULL).
pub fn orden_para_etiqueta(conn: &mut PgConnection, vendedor_id: Uuid, orden_id: Uuid) -> ApiResult<(MlCuenta, i64)> {
    let row: Option<(MlCuenta, Option<i64>, Option<String>)> = ordenes_ml::table
        .inner_join(ml_cuentas::table)
        .filter(ordenes_ml::id.eq(orden_id))
        .filter(ml_cuentas::vendedor_id.eq(vendedor_id))
        .select((MlCuenta::as_select(), ordenes_ml::envio_id, ordenes_ml::logistic_type))
        .first(conn)
        .optional()?;
    let (cuenta, envio, lt) = row.ok_or_else(|| ApiError::not_found("Orden"))?;
    if lt.as_deref() == Some("fulfillment") {
        return Err(ApiError::bad_request("Esta venta es FULL: la etiqueta la gestiona Mercado Libre."));
    }
    let envio = envio.ok_or_else(|| ApiError::bad_request("Esta venta no tiene etiqueta imprimible."))?;
    Ok((cuenta, envio))
}
