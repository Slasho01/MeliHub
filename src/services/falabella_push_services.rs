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

/// Normaliza una URL de imagen de ML para Falabella: https, variante **`-F`** (grande;
/// Falabella exige ≥500×500 y `-O` deja el lado corto <500) y fuerza `.jpg` (NO webp).
fn img_falabella(u: &str) -> String {
    let u = u.replace("http://", "https://");
    // ML termina en "-{tamaño}.{ext}" (I/O/F/W/V/G/S). Forzamos "-F.jpg".
    if let Some(dot) = u.rfind('.') {
        let base = &u[..dot];
        if let Some(dash) = base.rfind('-') {
            if base.len() - dash == 2 {
                // sufijo de tamaño de 1 letra -> reemplazar por F
                return format!("{}-F.jpg", &base[..dash]);
            }
        }
        return format!("{base}.jpg");
    }
    u
}

/// Imágenes (URLs) del producto desde `datos.pictures`, máx 8 (límite de Falabella), con
/// fallback al thumbnail.
fn imagenes_de(p: &Producto) -> Vec<String> {
    let mut urls: Vec<String> = Vec::new();
    if let Some(arr) = p.datos.get("pictures").and_then(|a| a.as_array()) {
        for pic in arr {
            if let Some(u) = pic
                .get("secure_url")
                .and_then(|u| u.as_str())
                .or_else(|| pic.get("url").and_then(|u| u.as_str()))
            {
                let u = img_falabella(u);
                if !u.is_empty() && !urls.contains(&u) {
                    urls.push(u);
                }
            }
        }
    }
    if urls.is_empty() {
        if let Some(t) = p.thumbnail.as_deref().filter(|s| !s.trim().is_empty()) {
            urls.push(img_falabella(t));
        }
    }
    urls.truncate(8);
    urls
}

/// XML de la acción `Image` (asocia las imágenes a un producto ya creado). `None` si no hay.
pub fn build_image_xml(p: &Producto) -> Option<String> {
    let imagenes = imagenes_de(p);
    if imagenes.is_empty() {
        return None;
    }
    let mut images = String::new();
    for u in &imagenes {
        images.push_str(&format!("<Image>{}</Image>", xml_escape(u)));
    }
    Some(format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<Request><ProductImage>\
<SellerSku>{}</SellerSku>\
<Images>{}</Images>\
</ProductImage></Request>",
        xml_escape(&p.ml_item_id),
        images,
    ))
}

/// Construye el XML de un `<Product>` para ProductCreate/Update.
///
/// En la API nueva de Falabella el **precio y el stock van dentro de `<BusinessUnits>`**
/// (con `OperatorCode`: facl=Chile, fape=Perú, faco=Colombia), NO como `<Price>`/`<Quantity>`
/// sueltos. `Variation` es obligatorio aunque el producto no tenga variantes (agrupa fotos).
#[allow(clippy::too_many_arguments)]
fn product_xml(sku: &str, name: &str, desc: &str, price: i64, qty: i32, category_id: &str, brand: &str, variation: &str, operador: &str, atributos: &[(String, String)], imagenes: &[String]) -> String {
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
    let business = format!(
        "<BusinessUnits><BusinessUnit>\
<OperatorCode>{op}</OperatorCode>\
<Price>{price}</Price>\
<Stock>{qty}</Stock>\
<Status>active</Status>\
<IsPublished>1</IsPublished>\
</BusinessUnit></BusinessUnits>",
        op = xml_escape(operador),
    );
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<Request><Product>\
<SellerSku>{sku}</SellerSku>\
<Name>{name}</Name>\
<Variation>{variation}</Variation>\
<PrimaryCategory>{cat}</PrimaryCategory>\
<Description>{desc}</Description>\
<Brand>{brand}</Brand>\
<ProductData>{data}</ProductData>\
{business}\
<Images>{images}</Images>\
</Product></Request>",
        sku = xml_escape(sku),
        name = xml_escape(name),
        variation = xml_escape(variation),
        cat = xml_escape(category_id),
        desc = xml_escape(desc),
        brand = xml_escape(brand),
    )
}

/// XML de ProductCreate/Update para un producto del hub con su categoría/atributos Falabella.
/// `variation` vacío → "No aplica"; `operador` vacío → "facl" (Chile).
pub fn build_product_xml(p: &Producto, category_id: &str, brand: &str, variation: &str, operador: &str, atributos: &[(String, String)]) -> String {
    let desc = p.datos.get("descripcion").and_then(|d| d.as_str()).unwrap_or(&p.titulo);
    let variation = if variation.trim().is_empty() { "No aplica" } else { variation.trim() };
    let operador = if operador.trim().is_empty() { "facl" } else { operador.trim() };
    product_xml(
        &p.ml_item_id,
        &p.titulo,
        desc,
        p.precio.max(0),
        p.stock.max(0),
        category_id,
        brand,
        variation,
        operador,
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
        let xml = product_xml("MLA123", "Polera <Azul>", "Linda & cómoda", 9990, 5, "CAT42", "Marca\"X\"", "No aplica", "facl", &atributos, &imgs);
        assert!(xml.contains("<SellerSku>MLA123</SellerSku>"));
        assert!(xml.contains("<Name>Polera &lt;Azul&gt;</Name>"));
        assert!(xml.contains("<Variation>No aplica</Variation>"));
        // precio y stock van DENTRO de BusinessUnits (no sueltos)
        assert!(xml.contains("<BusinessUnits><BusinessUnit><OperatorCode>facl</OperatorCode><Price>9990</Price><Stock>5</Stock><Status>active</Status><IsPublished>1</IsPublished></BusinessUnit></BusinessUnits>"));
        assert!(!xml.contains("<Quantity>"));
        assert!(xml.contains("<PrimaryCategory>CAT42</PrimaryCategory>"));
        assert!(xml.contains("<Brand>Marca&quot;X&quot;</Brand>"));
        assert!(xml.contains("<Material>Algodón &amp; Lino</Material>"));
        assert!(xml.contains("<_2Garantia>12 meses</_2Garantia>")); // número al inicio => _2Garantia
        assert!(!xml.contains("Vacio")); // atributo vacío omitido
        assert!(xml.contains("<Image>https://img/1-O.jpg</Image>"));
        assert!(xml.contains("<Description>Linda &amp; cómoda</Description>"));
    }

    #[test]
    fn imagen_fuerza_variante_grande_jpg() {
        // -I.webp (thumbnail) -> -F.jpg (864x789, ≥500 en ambos lados)
        assert_eq!(
            img_falabella("http://http2.mlstatic.com/D_891743-MLC109938399813_042026-I.webp"),
            "https://http2.mlstatic.com/D_891743-MLC109938399813_042026-F.jpg"
        );
        assert_eq!(img_falabella("https://x/D_123-O.jpg"), "https://x/D_123-F.jpg");
    }
}
