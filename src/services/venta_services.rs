//! Ventas centralizadas: importa las órdenes de Mercado Libre y calcula la
//! **ganancia real** = venta − comisión real de ML (`sale_fee`) − costo (el que
//! cargaste en Inventario). Expone el listado y un dashboard agregado.

use crate::errors::ApiResult;
use crate::models::{NewOrdenMl, OrdenMl};
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
    let mut top: Vec<TopProducto> = acc
        .into_iter()
        .map(|(id, (titulo, cant, vta))| TopProducto { ml_item_id: id, titulo, cantidad: cant, venta: vta })
        .collect();
    top.sort_by(|a, b| b.venta.cmp(&a.venta));
    top.truncate(10);

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
    })
}
