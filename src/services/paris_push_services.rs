//! Construye los cuerpos JSON para el Marketplace de Cencosud (Paris), API **v2**.
//! Estructura de `POST /v2/products`: `{ product:{name,sellerSku,familyId,category,attributes:[{id,value}]},
//! variants:[{skuSeller,attributes,medias:[{position,src,name}],quantity}], prices:[{value,storePrice}] }`.

use serde_json::{json, Value};

/// Precios v2: `[{ value, storePrice, type }]`. `store_price_id` = id de `/v2/price-types`;
/// `tipo` = su nombre (Paris exige `type` como texto).
pub fn build_precio(precio: i64, store_price_id: Option<&str>, tipo: Option<&str>) -> Value {
    match store_price_id {
        Some(sp) if !sp.is_empty() => json!([{ "value": precio.max(0), "storePrice": sp, "type": tipo.unwrap_or("Normal") }]),
        _ => json!([]),
    }
}

/// Stock v2: `{ "skus": [ { sku, quantity } ] }` (sku = SKU del seller).
pub fn build_stock(items: &[(String, i32)]) -> Value {
    let skus: Vec<Value> = items
        .iter()
        .filter(|(s, _)| !s.trim().is_empty())
        .map(|(sku, qty)| json!({ "sku": sku, "quantity": (*qty).max(0) }))
        .collect();
    json!({ "skus": skus })
}

/// Atributos v2: `[{ id, value }]` (id = id del atributo de la familia).
fn attrs_json(atributos: &[(String, String)]) -> Value {
    Value::Array(
        atributos
            .iter()
            .filter(|(id, v)| !id.trim().is_empty() && !v.trim().is_empty())
            .map(|(id, v)| json!({ "id": id, "value": v }))
            .collect(),
    )
}

/// Imágenes v2 (van dentro de la variante): `[{ position, src, name }]`.
fn medias_json(imagenes: &[String]) -> Value {
    Value::Array(
        imagenes
            .iter()
            .filter(|u| !u.trim().is_empty())
            .enumerate()
            .map(|(i, url)| json!({ "position": i + 1, "src": url, "name": format!("img-{}", i + 1) }))
            .collect(),
    )
}

/// Cuerpo completo para `POST /v2/products`.
#[allow(clippy::too_many_arguments)]
pub fn build_product(name: &str, seller_sku: &str, family_id: &str, categoria_id: &str, precio: i64, stock: i32, atributos: &[(String, String)], imagenes: &[String], store_price_id: Option<&str>, tipo_precio: Option<&str>) -> Value {
    json!({
        "product": {
            "name": name,
            "sellerSku": seller_sku,
            "familyId": family_id,
            "category": categoria_id,
            "attributes": attrs_json(atributos),
        },
        "variants": [ {
            "skuSeller": seller_sku,
            // La variante exige ≥1 atributo; reusamos los del producto.
            "attributes": attrs_json(atributos),
            "medias": medias_json(imagenes),
            "quantity": stock.max(0),
        } ],
        "prices": build_precio(precio, store_price_id, tipo_precio),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stock_v2_usa_sku() {
        let v = build_stock(&[("ABC-1".into(), 3), ("".into(), 9), ("ABC-2".into(), -1)]);
        let skus = v["skus"].as_array().unwrap();
        assert_eq!(skus.len(), 2);
        assert_eq!(skus[0]["sku"], "ABC-1");
        assert_eq!(skus[1]["quantity"], 0);
    }

    #[test]
    fn precio_con_store_price() {
        let v = build_precio(9990, Some("sp-1"), Some("Normal"));
        assert_eq!(v[0]["value"], 9990);
        assert_eq!(v[0]["storePrice"], "sp-1");
        assert_eq!(v[0]["type"], "Normal");
        assert_eq!(build_precio(9990, None, None).as_array().unwrap().len(), 0);
    }

    #[test]
    fn producto_v2_completo() {
        let v = build_product(
            "Polera", "MLC123", "fam-1", "cat-1", 9990, 5,
            &[("attr-color".into(), "Rojo".into()), ("".into(), "x".into())],
            &["https://img/1.jpg".into(), "".into()],
            Some("sp-1"), Some("Normal"),
        );
        // la variante también lleva atributos (Paris lo exige)
        assert_eq!(v["variants"][0]["attributes"].as_array().unwrap().len(), 1);
        assert_eq!(v["product"]["name"], "Polera");
        assert_eq!(v["product"]["sellerSku"], "MLC123");
        assert_eq!(v["product"]["familyId"], "fam-1");
        assert_eq!(v["product"]["category"], "cat-1");
        // atributos por id (el de id vacío se filtra)
        let attrs = v["product"]["attributes"].as_array().unwrap();
        assert_eq!(attrs.len(), 1);
        assert_eq!(attrs[0]["id"], "attr-color");
        assert_eq!(attrs[0]["value"], "Rojo");
        // variante con media + quantity
        assert_eq!(v["variants"][0]["skuSeller"], "MLC123");
        assert_eq!(v["variants"][0]["quantity"], 5);
        assert_eq!(v["variants"][0]["medias"].as_array().unwrap().len(), 1);
        assert_eq!(v["variants"][0]["medias"][0]["src"], "https://img/1.jpg");
        // precio
        assert_eq!(v["prices"][0]["value"], 9990);
        assert_eq!(v["prices"][0]["storePrice"], "sp-1");
    }
}
