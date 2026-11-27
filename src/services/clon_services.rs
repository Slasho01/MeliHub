//! Clona publicaciones de una cuenta de Mercado Libre a otra: arma el payload de
//! `POST /items` a partir de un producto ya sincronizado. El controlador hace las
//! llamadas a ML (crear + descripción) por cada producto.

use crate::models::Producto;
use serde_json::{json, Map, Value};

/// Atributos que NO se deben enviar al crear: de solo lectura/sistema (ML los asigna),
/// dimensiones de empaque/medidas (dan errores de formato) y el formato de venta
/// (crea requisitos condicionales como "Cantidad de envases").
fn attr_bloqueado(id: &str) -> bool {
    id.starts_with("PACKAGE_")
        || id.ends_with("_DATA_SOURCE")
        || matches!(
            id,
            "SYI_PYMES_ID"
                | "ITEM_CONDITION"
                | "SALE_FORMAT"
                | "UNITS_PER_PACK"
                | "LENGTH"
                | "WIDTH"
                | "HEIGHT"
                | "DEPTH"
                | "WEIGHT"
                | "DIAMETER"
                | "VOLUME_CAPACITY"
                | "GTIN"
                | "EMPTY_GTIN_REASON"
        )
}

/// Construye el payload de `POST /items` para recrear un Producto del hub en otra cuenta.
pub fn payload(p: &Producto, con_familia: bool) -> Value {
    payload_core(&p.datos, &p.titulo, p.precio, &p.moneda, p.stock, p.categoria_ml.as_deref(), con_familia)
}

/// Núcleo: arma el cuerpo de `POST /items` desde datos crudos (del hub o de un link de ML).
/// La descripción se envía aparte. `con_familia`: categorías de catálogo exigen `family_name`
/// (ML genera el título); se intenta primero con título y se reintenta con familia si ML lo pide.
pub fn payload_core(datos: &Value, titulo: &str, precio: i64, moneda: &str, stock: i32, categoria_ml: Option<&str>, con_familia: bool) -> Value {
    let d = datos;

    // Fotos: ML las reingesta por URL de origen.
    let pics: Vec<Value> = d
        .get("pictures")
        .and_then(|a| a.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|x| x.get("secure_url").or_else(|| x.get("url")).and_then(|u| u.as_str()))
                .map(|u| json!({ "source": u }))
                .collect()
        })
        .unwrap_or_default();

    // Condición (desde ITEM_CONDITION) y atributos (sin ITEM_CONDITION, que va como `condition`).
    let mut condition = "new".to_string();
    let attrs: Vec<Value> = d
        .get("attributes")
        .and_then(|a| a.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|a| {
                    let id = a.get("id").and_then(|v| v.as_str())?;
                    if id == "ITEM_CONDITION" {
                        if let Some(vn) = a.get("value_name").and_then(|v| v.as_str()) {
                            condition = match vn.to_lowercase().as_str() {
                                "usado" | "used" => "used",
                                "reacondicionado" | "refurbished" => "refurbished",
                                _ => "new",
                            }
                            .to_string();
                        }
                        return None;
                    }
                    // Atributos de solo lectura/sistema o de medida: ML los rechaza al crear.
                    if attr_bloqueado(id) {
                        return None;
                    }
                    // Medidas/números (value_struct) → formato distinto; se omiten para evitar errores.
                    if a.get("value_struct").map(|v| !v.is_null()).unwrap_or(false) {
                        return None;
                    }
                    // Enviar value_id Y value_name juntos: si ML no resuelve el id, usa el nombre.
                    let vid = a.get("value_id").and_then(|v| v.as_str()).filter(|s| !s.is_empty());
                    let vn = a.get("value_name").and_then(|v| v.as_str()).filter(|s| !s.is_empty());
                    if vid.is_none() && vn.is_none() {
                        return None; // atributo sin valor → no sirve al crear
                    }
                    let mut o = Map::new();
                    o.insert("id".into(), json!(id));
                    if let Some(v) = vid {
                        o.insert("value_id".into(), json!(v));
                    }
                    if let Some(v) = vn {
                        o.insert("value_name".into(), json!(v));
                    }
                    Some(Value::Object(o))
                })
                .collect()
        })
        .unwrap_or_default();

    let listing_type = d
        .get("listing_type_id")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("gold_special");

    let mut body = json!({
        "category_id": categoria_ml,
        "price": precio,
        "currency_id": moneda,
        "buying_mode": "buy_it_now",
        "listing_type_id": listing_type,
        "condition": condition,
        "pictures": pics,
        "attributes": attrs,
        // Envío normal (Mercado Envíos), SIN envío gratis (lo activa el vendedor luego si
        // le conviene: forzarlo en productos baratos falla por "costo supera la venta").
        // No se copia FULL: el stock no está en la bodega destino.
        "shipping": { "mode": "me2", "local_pick_up": false, "free_shipping": false },
    });

    // Modo normal: va el título. Modo familia (catálogo): va family_name y ML genera el título.
    if con_familia {
        body["family_name"] = json!(titulo.chars().take(60).collect::<String>());
    } else {
        body["title"] = json!(titulo);
    }

    // Variaciones: se copian las combinaciones y cantidades (sin picture_ids, que son del origen).
    let variations = d.get("variations").and_then(|v| v.as_array()).filter(|a| !a.is_empty());
    if let Some(vars) = variations {
        let vout: Vec<Value> = vars
            .iter()
            .map(|v| {
                let mut o = Map::new();
                if let Some(ac) = v.get("attribute_combinations") {
                    o.insert("attribute_combinations".into(), ac.clone());
                }
                let q = v.get("available_quantity").and_then(|x| x.as_i64()).unwrap_or(0).max(0);
                o.insert("available_quantity".into(), json!(q));
                if let Some(pr) = v.get("price").and_then(|x| x.as_f64()) {
                    o.insert("price".into(), json!(pr));
                }
                if let Some(va) = v.get("attributes") {
                    o.insert("attributes".into(), va.clone());
                }
                Value::Object(o)
            })
            .collect();
        body["variations"] = json!(vout);
    } else {
        body["available_quantity"] = json!(stock.max(1));
    }

    body
}
