//! Publicación de productos del hub hacia Falabella (ProductCreate/Update).
//! Arma el XML del feed a partir de un `Producto` del catálogo + la categoría y los
//! atributos obligatorios que correspondan (vienen del mapeo por categoría).

use crate::models::Producto;

/// Escapa texto para contenido XML.
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Normaliza el nombre de un atributo a un nombre de elemento XML válido
/// (letras/números/guion-bajo; no puede empezar con número).
fn xml_tag(s: &str) -> String {
    let mut t: String = s.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect();
    if t.is_empty() || t.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        t.insert(0, '_');
    }
    t
}

/// Imágenes (URLs) del producto desde `datos.pictures` (alta calidad), con fallback al thumbnail.
fn imagenes_de(p: &Producto) -> Vec<String> {
    let mut urls: Vec<String> = Vec::new();
    if let Some(arr) = p.datos.get("pictures").and_then(|a| a.as_array()) {
        for pic in arr {
            if let Some(u) = pic
                .get("secure_url")
                .and_then(|u| u.as_str())
                .or_else(|| pic.get("url").and_then(|u| u.as_str()))
            {
                let u = u.replace("-I.jpg", "-O.jpg").replace("http://", "https://");
                if !u.is_empty() && !urls.contains(&u) {
                    urls.push(u);
                }
            }
        }
    }
    if urls.is_empty() {
        if let Some(t) = p.thumbnail.as_deref().filter(|s| !s.trim().is_empty()) {
            urls.push(t.replace("http://", "https://"));
        }
    }
    urls
}

/// Construye el XML de un `<Product>` para ProductCreate/Update.
#[allow(clippy::too_many_arguments)]
fn product_xml(sku: &str, name: &str, desc: &str, price: i64, qty: i32, category_id: &str, brand: &str, atributos: &[(String, String)], imagenes: &[String]) -> String {
    let mut data = String::from("<ConditionType>Nuevo</ConditionType>");
    for (k, v) in atributos {
        let v = v.trim();
        if v.is_empty() {
            continue;
        }
        let tag = xml_tag(k);
        data.push_str(&format!("<{tag}>{}</{tag}>", xml_escape(v)));
    }
    let mut images = String::new();
    for u in imagenes {
        images.push_str(&format!("<Image>{}</Image>", xml_escape(u)));
    }
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<Request><Product>\
<SellerSku>{sku}</SellerSku>\
<Name>{name}</Name>\
<PrimaryCategory>{cat}</PrimaryCategory>\
<Description>{desc}</Description>\
<Brand>{brand}</Brand>\
<Price>{price}</Price>\
<ProductId>{sku}</ProductId>\
<Quantity>{qty}</Quantity>\
<ProductData>{data}</ProductData>\
<Images>{images}</Images>\
</Product></Request>",
        sku = xml_escape(sku),
        name = xml_escape(name),
        cat = xml_escape(category_id),
        desc = xml_escape(desc),
        brand = xml_escape(brand),
    )
}

/// XML de ProductCreate/Update para un producto del hub con su categoría/atributos Falabella.
pub fn build_product_xml(p: &Producto, category_id: &str, brand: &str, atributos: &[(String, String)]) -> String {
    let desc = p.datos.get("descripcion").and_then(|d| d.as_str()).unwrap_or(&p.titulo);
    product_xml(
        &p.ml_item_id,
        &p.titulo,
        desc,
        p.precio.max(0),
        p.stock.max(0),
        category_id,
        brand,
        atributos,
        &imagenes_de(p),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xml_bien_formado_y_escapado() {
        let atributos = vec![
            ("Material".to_string(), "Algodón & Lino".to_string()),
            ("2Garantia".to_string(), "12 meses".to_string()), // tag empieza con número
            ("Vacio".to_string(), "  ".to_string()),            // se omite
        ];
        let imgs = vec!["https://img/1-O.jpg".to_string(), "https://img/2-O.jpg".to_string()];
        let xml = product_xml("MLA123", "Polera <Azul>", "Linda & cómoda", 9990, 5, "CAT42", "Marca\"X\"", &atributos, &imgs);
        assert!(xml.contains("<SellerSku>MLA123</SellerSku>"));
        assert!(xml.contains("<Name>Polera &lt;Azul&gt;</Name>"));
        assert!(xml.contains("<Price>9990</Price>"));
        assert!(xml.contains("<Quantity>5</Quantity>"));
        assert!(xml.contains("<PrimaryCategory>CAT42</PrimaryCategory>"));
        assert!(xml.contains("<Brand>Marca&quot;X&quot;</Brand>"));
        assert!(xml.contains("<Material>Algodón &amp; Lino</Material>"));
        assert!(xml.contains("<_2Garantia>12 meses</_2Garantia>")); // número al inicio => _2Garantia
        assert!(!xml.contains("Vacio")); // atributo vacío omitido
        assert!(xml.contains("<Image>https://img/1-O.jpg</Image>"));
        assert!(xml.contains("<Description>Linda &amp; cómoda</Description>"));
    }
}
