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

/// Extrae dimensiones/peso del paquete desde los `attributes` de ML y los convierte a los
/// FeedNames obligatorios de Falabella (peso en **kg**, medidas en **cm**). Prefiere
/// `PACKAGE_*` (medido por ML) y cae a `SELLER_PACKAGE_*`.
pub fn dimensiones_ml(p: &Producto) -> Vec<(String, String)> {
    let arr = match p.datos.get("attributes").and_then(|a| a.as_array()) {
        Some(a) => a,
        None => return Vec::new(),
    };
    // Devuelve (número, unidad) del primer id que exista.
    let buscar = |ids: &[&str]| -> Option<(f64, String)> {
        for id in ids {
            if let Some(vn) = arr
                .iter()
                .find(|a| a.get("id").and_then(|v| v.as_str()) == Some(*id))
                .and_then(|a| a.get("value_name").and_then(|v| v.as_str()))
            {
                let num: String = vn.trim().chars().take_while(|c| c.is_ascii_digit() || *c == '.' || *c == ',').collect();
                if let Ok(n) = num.replace(',', ".").parse::<f64>() {
                    let unidad = vn.trim().trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == ',' || c == ' ').to_lowercase();
                    return Some((n, unidad));
                }
            }
        }
        None
    };
    let mut out = Vec::new();
    if let Some((n, u)) = buscar(&["PACKAGE_WEIGHT", "SELLER_PACKAGE_WEIGHT"]) {
        let kg = match u.as_str() {
            "g" | "gr" | "grs" | "gramos" => n / 1000.0,
            "mg" => n / 1_000_000.0,
            _ => n, // kg
        };
        out.push(("PackageWeight".to_string(), format!("{:.3}", kg.max(0.001))));
    }
    for (ids, feed) in [
        (["PACKAGE_WIDTH", "SELLER_PACKAGE_WIDTH"], "PackageWidth"),
        (["PACKAGE_LENGTH", "SELLER_PACKAGE_LENGTH"], "PackageLength"),
        (["PACKAGE_HEIGHT", "SELLER_PACKAGE_HEIGHT"], "PackageHeight"),
    ] {
        if let Some((n, u)) = buscar(&ids) {
            let cm = match u.as_str() {
                "m" | "mt" | "mts" | "metros" => n * 100.0,
                "mm" => n / 10.0,
                _ => n, // cm
            };
            out.push((feed.to_string(), format!("{}", (cm * 10.0).round() / 10.0)));
        }
    }
    out
}

/// XML de ProductUpdate con SOLO precio y stock (para la sincronización continua).
pub fn build_stock_precio_xml(sku: &str, precio: i64, stock: i32, operador: &str) -> String {
    let op = if operador.trim().is_empty() { "facl" } else { operador.trim() };
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<Request><Product>\
<SellerSku>{}</SellerSku>\
<BusinessUnits><BusinessUnit>\
<OperatorCode>{}</OperatorCode>\
<Price>{}</Price>\
<Stock>{}</Stock>\
<Status>active</Status>\
<IsPublished>1</IsPublished>\
</BusinessUnit></BusinessUnits>\
</Product></Request>",
        xml_escape(sku),
        xml_escape(op),
        precio.max(0),
        stock.max(0),
    )
}

/// XML de `ProductUpdate` en lote con precio/stock de varios productos (sync continuo).
/// Items: (seller_sku, precio, stock, operador).
pub fn build_stock_precio_batch_xml(items: &[(String, i64, i32, String)]) -> String {
    let mut products = String::new();
    for (sku, precio, stock, op) in items {
        let op = if op.trim().is_empty() { "facl" } else { op.trim() };
        products.push_str(&format!(
            "<Product><SellerSku>{}</SellerSku><BusinessUnits><BusinessUnit>\
<OperatorCode>{}</OperatorCode><Price>{}</Price><Stock>{}</Stock>\
<Status>active</Status><IsPublished>1</IsPublished>\
</BusinessUnit></BusinessUnits></Product>",
            xml_escape(sku),
            xml_escape(op),
            (*precio).max(0),
            (*stock).max(0),
        ));
    }
    format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<Request>{products}</Request>")
}

/// XML de `ProductUpdate` para pausar/reactivar un producto en Falabella.
pub fn build_estado_xml(sku: &str, activo: bool, operador: &str) -> String {
    let op = if operador.trim().is_empty() { "facl" } else { operador.trim() };
    let (status, pub_) = if activo { ("active", "1") } else { ("inactive", "0") };
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<Request><Product>\
<SellerSku>{}</SellerSku>\
<BusinessUnits><BusinessUnit>\
<OperatorCode>{}</OperatorCode><Status>{}</Status><IsPublished>{}</IsPublished>\
</BusinessUnit></BusinessUnits>\
</Product></Request>",
        xml_escape(sku),
        xml_escape(op),
        status,
        pub_,
    )
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

/// Mapa `picture_id -> url` (alta calidad jpg) para resolver las imágenes de cada variante.
fn mapa_pics(p: &Producto) -> std::collections::HashMap<String, String> {
    let mut m = std::collections::HashMap::new();
    if let Some(arr) = p.datos.get("pictures").and_then(|a| a.as_array()) {
        for pic in arr {
            if let (Some(id), Some(url)) = (
                pic.get("id").and_then(|v| v.as_str()),
                pic.get("secure_url").and_then(|v| v.as_str()).or_else(|| pic.get("url").and_then(|v| v.as_str())),
            ) {
                m.insert(id.to_string(), img_falabella(url));
            }
        }
    }
    m
}

/// Bloque `<Product>` para una variante (comparte nombre/categoría/marca/descr. con las demás).
#[allow(clippy::too_many_arguments)]
fn product_block(sku: &str, name: &str, desc: &str, category_id: &str, brand: &str, variation: &str, operador: &str, precio: i64, stock: i32, data_extra: &str, imagenes: &[String]) -> String {
    let mut images = String::new();
    for u in imagenes {
        images.push_str(&format!("<Image>{}</Image>", xml_escape(u)));
    }
    format!(
        "<Product><SellerSku>{}</SellerSku><Name>{}</Name><Variation>{}</Variation>\
<PrimaryCategory>{}</PrimaryCategory><Description>{}</Description><Brand>{}</Brand>\
<ProductData><ConditionType>Nuevo</ConditionType>{}</ProductData>\
<BusinessUnits><BusinessUnit><OperatorCode>{}</OperatorCode><Price>{}</Price><Stock>{}</Stock><Status>active</Status><IsPublished>1</IsPublished></BusinessUnit></BusinessUnits>\
<Images>{}</Images></Product>",
        xml_escape(sku), xml_escape(name), xml_escape(variation),
        xml_escape(category_id), xml_escape(desc), xml_escape(brand),
        data_extra, xml_escape(operador), precio.max(0), stock.max(0), images,
    )
}

/// Si el producto tiene variantes de ML, arma un ProductCreate con un `<Product>` por variante.
/// `None` si no tiene (usar `build_product_xml` normal). `atributos` = los comunes (obligatorios).
pub fn build_variantes_xml(p: &Producto, category_id: &str, brand: &str, operador: &str, atributos: &[(String, String)]) -> Option<String> {
    let vars = p.variantes.as_array().filter(|a| !a.is_empty())?;
    let op = if operador.trim().is_empty() { "facl" } else { operador.trim() };
    let desc = p.datos.get("descripcion").and_then(|d| d.as_str()).unwrap_or(&p.titulo);
    let precio_base = p.precio_falabella.filter(|&x| x > 0).unwrap_or(p.precio);
    let pics = mapa_pics(p);
    let comunes: String = atributos
        .iter()
        .filter(|(_, v)| !v.trim().is_empty())
        .map(|(k, v)| format!("<{t}>{}</{t}>", xml_escape(v.trim()), t = xml_tag(k)))
        .collect();

    let mut bloques = String::new();
    for v in vars {
        let Some(vid) = v.get("id").and_then(|x| x.as_i64()) else { continue };
        let combos = v.get("attribute_combinations").and_then(|a| a.as_array());
        let variation: String = combos
            .map(|c| c.iter().filter_map(|a| a.get("value_name").and_then(|x| x.as_str())).collect::<Vec<_>>().join(" / "))
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| format!("Opción {vid}"));
        // atributos propios de la variante (color/talla) + los comunes
        let mut data = comunes.clone();
        if let Some(c) = combos {
            for a in c {
                if let (Some(name), Some(val)) = (a.get("name").and_then(|x| x.as_str()), a.get("value_name").and_then(|x| x.as_str())) {
                    if !name.is_empty() && !val.is_empty() {
                        data.push_str(&format!("<{t}>{}</{t}>", xml_escape(val), t = xml_tag(name)));
                    }
                }
            }
        }
        let precio = v.get("price").and_then(|x| x.as_f64()).map(|f| f.round() as i64).filter(|&x| x > 0).unwrap_or(precio_base);
        let stock = v.get("available_quantity").and_then(|x| x.as_i64()).unwrap_or(0).max(0) as i32;
        let imagenes: Vec<String> = v
            .get("picture_ids")
            .and_then(|a| a.as_array())
            .map(|arr| arr.iter().filter_map(|pid| pid.as_str()).filter_map(|pid| pics.get(pid).cloned()).collect())
            .unwrap_or_default();
        let sku = format!("{}-{}", p.ml_item_id, vid);
        bloques.push_str(&product_block(&sku, &p.titulo, desc, category_id, brand, &variation, op, precio, stock, &data, &imagenes));
    }
    (!bloques.is_empty()).then(|| format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<Request>{bloques}</Request>"))
}

/// XML de ProductCreate/Update para un producto del hub con su categoría/atributos Falabella.
/// `variation` vacío → "No aplica"; `operador` vacío → "facl" (Chile).
pub fn build_product_xml(p: &Producto, category_id: &str, brand: &str, variation: &str, operador: &str, atributos: &[(String, String)]) -> String {
    let desc = p.datos.get("descripcion").and_then(|d| d.as_str()).unwrap_or(&p.titulo);
    let variation = if variation.trim().is_empty() { "No aplica" } else { variation.trim() };
    let operador = if operador.trim().is_empty() { "facl" } else { operador.trim() };
    let precio = p.precio_falabella.filter(|&x| x > 0).unwrap_or(p.precio); // precio propio de Falabella o el de ML
    product_xml(
        &p.ml_item_id,
        &p.titulo,
        desc,
        precio.max(0),
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
    fn bloque_variante_bien_formado() {
        let xml = product_block("MLA1-77", "Polera", "desc", "CAT1", "Nike", "Rojo / M", "facl", 9990, 4, "<Color>Rojo</Color>", &["https://img/a-F.jpg".to_string()]);
        assert!(xml.starts_with("<Product><SellerSku>MLA1-77</SellerSku>"));
        assert!(xml.contains("<Variation>Rojo / M</Variation>"));
        assert!(xml.contains("<ProductData><ConditionType>Nuevo</ConditionType><Color>Rojo</Color></ProductData>"));
        assert!(xml.contains("<Price>9990</Price><Stock>4</Stock>"));
        assert!(xml.contains("<Image>https://img/a-F.jpg</Image>"));
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
