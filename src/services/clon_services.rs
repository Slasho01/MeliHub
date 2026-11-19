//! Clona publicaciones de una cuenta de Mercado Libre a otra: arma el payload de
//! `POST /items` a partir de un producto ya sincronizado. El controlador hace las
//! llamadas a ML (crear + descripción) por cada producto.

use crate::models::Producto;
use serde_json::{json, Map, Value};

/// Construye el cuerpo de `POST /items` para recrear `p` en otra cuenta.
/// La descripción se envía aparte (ver `datos.descripcion`).
pub fn payload(p: &Producto) -> Value {
    let d = &p.datos;

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
                    let mut o = Map::new();
                    o.insert("id".into(), json!(id));
                    if let Some(vid) = a.get("value_id").and_then(|v| v.as_str()) {
                        o.insert("value_id".into(), json!(vid));
                    } else if let Some(vn) = a.get("value_name").and_then(|v| v.as_str()).filter(|s| !s.is_empty()) {
                        o.insert("value_name".into(), json!(vn));
                    } else {
                        return None; // atributo sin valor → no sirve al crear
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
    let free_shipping = d
        .get("shipping")
        .and_then(|s| s.get("free_shipping"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let mut body = json!({
        "title": p.titulo,
        "category_id": p.categoria_ml,
        "price": p.precio,
        "currency_id": p.moneda,
        "buying_mode": "buy_it_now",
        "listing_type_id": listing_type,
        "condition": condition,
        "pictures": pics,
        "attributes": attrs,
        // Envío normal (Mercado Envíos). No se copia FULL: el stock no está en la bodega destino.
        "shipping": { "mode": "me2", "local_pick_up": false, "free_shipping": free_shipping },
    });

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
        body["available_quantity"] = json!(p.stock.max(1));
    }

    body
}
