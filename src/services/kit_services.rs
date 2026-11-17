//! Kits / combos: un producto (publicación) compuesto por varios productos componentes.
//! Al venderse el kit se descuenta el stock de cada componente; la disponibilidad del kit
//! = min(floor(stock_componente / cantidad)). Cuando un componente cambia de stock, los
//! kits que lo usan se recomputan y se empujan a Mercado Libre para no sobrevender.

use crate::errors::{ApiError, ApiResult, OrNotFound};
use crate::models::{NewKitComponente, Producto};
use crate::schema::{kit_componentes, ml_cuentas, productos};
use crate::services::pub_services::{aplicar_venta_externa, VentaAplicada};
use crate::services::producto_services::registrar_cambio;
use diesel::prelude::*;
use serde::Serialize;
use uuid::Uuid;

fn now() -> chrono::NaiveDateTime {
    chrono::Utc::now().naive_utc()
}

#[derive(Serialize)]
pub struct ComponenteView {
    pub producto_id: Uuid,
    pub titulo: String,
    pub thumbnail: Option<String>,
    pub stock: i32,
    pub cantidad: i32,
    /// Cuántos kits aporta este componente = floor(stock / cantidad).
    pub aporta: i32,
}

#[derive(Serialize)]
pub struct KitView {
    pub kit_producto_id: Uuid,
    pub titulo: String,
    pub thumbnail: Option<String>,
    /// Stock de la publicación del kit (lo que ve ML/los canales).
    pub stock_actual: i32,
    /// Disponibilidad calculada a partir de los componentes.
    pub disponible: i32,
    pub componentes: Vec<ComponenteView>,
}

/// Vendedor dueño de un producto (vía su cuenta de ML).
fn vendedor_de(conn: &mut PgConnection, producto_id: Uuid) -> ApiResult<Uuid> {
    let cuenta: Uuid = productos::table
        .find(producto_id)
        .select(productos::ml_cuenta_id)
        .first(conn)
        .optional()?
        .or_not_found("Producto")?;
    Ok(ml_cuentas::table.find(cuenta).select(ml_cuentas::vendedor_id).first(conn)?)
}

/// (componente_id, cantidad) de un kit.
pub fn componentes_raw(conn: &mut PgConnection, kit_producto_id: Uuid) -> ApiResult<Vec<(Uuid, i32)>> {
    Ok(kit_componentes::table
        .filter(kit_componentes::kit_producto_id.eq(kit_producto_id))
        .select((kit_componentes::componente_producto_id, kit_componentes::cantidad))
        .load(conn)?)
}

/// ¿El producto tiene componentes (es un kit)?
pub fn es_kit(conn: &mut PgConnection, producto_id: Uuid) -> ApiResult<bool> {
    let n: i64 = kit_componentes::table
        .filter(kit_componentes::kit_producto_id.eq(producto_id))
        .count()
        .get_result(conn)?;
    Ok(n > 0)
}

/// Disponibilidad del kit = min(floor(stock_componente / cantidad)). 0 si no hay componentes.
pub fn disponible(conn: &mut PgConnection, kit_producto_id: Uuid) -> ApiResult<i32> {
    let comps = componentes_raw(conn, kit_producto_id)?;
    if comps.is_empty() {
        return Ok(0);
    }
    let mut min = i32::MAX;
    for (cid, cant) in comps {
        let stock: i32 = productos::table
            .find(cid)
            .select(productos::stock)
            .first(conn)
            .optional()?
            .unwrap_or(0);
        let aporta = if cant > 0 { stock / cant } else { 0 };
        min = min.min(aporta);
    }
    Ok(min.max(0))
}

/// Kit ids (distintos) que usan `componente_id` como componente.
pub fn kits_que_usan(conn: &mut PgConnection, componente_id: Uuid) -> ApiResult<Vec<Uuid>> {
    Ok(kit_componentes::table
        .filter(kit_componentes::componente_producto_id.eq(componente_id))
        .select(kit_componentes::kit_producto_id)
        .distinct()
        .load(conn)?)
}

/// Fija el stock de la publicación del kit = disponibilidad. Si cambió, devuelve la
/// `VentaAplicada` para empujar el nuevo stock a Mercado Libre.
pub fn sincronizar_stock_kit(conn: &mut PgConnection, kit_producto_id: Uuid) -> ApiResult<Option<VentaAplicada>> {
    let disp = disponible(conn, kit_producto_id)?;
    let p: Producto = productos::table
        .find(kit_producto_id)
        .select(Producto::as_select())
        .first(conn)
        .optional()?
        .or_not_found("Producto")?;
    if p.stock == disp {
        return Ok(None);
    }
    diesel::update(productos::table.find(kit_producto_id))
        .set((productos::stock.eq(disp), productos::sincronizado_at.eq(now())))
        .execute(conn)?;
    registrar_cambio(conn, kit_producto_id, "stock", Some(p.stock.to_string()), disp.to_string(), "kit");
    let user_product_id = p
        .datos
        .get("user_product_id")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());
    Ok(Some(VentaAplicada { nuevo_stock: disp, ml_cuenta_id: p.ml_cuenta_id, ml_item_id: p.ml_item_id, user_product_id }))
}

/// Aplica la venta de `cantidad` del producto vendido (puede ser un kit o un producto normal)
/// y devuelve TODAS las `VentaAplicada` a empujar a Mercado Libre: la de cada componente (si
/// es kit) o la del propio producto (si no), más la de cada kit cuyo stock cambió.
pub fn aplicar_venta_con_kits(
    conn: &mut PgConnection,
    producto_id: Uuid,
    cantidad: i32,
    motivo: &str,
) -> ApiResult<Vec<VentaAplicada>> {
    conn.transaction(|conn| {
        let mut out: Vec<VentaAplicada> = Vec::new();
        let comps = componentes_raw(conn, producto_id)?;
        // Componentes cuyo stock bajó (para recomputar los kits que los usan).
        let mut tocados: Vec<Uuid> = Vec::new();
        if comps.is_empty() {
            out.push(aplicar_venta_externa(conn, producto_id, cantidad, motivo)?);
            tocados.push(producto_id);
        } else {
            for (cid, cant) in comps {
                out.push(aplicar_venta_externa(conn, cid, cantidad.saturating_mul(cant), motivo)?);
                tocados.push(cid);
            }
        }
        // Recomputa el stock de todos los kits afectados (incluido el propio kit vendido).
        let mut kits: Vec<Uuid> = Vec::new();
        for c in &tocados {
            for k in kits_que_usan(conn, *c)? {
                if !kits.contains(&k) {
                    kits.push(k);
                }
            }
        }
        for k in kits {
            if let Some(v) = sincronizar_stock_kit(conn, k)? {
                out.push(v);
            }
        }
        Ok(out)
    })
}

/// Define (o redefine) los componentes de un kit. Valida pertenencia y que no haya anidación
/// (un componente no puede ser kit, y un kit no puede ser componente de otro). Deja el stock
/// del kit igual a su disponibilidad y devuelve la `VentaAplicada` si cambió.
pub fn definir_kit(
    conn: &mut PgConnection,
    vendedor_id: Uuid,
    kit_producto_id: Uuid,
    componentes: Vec<(Uuid, i32)>,
) -> ApiResult<Option<VentaAplicada>> {
    if componentes.is_empty() {
        return Err(ApiError::bad_request("Un kit necesita al menos un componente"));
    }
    if vendedor_de(conn, kit_producto_id)? != vendedor_id {
        return Err(ApiError::forbidden("Ese producto no te pertenece"));
    }
    // El kit no puede ser, a su vez, componente de otro kit.
    let usado_como_comp: i64 = kit_componentes::table
        .filter(kit_componentes::componente_producto_id.eq(kit_producto_id))
        .count()
        .get_result(conn)?;
    if usado_como_comp > 0 {
        return Err(ApiError::bad_request("Ese producto ya es componente de otro kit; no puede ser también un kit"));
    }
    conn.transaction(|conn| {
        for (cid, cant) in &componentes {
            if *cant <= 0 {
                return Err(ApiError::bad_request("La cantidad de cada componente debe ser mayor que 0"));
            }
            if *cid == kit_producto_id {
                return Err(ApiError::bad_request("Un kit no puede contenerse a sí mismo"));
            }
            if vendedor_de(conn, *cid)? != vendedor_id {
                return Err(ApiError::forbidden("Un componente no te pertenece"));
            }
            let es_k: i64 = kit_componentes::table
                .filter(kit_componentes::kit_producto_id.eq(cid))
                .count()
                .get_result(conn)?;
            if es_k > 0 {
                return Err(ApiError::bad_request("Un componente no puede ser a su vez un kit"));
            }
        }
        diesel::delete(kit_componentes::table.filter(kit_componentes::kit_producto_id.eq(kit_producto_id))).execute(conn)?;
        for (cid, cant) in &componentes {
            diesel::insert_into(kit_componentes::table)
                .values(NewKitComponente {
                    vendedor_id,
                    kit_producto_id,
                    componente_producto_id: *cid,
                    cantidad: *cant,
                })
                .execute(conn)?;
        }
        sincronizar_stock_kit(conn, kit_producto_id)
    })
}

/// Deshace un kit (quita todos sus componentes). No toca el stock de la publicación.
pub fn eliminar_kit(conn: &mut PgConnection, vendedor_id: Uuid, kit_producto_id: Uuid) -> ApiResult<()> {
    if vendedor_de(conn, kit_producto_id)? != vendedor_id {
        return Err(ApiError::forbidden("Ese producto no te pertenece"));
    }
    diesel::delete(kit_componentes::table.filter(kit_componentes::kit_producto_id.eq(kit_producto_id))).execute(conn)?;
    Ok(())
}

/// Todos los kits del vendedor con sus componentes y disponibilidad.
pub fn kits_de(conn: &mut PgConnection, vendedor_id: Uuid) -> ApiResult<Vec<KitView>> {
    let kit_ids: Vec<Uuid> = kit_componentes::table
        .filter(kit_componentes::vendedor_id.eq(vendedor_id))
        .select(kit_componentes::kit_producto_id)
        .distinct()
        .load(conn)?;
    let mut out = Vec::new();
    for kid in kit_ids {
        let kp: Producto = match productos::table.find(kid).select(Producto::as_select()).first(conn).optional()? {
            Some(p) => p,
            None => continue,
        };
        let filas = componentes_raw(conn, kid)?;
        let mut comps = Vec::new();
        let mut disp = i32::MAX;
        for (cid, cant) in filas {
            let cp: Option<Producto> = productos::table.find(cid).select(Producto::as_select()).first(conn).optional()?;
            let (titulo, thumbnail, stock) = match cp {
                Some(p) => (p.titulo, p.thumbnail, p.stock),
                None => ("(producto eliminado)".to_string(), None, 0),
            };
            let aporta = if cant > 0 { stock / cant } else { 0 };
            disp = disp.min(aporta);
            comps.push(ComponenteView { producto_id: cid, titulo, thumbnail, stock, cantidad: cant, aporta });
        }
        let disponible = if comps.is_empty() { 0 } else { disp.max(0) };
        out.push(KitView {
            kit_producto_id: kid,
            titulo: kp.titulo,
            thumbnail: kp.thumbnail,
            stock_actual: kp.stock,
            disponible,
            componentes: comps,
        });
    }
    Ok(out)
}
