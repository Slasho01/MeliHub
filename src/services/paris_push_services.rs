//! Construye los cuerpos JSON para el Marketplace de Cencosud (Paris) a partir de datos
//! del hub. Los atributos son específicos de cada familia (se obtienen con
//! `GET /v1/attribute/family/{id}`), por eso llegan como pares (code, value).

use serde_json::{json, Value};

/// Bloque de precios: precio de lista (+ oferta opcional). Tienda "default".
pub fn build_precio(precio: i64, oferta: Option<i64>) -> Value {
    let mut arr = vec![json!({
        "value": precio.max(0),
        "store": { "code": "default" },
        "price": { "code": "clp-list-prices" }
    })];
    if let Some(of) = oferta {
        if of > 0 {
            arr.push(json!({
                "value": of,
                "store": { "code": "default" },
                "price": { "code": "clp-offer-prices" }
            }));
        }
    }
    Value::Array(arr)
}

/// Stock por SKU del seller (nuestro SKU): `{ "skus": [ { sku_seller, quantity } ] }`.
pub fn build_stock(items: &[(String, i32)]) -> Value {
    let skus: Vec<Value> = items
        .iter()
        .filter(|(s, _)| !s.trim().is_empty())
        .map(|(sku, qty)| json!({ "sku_seller": sku, "quantity": (*qty).max(0) }))
        .collect();
    json!({ "skus": skus })
}

/// Atributos normalizados a `[{ value, attribute: { code } }]` (ignora vacíos).
fn attrs_json(atributos: &[(String, String)]) -> Value {
    Value::Array(
        atributos
            .iter()
            .filter(|(c, v)| !c.trim().is_empty() && !v.trim().is_empty())
            .map(|(c, v)| json!({ "value": v, "attribute": { "code": c } }))
            .collect(),
    )
}

/// Cuerpo para crear un producto. `ref_product` = nuestro SKU/identificador estable.
pub fn build_product(name: &str, ref_product: &str, family_code: &str, precio: i64, oferta: Option<i64>, atributos: &[(String, String)]) -> Value {
    json!({
        "name": name,
        "refProduct": ref_product,
        "family": { "code": family_code },
        "productPrices": build_precio(precio, oferta),
        "attributeDetails": attrs_json(atributos),
    })
}

/// Cuerpo para crear una variante colgando de `product_sku` (el SKU de Paris del padre).
pub fn build_variante(name: &str, product_sku: &str, ref_variant: &str, atributos: &[(String, String)]) -> Value {
    json!({
        "name": name,
        "productSku": product_sku,
        "refVariant": ref_variant,
        "attributeDetails": attrs_json(atributos),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn precio_lista_y_oferta() {
        let v = build_precio(29990, Some(24990));
        let arr = v.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["value"], 29990);
        assert_eq!(arr[0]["price"]["code"], "clp-list-prices");
        assert_eq!(arr[1]["price"]["code"], "clp-offer-prices");
    }

    #[test]
    fn precio_sin_oferta_negativo_cae_a_cero() {
        let v = build_precio(-5, None);
        assert_eq!(v.as_array().unwrap().len(), 1);
        assert_eq!(v[0]["value"], 0);
    }

    #[test]
    fn stock_por_sku_seller() {
        let v = build_stock(&[("ABC-1".into(), 3), ("".into(), 9), ("ABC-2".into(), -1)]);
        let skus = v["skus"].as_array().unwrap();
        assert_eq!(skus.len(), 2); // el de sku vacío se filtra
        assert_eq!(skus[0]["sku_seller"], "ABC-1");
        assert_eq!(skus[1]["quantity"], 0); // negativo → 0
    }

    #[test]
    fn producto_con_atributos() {
        let v = build_product("Polera", "MLC123", "fam-poleras", 9990, None, &[("color".into(), "Rojo".into()), ("".into(), "x".into())]);
        assert_eq!(v["name"], "Polera");
        assert_eq!(v["refProduct"], "MLC123");
        assert_eq!(v["family"]["code"], "fam-poleras");
        let attrs = v["attributeDetails"].as_array().unwrap();
        assert_eq!(attrs.len(), 1); // el de code vacío se filtra
        assert_eq!(attrs[0]["attribute"]["code"], "color");
        assert_eq!(attrs[0]["value"], "Rojo");
    }
}
