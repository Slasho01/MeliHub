use crate::errors::{ApiError, ApiResult};
use crate::ml::MlItem;
use crate::models::{HistorialProducto, MlCuenta, NewHistorialProducto, Producto, UpsertProducto};
use crate::schema::{historial_producto, ml_cuentas, productos};
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
    let vendedor_id: Option<Uuid> = ml_cuentas::table.find(cuenta_id).select(ml_cuentas::vendedor_id).first(conn).optional()?;
    let mut n = 0;
    for it in items {
        let rec = a_registro(cuenta_id, it);
        // Valor anterior (para registrar cambios de precio/stock en el historial).
        let anterior: Option<(Uuid, i64, i32)> = productos::table
            .filter(productos::ml_cuenta_id.eq(cuenta_id))
            .filter(productos::ml_item_id.eq(&it.id))
            .select((productos::id, productos::precio, productos::stock))
            .first(conn)
            .optional()?;
        diesel::insert_into(productos::table)
            .values(&rec)
            .on_conflict((productos::ml_cuenta_id, productos::ml_item_id))
            .do_update()
            .set(&rec)
            .execute(conn)?;
        if let Some((pid, precio_ant, stock_ant)) = anterior {
            if precio_ant != rec.precio {
                registrar_cambio(conn, pid, "precio", Some(precio_ant.to_string()), rec.precio.to_string(), "sync");
            }
            if stock_ant != rec.stock {
                registrar_cambio(conn, pid, "stock", Some(stock_ant.to_string()), rec.stock.to_string(), "sync");
            }
            // Quiebre de stock: notificar cuando llega a 0 (si estaba con stock).
            if rec.stock == 0 && stock_ant > 0 {
                if let Some(vid) = vendedor_id {
                    crate::services::saas_services::notificar(conn, vid, "quiebre", &format!("Quiebre de stock: {}", rec.titulo), Some(format!("La publicación {} se quedó sin stock en bodega.", it.id)));
                }
            }
        }
        n += 1;
    }
    Ok(n)
}

/// Registra un cambio en el historial (best-effort; no interrumpe la operación).
pub fn registrar_cambio(conn: &mut PgConnection, producto_id: Uuid, campo: &str, anterior: Option<String>, nuevo: String, fuente: &str) {
    let _ = diesel::insert_into(historial_producto::table)
        .values(NewHistorialProducto {
            producto_id,
            campo: campo.to_string(),
            anterior,
            nuevo: Some(nuevo),
            fuente: fuente.to_string(),
        })
        .execute(conn);
}

/// Historial de cambios de un producto (más recientes primero).
pub fn listar_historial(conn: &mut PgConnection, producto_id: Uuid, limit: i64) -> ApiResult<Vec<HistorialProducto>> {
    historial_producto::table
        .filter(historial_producto::producto_id.eq(producto_id))
        .order(historial_producto::created_at.desc())
        .limit(limit.clamp(1, 200))
        .select(HistorialProducto::as_select())
        .load(conn)
        .map_err(ApiError::from)
}

#[derive(Deserialize, Default)]
pub struct ProductoFiltro {
    pub cuenta_id: Option<Uuid>,
    pub q: Option<String>,
    /// Filtra por estado de la publicación en ML: "active" | "paused".
    pub estado: Option<String>,
}

/// Catálogo del vendedor. Siempre acotado a sus cuentas conectadas: al desconectar
/// una cuenta sus productos (CASCADE) desaparecen y no se filtran los de otros usuarios.
pub fn listar(
    conn: &mut PgConnection,
    vendedor_id: Uuid,
    _es_admin: bool,
    filtro: ProductoFiltro,
) -> ApiResult<Vec<Producto>> {
    let mut q = productos::table
        .inner_join(ml_cuentas::table)
        .filter(ml_cuentas::vendedor_id.eq(vendedor_id))
        .into_boxed();
    if let Some(cid) = filtro.cuenta_id {
        q = q.filter(productos::ml_cuenta_id.eq(cid));
    }
    if let Some(txt) = filtro.q.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        // Busca por título, por el código ML (ml_item_id) o por el SKU PROPIO del vendedor
        // (atributo SELLER_SKU de Mercado Libre). Si el vendedor no usa SKUs, ese atributo
        // viene vacío y simplemente no matchea — no afecta la búsqueda por título/código.
        let pat = format!("%{txt}%");
        let sku_vendedor = diesel::dsl::sql::<diesel::sql_types::Bool>(
            "EXISTS (SELECT 1 FROM jsonb_array_elements(COALESCE(productos.datos->'attributes','[]'::jsonb)) a WHERE a->>'id' = 'SELLER_SKU' AND a->>'value_name' ILIKE ",
        )
        .bind::<diesel::sql_types::Text, _>(pat.clone())
        .sql(")");
        q = q.filter(
            productos::titulo
                .ilike(pat.clone())
                .or(productos::ml_item_id.ilike(pat))
                .or(sku_vendedor),
        );
    }
    if let Some(est) = filtro.estado.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        q = q.filter(productos::estado.eq(est.to_string()));
    }
    q.select(Producto::as_select())
        .order(productos::sincronizado_at.desc())
        .limit(500)
        .load(conn)
        .map_err(ApiError::from)
}

/// Mapa `ml_item_id -> título` de las publicaciones del vendedor, para mostrar nombres
/// (p. ej. en Preguntas) sin llamar a Mercado Libre. Los item_ids que no tengamos no aparecen.
pub fn titulos_por_item(conn: &mut PgConnection, vendedor_id: Uuid, item_ids: &[String]) -> std::collections::HashMap<String, String> {
    if item_ids.is_empty() {
        return std::collections::HashMap::new();
    }
    productos::table
        .inner_join(ml_cuentas::table)
        .filter(ml_cuentas::vendedor_id.eq(vendedor_id))
        .filter(productos::ml_item_id.eq_any(item_ids))
        .select((productos::ml_item_id, productos::titulo))
        .load::<(String, String)>(conn)
        .map(|rows| rows.into_iter().collect())
        .unwrap_or_default()
}

/// Mapa `item_id -> (título, thumbnail)` desde la base local (para mostrar ficha + foto).
pub fn fichas_por_item(conn: &mut PgConnection, vendedor_id: Uuid, item_ids: &[String]) -> std::collections::HashMap<String, (String, Option<String>)> {
    if item_ids.is_empty() {
        return std::collections::HashMap::new();
    }
    productos::table
        .inner_join(ml_cuentas::table)
        .filter(ml_cuentas::vendedor_id.eq(vendedor_id))
        .filter(productos::ml_item_id.eq_any(item_ids))
        .select((productos::ml_item_id, productos::titulo, productos::thumbnail))
        .load::<(String, String, Option<String>)>(conn)
        .map(|rows| rows.into_iter().map(|(i, t, th)| (i, (t, th))).collect())
        .unwrap_or_default()
}

// ----------------------------- Control de publicaciones (Fase A) -----------------------------

/// Trae el producto y su cuenta ML (para el token), verificando propiedad.
pub fn get_con_cuenta(conn: &mut PgConnection, producto_id: Uuid, vendedor_id: Uuid, es_admin: bool) -> ApiResult<(Producto, MlCuenta)> {
    let prod: Producto = productos::table
        .find(producto_id)
        .select(Producto::as_select())
        .first(conn)
        .optional()?
        .ok_or_else(|| ApiError::not_found("Producto no encontrado"))?;
    let cuenta: MlCuenta = ml_cuentas::table.find(prod.ml_cuenta_id).select(MlCuenta::as_select()).first(conn)?;
    if !es_admin && cuenta.vendedor_id != vendedor_id {
        return Err(ApiError::forbidden("Ese producto no te pertenece"));
    }
    Ok((prod, cuenta))
}

/// Refleja en local el estado tras empujarlo a ML ("active"/"paused").
pub fn aplicar_estado_local(conn: &mut PgConnection, producto_id: Uuid, estado: &str) -> ApiResult<Producto> {
    let anterior: Option<String> = productos::table.find(producto_id).select(productos::estado).first(conn).optional()?;
    let p = diesel::update(productos::table.find(producto_id))
        .set((productos::estado.eq(estado), productos::sincronizado_at.eq(now())))
        .returning(Producto::as_returning())
        .get_result(conn)
        .map_err(ApiError::from)?;
    if anterior.as_deref() != Some(estado) {
        registrar_cambio(conn, producto_id, "estado", anterior, estado.to_string(), "manual");
    }
    Ok(p)
}

/// Refleja en local el precio tras empujarlo a ML.
pub fn aplicar_precio_local(conn: &mut PgConnection, producto_id: Uuid, precio: i64) -> ApiResult<Producto> {
    let anterior: Option<i64> = productos::table.find(producto_id).select(productos::precio).first(conn).optional()?;
    let p = diesel::update(productos::table.find(producto_id))
        .set((productos::precio.eq(precio.max(0)), productos::sincronizado_at.eq(now())))
        .returning(Producto::as_returning())
        .get_result(conn)
        .map_err(ApiError::from)?;
    if anterior != Some(p.precio) {
        registrar_cambio(conn, producto_id, "precio", anterior.map(|v| v.to_string()), p.precio.to_string(), "manual");
    }
    Ok(p)
}

/// Fija costo y/o precio JarStore a VARIOS productos a la vez (solo los del vendedor).
/// Devuelve cuántos productos propios se afectaron.
pub fn set_inventario_masivo(
    conn: &mut PgConnection,
    vendedor_id: Uuid,
    _es_admin: bool,
    ids: &[Uuid],
    costo: Option<i64>,
    precio_jarstore: Option<i64>,
) -> ApiResult<usize> {
    if ids.is_empty() {
        return Err(ApiError::bad_request("Selecciona al menos un producto"));
    }
    if ids.len() > 500 {
        return Err(ApiError::bad_request("Máximo 500 productos por operación"));
    }
    // Solo los IDs que pertenecen de verdad al vendedor (nunca de otro usuario).
    let propios: Vec<Uuid> = productos::table
        .inner_join(ml_cuentas::table)
        .filter(productos::id.eq_any(ids))
        .filter(ml_cuentas::vendedor_id.eq(vendedor_id))
        .select(productos::id)
        .load(conn)?;
    if let Some(c) = costo {
        diesel::update(productos::table.filter(productos::id.eq_any(&propios)))
            .set(productos::costo.eq(Some(c.max(0))))
            .execute(conn)?;
    }
    if let Some(p) = precio_jarstore {
        diesel::update(productos::table.filter(productos::id.eq_any(&propios)))
            .set(productos::precio_jarstore.eq(Some(p.max(0))))
            .execute(conn)?;
    }
    Ok(propios.len())
}

/// Productos ACTIVOS con stock (depósito) en o por debajo del umbral. Para alertas.
pub fn stock_bajo(conn: &mut PgConnection, vendedor_id: Uuid, _es_admin: bool, umbral: i32) -> ApiResult<Vec<Producto>> {
    productos::table
        .inner_join(ml_cuentas::table)
        .filter(ml_cuentas::vendedor_id.eq(vendedor_id))
        .filter(productos::estado.eq("active"))
        .filter(productos::stock.le(umbral))
        .order(productos::stock.asc())
        .limit(200)
        .select(Producto::as_select())
        .load(conn)
        .map_err(ApiError::from)
}
