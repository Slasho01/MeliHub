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

/// Una variante (combinación) de la publicación, normalizada para cualquier plataforma.
#[derive(Serialize)]
pub struct VariantePublica {
    /// Id de la variación en Mercado Libre (para emparejar al re-sincronizar).
    pub id: i64,
    /// Atributos que definen la variante: [{nombre:"Color", valor:"Rojo"}, {nombre:"Talla", valor:"M"}].
    pub atributos: Vec<Especificacion>,
    pub precio: i64,
    pub stock: i32,
    /// Imágenes propias de la variante (ya resueltas a URL, alta calidad).
    pub imagenes: Vec<String>,
}

/// Producto tal como lo ve una plataforma externa (precio del canal de la key).
#[derive(Serialize)]
pub struct PublicProducto {
    pub id: Uuid,
    /// SKU estable para los canales (catalog_product_id si existe, si no ml_item_id).
    pub sku: String,
    pub ml_item_id: String,
    pub titulo: String,
    pub precio: i64,
    pub moneda: String,
    pub stock: i32,
    pub estado: String,
    pub thumbnail: Option<String>,
    /// TODAS las imágenes de la publicación (galería), alta calidad y https. La primera
    /// equivale al thumbnail. Cada plataforma decide cuántas usar.
    pub imagenes: Vec<String>,
    /// URL del video (YouTube) si la publicación tiene.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video: Option<String>,
    pub canal: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub descripcion: Option<String>,
    pub especificaciones: Vec<Especificacion>,
    /// Variantes (color/talla/etc.) con su propio stock, precio e imágenes.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub variantes: Vec<VariantePublica>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub categoria: Option<String>,
    /// Ruta completa de categorías (raíz → hoja) para armar jerarquía/submenús en el canal.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub categoria_ruta: Vec<String>,
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

/// Mercado Libre sirve variantes por tamaño vía el sufijo final (`-I` 90px, `-O` 500px,
/// `-W` 568px, `-F` 755px+ = la más grande). Forzamos https y la variante **`-F` en `.jpg`**
/// para traer la mayor resolución en todos los canales (WooCommerce, Paris, etc.).
fn mejor_imagen(url: Option<String>) -> Option<String> {
    url.map(|u| {
        let u = if let Some(r) = u.strip_prefix("http://") { format!("https://{r}") } else { u };
        if u.contains("mlstatic.com") {
            // Reemplaza el sufijo de tamaño del final (`-X.ext`) por `-F.jpg`, dejando intacto
            // el resto (incluido un posible prefijo de zoom `2X`). El guard de largo evita
            // tocar URLs sin sufijo de tamaño.
            if let Some(pos) = u.rfind('-') {
                let tail = &u[pos + 1..];
                if tail.len() <= 6 && tail.contains('.') {
                    return format!("{}-F.jpg", &u[..pos]);
                }
            }
        }
        u.replace(".webp", ".jpg")
    })
}

/// Todas las imágenes de la publicación (desde `datos.pictures`), alta calidad y https,
/// sin duplicados. Si no hay galería, cae al thumbnail.
fn galeria(datos: &serde_json::Value, thumb: &Option<String>) -> Vec<String> {
    let mut urls: Vec<String> = Vec::new();
    if let Some(arr) = datos.get("pictures").and_then(|p| p.as_array()) {
        for pic in arr {
            let u = pic
                .get("secure_url")
                .and_then(|v| v.as_str())
                .or_else(|| pic.get("url").and_then(|v| v.as_str()));
            if let Some(u) = mejor_imagen(u.map(|s| s.to_string())) {
                if !u.is_empty() && !urls.contains(&u) {
                    urls.push(u);
                }
            }
        }
    }
    if urls.is_empty() {
        if let Some(t) = mejor_imagen(thumb.clone()) {
            urls.push(t);
        }
    }
    urls
}

/// Mapa `picture_id -> url` para resolver las imágenes de cada variante.
fn mapa_imagenes(datos: &serde_json::Value) -> std::collections::HashMap<String, String> {
    let mut m = std::collections::HashMap::new();
    if let Some(arr) = datos.get("pictures").and_then(|p| p.as_array()) {
        for pic in arr {
            let id = pic.get("id").and_then(|v| v.as_str());
            let url = pic
                .get("secure_url")
                .and_then(|v| v.as_str())
                .or_else(|| pic.get("url").and_then(|v| v.as_str()));
            if let (Some(id), Some(url)) = (id, mejor_imagen(url.map(|s| s.to_string()))) {
                m.insert(id.to_string(), url);
            }
        }
    }
    m
}

/// URL del video de la publicación (ML guarda el id de YouTube en `video_id`).
fn video_de(datos: &serde_json::Value) -> Option<String> {
    datos
        .get("video_id")
        .and_then(|v| v.as_str())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|id| format!("https://www.youtube.com/watch?v={id}"))
}

/// Normaliza las variantes de ML (`productos.variantes`) a `VariantePublica`.
fn variantes_de(p: &Producto, precio_base: i64, imgs: &std::collections::HashMap<String, String>) -> Vec<VariantePublica> {
    let Some(arr) = p.variantes.as_array() else { return Vec::new() };
    arr.iter()
        .filter_map(|v| {
            let id = v.get("id").and_then(|x| x.as_i64())?;
            let atributos = v
                .get("attribute_combinations")
                .and_then(|a| a.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|a| {
                            let nombre = a.get("name").and_then(|x| x.as_str()).unwrap_or("").trim().to_string();
                            let valor = a.get("value_name").and_then(|x| x.as_str()).unwrap_or("").trim().to_string();
                            (!nombre.is_empty() && !valor.is_empty()).then_some(Especificacion { nombre, valor })
                        })
                        .collect()
                })
                .unwrap_or_default();
            let precio = v
                .get("price")
                .and_then(|x| x.as_f64())
                .map(|f| f.round() as i64)
                .filter(|&x| x > 0)
                .unwrap_or(precio_base);
            let stock = v.get("available_quantity").and_then(|x| x.as_i64()).unwrap_or(0).max(0) as i32;
            let imagenes = v
                .get("picture_ids")
                .and_then(|a| a.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|pid| pid.as_str())
                        .filter_map(|pid| imgs.get(pid).cloned())
                        .filter(|s| !s.is_empty())
                        .collect()
                })
                .unwrap_or_default();
            Some(VariantePublica { id, atributos, precio, stock, imagenes })
        })
        .collect()
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
    // Solo publicaciones ACTIVAS en ML: los pausados/cerrados no se distribuyen (la
    // mayoría son sin stock). Si se pausa uno después, sale del feed y, con "ocultar
    // retirados", el canal lo oculta solo.
    q = q.filter(productos::estado.eq("active"));
    if let Some(cid) = key.ml_cuenta_id {
        q = q.filter(productos::ml_cuenta_id.eq(cid));
    }
    let items: Vec<Producto> = q.select(Producto::as_select()).order(productos::titulo.asc()).load(conn)?;
    let items = dedup_catalogo(items);
    Ok(items
        .into_iter()
        .filter(|p| p.stock > 0) // solo se distribuyen productos CON stock
        .map(|p| {
            let precio = precio_canal(&p, &key.canal);
            let (descripcion, especificaciones) = ficha(&p.datos);
            let categoria = p
                .datos
                .get("categoria_nombre")
                .and_then(|c| c.as_str())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());
            let categoria_ruta = p
                .datos
                .get("categoria_ruta")
                .and_then(|r| r.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|x| x.as_str())
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let imagenes = galeria(&p.datos, &p.thumbnail);
            let video = video_de(&p.datos);
            let variantes = variantes_de(&p, precio, &mapa_imagenes(&p.datos));
            // SKU ESTABLE: el id de producto de catálogo (compartido por la publicación de
            // catálogo y la normal) si existe; si no, el item id. Evita duplicados cuando el
            // representante salta entre la publicación normal y la de catálogo.
            let sku = p
                .datos
                .get("catalog_product_id")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .unwrap_or_else(|| p.ml_item_id.clone());
            PublicProducto {
                precio,
                id: p.id,
                sku,
                ml_item_id: p.ml_item_id,
                titulo: p.titulo,
                moneda: p.moneda,
                stock: p.stock,
                estado: p.estado,
                thumbnail: mejor_imagen(p.thumbnail),
                imagenes,
                video,
                canal: key.canal.clone(),
                descripcion,
                especificaciones,
                variantes,
                categoria,
                categoria_ruta,
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
/// Descuenta stock por una venta EXTERNA (p. ej. una orden de Falabella) y deja la info
/// para empujar el nuevo stock a Mercado Libre. Uso interno (sin API key). Nunca baja de 0.
pub fn aplicar_venta_externa(conn: &mut PgConnection, producto_id: Uuid, cantidad: i32, motivo: &str) -> ApiResult<VentaAplicada> {
    conn.transaction(|conn| {
        let p: Producto = productos::table
            .find(producto_id)
            .select(Producto::as_select())
            .for_update()
            .first(conn)
            .optional()?
            .or_not_found("Producto")?;
        let nuevo = (p.stock - cantidad.max(0)).max(0);
        diesel::update(productos::table.find(producto_id))
            .set((productos::stock.eq(nuevo), productos::sincronizado_at.eq(now())))
            .execute(conn)?;
        crate::services::producto_services::registrar_cambio(conn, producto_id, "stock", Some(p.stock.to_string()), nuevo.to_string(), motivo);
        let user_product_id = p
            .datos
            .get("user_product_id")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
        Ok(VentaAplicada { nuevo_stock: nuevo, ml_cuenta_id: p.ml_cuenta_id, ml_item_id: p.ml_item_id, user_product_id })
    })
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mejor_imagen_maximiza_a_f() {
        let f = |s: &str| mejor_imagen(Some(s.to_string())).unwrap();
        // thumbnail -I (90px) -> -F (grande)
        assert_eq!(f("https://http2.mlstatic.com/D_971195-MLC109_042026-I.webp"), "https://http2.mlstatic.com/D_971195-MLC109_042026-F.jpg");
        // -O (500px) -> -F, y http -> https
        assert_eq!(f("http://http2.mlstatic.com/D_123-O.jpg"), "https://http2.mlstatic.com/D_123-F.jpg");
        // ya en -F: se mantiene (solo normaliza extensión)
        assert_eq!(f("https://http2.mlstatic.com/D_123-F.webp"), "https://http2.mlstatic.com/D_123-F.jpg");
        // conserva prefijo de zoom 2X
        assert_eq!(f("https://http2.mlstatic.com/D_NQ_NP_2X_777-O.webp"), "https://http2.mlstatic.com/D_NQ_NP_2X_777-F.jpg");
        // url sin sufijo de tamaño: no se rompe (solo webp->jpg)
        assert_eq!(f("https://otro.com/foto.webp"), "https://otro.com/foto.jpg");
    }
}
