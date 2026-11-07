use crate::errors::{ApiError, ApiResult, OrNotFound};
use crate::models::{ApiKey, Producto};
use crate::schema::productos;
use crate::services::common::now;
use diesel::prelude::*;
use serde::Serialize;
use uuid::Uuid;

/// Una especificación técnica (par nombre/valor), tomada de los `attributes` de ML.
#[derive(Serialize)]
pub struct Especificacion {
    pub nombre: String,
    pub valor: String,
}

/// Producto tal como lo ve una plataforma externa (precio del canal de la key).
#[derive(Serialize)]
pub struct PublicProducto {
    pub id: Uuid,
    pub ml_item_id: String,
    pub titulo: String,
    pub precio: i64,
    pub moneda: String,
    pub stock: i32,
    pub estado: String,
    pub thumbnail: Option<String>,
    pub canal: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub descripcion: Option<String>,
    pub especificaciones: Vec<Especificacion>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub categoria: Option<String>,
}

/// Extrae descripción y ficha técnica del JSON `datos` (el item de ML guardado).
fn ficha(datos: &serde_json::Value) -> (Option<String>, Vec<Especificacion>) {
    let descripcion = datos
        .get("descripcion")
        .and_then(|d| d.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let especificaciones = datos
        .get("attributes")
        .and_then(|a| a.as_array())
        .map(|arr| {
            // Atributos meta/logísticos de ML que no son ficha técnica útil.
            const RUIDO: &[&str] = &[
                "GTIN", "EMPTY_GTIN_REASON", "SELLER_SKU", "PACKAGE_WEIGHT", "PACKAGE_LENGTH",
                "PACKAGE_WIDTH", "PACKAGE_HEIGHT", "PACKAGING_TYPE", "SHIPMENT_PACKING",
            ];
            arr.iter()
                .filter_map(|a| {
                    let id = a.get("id").and_then(|v| v.as_str()).unwrap_or("");
                    if RUIDO.contains(&id) {
                        return None;
                    }
                    let nombre = a.get("name").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
                    let valor = a.get("value_name").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
                    // Dimensiones logísticas del envío: no son ficha técnica del producto.
                    let n = nombre.to_lowercase();
                    if n.contains("paquete del seller")
                        || n.contains("código universal")
                        || n.contains("pymes")
                        || n.contains("origen del dato")
                        || n.contains("orígen del dato")
                        || n.contains("dato del paquete")
                    {
                        return None;
                    }
                    if nombre.is_empty() || valor.is_empty() {
                        None
                    } else {
                        Some(Especificacion { nombre, valor })
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    (descripcion, especificaciones)
}

/// Mercado Libre sirve el thumbnail en baja (`-I`, ~1 KB). Forzamos https y la
/// variante `-O` (original, mucho mejor calidad) para todos los canales.
fn mejor_imagen(url: Option<String>) -> Option<String> {
    url.map(|u| {
        let u = if let Some(r) = u.strip_prefix("http://") { format!("https://{r}") } else { u };
        u.replace("-I.jpg", "-O.jpg").replace("-I.webp", "-O.webp").replace("-I.png", "-O.png")
    })
}

/// ¿La publicación es de Mercado Libre FULL (fulfillment)? Ese stock lo gestiona
/// ML y el vendedor NO puede despacharlo desde su bodega, así que no sirve para
/// JarStore/WordPress.
fn es_full(p: &Producto) -> bool {
    p.datos
        .get("shipping")
        .and_then(|s| s.get("logistic_type"))
        .and_then(|l| l.as_str())
        .map(|t| t.eq_ignore_ascii_case("fulfillment"))
        .unwrap_or(false)
}

/// Un producto de catálogo puede tener 2 publicaciones sincronizadas (una en FULL
/// y otra en depósito propio) con el mismo `catalog_product_id`. Al distribuir
/// dejamos **una sola**, con el stock de la **bodega** (FULL no se suma ni se usa).
/// El representante es ESTABLE (prefiere la de bodega/no-FULL, luego la de
/// catálogo, luego por `ml_item_id`) para que re-sincronizar elija siempre la
/// misma publicación y no se dupliquen productos.
fn dedup_catalogo(items: Vec<Producto>) -> Vec<Producto> {
    use std::collections::HashMap;
    let mut idx_por_catalogo: HashMap<String, usize> = HashMap::new();
    let mut out: Vec<Producto> = Vec::with_capacity(items.len());
    for p in items {
        let cat = p
            .datos
            .get("catalog_product_id")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
        match cat {
            Some(cid) => match idx_por_catalogo.get(&cid).copied() {
                Some(i) => {
                    if prefiere(&p, &out[i]) {
                        out[i] = p; // mejor representante; NO se suma el stock
                    }
                }
                None => {
                    idx_por_catalogo.insert(cid, out.len());
                    out.push(p);
                }
            },
            None => out.push(p),
        }
    }
    out
}

fn prefiere(nuevo: &Producto, actual: &Producto) -> bool {
    // 1) la publicación de bodega (no-FULL) manda sobre la de FULL
    match (es_full(nuevo), es_full(actual)) {
        (false, true) => return true,
        (true, false) => return false,
        _ => {}
    }
    // 2) luego la de catálogo
    let es_catalogo = |p: &Producto| p.datos.get("catalog_listing").and_then(|v| v.as_bool()).unwrap_or(false);
    match (es_catalogo(nuevo), es_catalogo(actual)) {
        (true, false) => true,
        (false, true) => false,
        // 3) desempate determinista (estable entre syncs)
        _ => nuevo.ml_item_id < actual.ml_item_id,
    }
}

fn precio_canal(p: &Producto, canal: &str) -> i64 {
    match canal {
        "mercadolibre" => p.precio,
        _ => p.precio_jarstore.unwrap_or(p.precio), // jarstore: precio del canal, si no hay cae al de ML
    }
}

/// Productos del vendedor dueño de la key (acotados a su cuenta si la key lo indica).
pub fn productos(conn: &mut PgConnection, key: &ApiKey) -> ApiResult<Vec<PublicProducto>> {
    use crate::schema::ml_cuentas;
    let mut q = productos::table.inner_join(ml_cuentas::table).into_boxed();
    q = q.filter(ml_cuentas::vendedor_id.eq(key.vendedor_id));
    if let Some(cid) = key.ml_cuenta_id {
        q = q.filter(productos::ml_cuenta_id.eq(cid));
    }
    let items: Vec<Producto> = q.select(Producto::as_select()).order(productos::titulo.asc()).load(conn)?;
    let items = dedup_catalogo(items);
    Ok(items
        .into_iter()
        .map(|p| {
            let precio = precio_canal(&p, &key.canal);
            let (descripcion, especificaciones) = ficha(&p.datos);
            let categoria = p
                .datos
                .get("categoria_nombre")
                .and_then(|c| c.as_str())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());
            PublicProducto {
                precio,
                id: p.id,
                ml_item_id: p.ml_item_id,
                titulo: p.titulo,
                moneda: p.moneda,
                stock: p.stock,
                estado: p.estado,
                thumbnail: mejor_imagen(p.thumbnail),
                canal: key.canal.clone(),
                descripcion,
                especificaciones,
                categoria,
            }
        })
        .collect())
}

/// Resultado de una venta: nuevo stock y, si está vinculado, datos para empujar a ML.
pub struct VentaAplicada {
    pub nuevo_stock: i32,
    pub ml_cuenta_id: Uuid,
    pub ml_item_id: String,
    /// Si la publicación usa stock por ubicación (FULL/depósito), su user-product;
    /// se usa para descontar SOLO el depósito en ML.
    pub user_product_id: Option<String>,
}

/// Descuenta stock por una venta en una plataforma externa (transacción con bloqueo).
pub fn reportar_venta(conn: &mut PgConnection, key: &ApiKey, producto_id: Uuid, cantidad: i32) -> ApiResult<VentaAplicada> {
    if cantidad <= 0 {
        return Err(ApiError::bad_request("La cantidad debe ser mayor que 0"));
    }
    conn.transaction(|conn| {
        let p: Producto = productos::table
            .find(producto_id)
            .select(Producto::as_select())
            .for_update()
            .first(conn)
            .optional()?
            .or_not_found("Producto")?;
        // Verificar que el producto sea de este vendedor (y de la cuenta de la key si aplica).
        use crate::schema::ml_cuentas;
        let vend: Uuid = ml_cuentas::table.find(p.ml_cuenta_id).select(ml_cuentas::vendedor_id).first(conn)?;
        if vend != key.vendedor_id || key.ml_cuenta_id.is_some_and(|c| c != p.ml_cuenta_id) {
            return Err(ApiError::forbidden("Ese producto no corresponde a esta API key"));
        }
        if p.stock < cantidad {
            return Err(ApiError::bad_request(format!("Stock insuficiente: disponible {}, solicitado {cantidad}", p.stock)));
        }
        let nuevo = p.stock - cantidad;
        diesel::update(productos::table.find(producto_id))
            .set((productos::stock.eq(nuevo), productos::sincronizado_at.eq(now())))
            .execute(conn)?;
        crate::services::producto_services::registrar_cambio(conn, producto_id, "stock", Some(p.stock.to_string()), nuevo.to_string(), "venta");
        let user_product_id = p
            .datos
            .get("user_product_id")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
        Ok(VentaAplicada { nuevo_stock: nuevo, ml_cuenta_id: p.ml_cuenta_id, ml_item_id: p.ml_item_id, user_product_id })
    })
}

/// Devuelve stock a un producto (cancelación/devolución en la plataforma): lo SUMA y lo
/// deja listo para empujarlo de vuelta a Mercado Libre. Mismas validaciones de pertenencia.
pub fn reportar_devolucion(conn: &mut PgConnection, key: &ApiKey, producto_id: Uuid, cantidad: i32) -> ApiResult<VentaAplicada> {
    if cantidad <= 0 {
        return Err(ApiError::bad_request("La cantidad debe ser mayor que 0"));
    }
    conn.transaction(|conn| {
        let p: Producto = productos::table
            .find(producto_id)
            .select(Producto::as_select())
            .for_update()
            .first(conn)
            .optional()?
            .or_not_found("Producto")?;
        use crate::schema::ml_cuentas;
        let vend: Uuid = ml_cuentas::table.find(p.ml_cuenta_id).select(ml_cuentas::vendedor_id).first(conn)?;
        if vend != key.vendedor_id || key.ml_cuenta_id.is_some_and(|c| c != p.ml_cuenta_id) {
            return Err(ApiError::forbidden("Ese producto no corresponde a esta API key"));
        }
        let nuevo = p.stock.saturating_add(cantidad);
        diesel::update(productos::table.find(producto_id))
            .set((productos::stock.eq(nuevo), productos::sincronizado_at.eq(now())))
            .execute(conn)?;
        crate::services::producto_services::registrar_cambio(conn, producto_id, "stock", Some(p.stock.to_string()), nuevo.to_string(), "devolucion");
        let user_product_id = p
            .datos
            .get("user_product_id")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
        Ok(VentaAplicada { nuevo_stock: nuevo, ml_cuenta_id: p.ml_cuenta_id, ml_item_id: p.ml_item_id, user_product_id })
    })
}
