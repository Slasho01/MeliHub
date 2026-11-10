//! Cliente del **Seller Center API de Falabella** (marketplace).
//!
//! Cada solicitud va firmada: se toman todos los parámetros (menos `Signature`), se
//! ordenan por nombre, se codifican RFC 3986 y se unen `k=v&k=v`; sobre esa cadena se
//! calcula `HMAC-SHA256` con la API key (hex minúscula) y el resultado se agrega como
//! `Signature`. Base: `https://sellercenter-api.falabella.com`, respuestas en JSON.

use crate::errors::{ApiError, ApiResult};
use chrono::Utc;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::collections::BTreeMap;

const BASE_URL: &str = "https://sellercenter-api.falabella.com";

type HmacSha256 = Hmac<Sha256>;

/// Codifica según RFC 3986 (sin reservar: `A-Z a-z 0-9 - _ . ~`; el resto `%XX` en mayúscula).
fn encode_rfc3986(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for &b in s.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Arma la cadena a firmar/consultar (`k=v&k=v`, ya ordenada por `BTreeMap` y RFC3986).
fn query_string(params: &BTreeMap<String, String>) -> String {
    params
        .iter()
        .map(|(k, v)| format!("{}={}", encode_rfc3986(k), encode_rfc3986(v)))
        .collect::<Vec<_>>()
        .join("&")
}

/// Firma HMAC-SHA256 (hex minúscula) de los parámetros con la API key.
fn firmar(params: &BTreeMap<String, String>, api_key: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(api_key.as_bytes()).expect("HMAC acepta clave de cualquier tamaño");
    mac.update(query_string(params).as_bytes());
    mac.finalize().into_bytes().iter().map(|b| format!("{b:02x}")).collect()
}

#[derive(Clone)]
pub struct FalabellaClient {
    http: reqwest::Client,
}

impl Default for FalabellaClient {
    fn default() -> Self {
        Self::new()
    }
}

impl FalabellaClient {
    pub fn new() -> Self {
        FalabellaClient { http: reqwest::Client::new() }
    }

    /// Ejecuta una acción del API. `extra` = parámetros propios de la acción; `body` = XML
    /// para las acciones POST (ProductCreate/Update), o `None` para las GET.
    pub async fn request(
        &self,
        user_id: &str,
        api_key: &str,
        action: &str,
        extra: &[(&str, &str)],
        body: Option<String>,
    ) -> ApiResult<serde_json::Value> {
        let mut params: BTreeMap<String, String> = BTreeMap::new();
        params.insert("UserID".into(), user_id.to_string());
        params.insert("Version".into(), "1.0".into());
        params.insert("Action".into(), action.to_string());
        params.insert("Format".into(), "JSON".into());
        params.insert("Timestamp".into(), Utc::now().format("%Y-%m-%dT%H:%M:%S%:z").to_string());
        for (k, v) in extra {
            params.insert((*k).to_string(), (*v).to_string());
        }
        params.insert("Signature".into(), firmar(&params, api_key));

        let url = format!("{BASE_URL}/?{}", query_string(&params));
        let req = match &body {
            Some(xml) => self.http.post(&url).header("Content-Type", "text/xml; charset=UTF-8").body(xml.clone()),
            None => self.http.get(&url),
        };
        let res = req.send().await.map_err(|e| ApiError::internal(format!("Falabella no respondió: {e}")))?;
        let status = res.status();
        let text = res.text().await.unwrap_or_default();
        let json: serde_json::Value = serde_json::from_str(&text).unwrap_or_else(|_| serde_json::json!({ "raw": text }));

        if let Some(err) = json.get("ErrorResponse") {
            let msg = err
                .get("Head")
                .and_then(|h| h.get("ErrorMessage"))
                .and_then(|m| m.as_str())
                .unwrap_or("credenciales o solicitud inválida");
            return Err(ApiError::bad_request(format!("Falabella rechazó la solicitud: {msg}")));
        }
        if !status.is_success() {
            return Err(ApiError::bad_request(format!("Falabella respondió {status}")));
        }
        Ok(json)
    }

    /// Marcas del vendedor. Es una llamada de SOLO LECTURA: sirve para validar las
    /// credenciales al conectar la cuenta (si la firma/credenciales fallan, Falabella
    /// responde `ErrorResponse`).
    pub async fn get_brands(&self, user_id: &str, api_key: &str) -> ApiResult<serde_json::Value> {
        self.request(user_id, api_key, "GetBrands", &[], None).await
    }

    /// Árbol de categorías de Falabella (para mapear la categoría del producto).
    pub async fn get_category_tree(&self, user_id: &str, api_key: &str) -> ApiResult<serde_json::Value> {
        self.request(user_id, api_key, "GetCategoryTree", &[], None).await
    }

    /// Productos del vendedor (opcionalmente filtrados por SellerSku) — para ver estado/stock/imágenes.
    pub async fn get_products(&self, user_id: &str, api_key: &str, sku: Option<&str>) -> ApiResult<serde_json::Value> {
        let extra: Vec<(&str, &str)> = match sku {
            Some(s) => vec![("SkuSellerList", s)],
            None => vec![("Limit", "10")],
        };
        self.request(user_id, api_key, "GetProducts", &extra, None).await
    }

    /// Atributos (obligatorios y opcionales) de una categoría.
    pub async fn get_category_attributes(&self, user_id: &str, api_key: &str, category_id: &str) -> ApiResult<serde_json::Value> {
        self.request(user_id, api_key, "GetCategoryAttributes", &[("PrimaryCategory", category_id)], None).await
    }

    /// Crea productos en Falabella. El cuerpo es un XML `<Request><Product>…`. Devuelve el
    /// feed (asíncrono); su resultado se consulta luego con `FeedStatus`.
    pub async fn product_create(&self, user_id: &str, api_key: &str, xml: String) -> ApiResult<serde_json::Value> {
        self.request(user_id, api_key, "ProductCreate", &[], Some(xml)).await
    }

    /// Actualiza productos existentes (precio, stock, datos). Mismo formato XML.
    pub async fn product_update(&self, user_id: &str, api_key: &str, xml: String) -> ApiResult<serde_json::Value> {
        self.request(user_id, api_key, "ProductUpdate", &[], Some(xml)).await
    }

    /// Asocia imágenes a un producto (acción `Image`, aparte de ProductCreate). El cuerpo es
    /// `<Request><ProductImage><SellerSku>…</SellerSku><Images>…</Images></ProductImage></Request>`.
    pub async fn product_image(&self, user_id: &str, api_key: &str, xml: String) -> ApiResult<serde_json::Value> {
        self.request(user_id, api_key, "Image", &[], Some(xml)).await
    }

    /// Estado detallado de un feed (ProductCreate/Update son asíncronos): errores por producto.
    pub async fn feed_status(&self, user_id: &str, api_key: &str, feed_id: &str) -> ApiResult<serde_json::Value> {
        self.request(user_id, api_key, "FeedStatus", &[("FeedID", feed_id)], None).await
    }

    /// Lista los feeds recientes (últimos ~30 días) con su estado y nº de errores.
    pub async fn feed_list(&self, user_id: &str, api_key: &str) -> ApiResult<serde_json::Value> {
        self.request(user_id, api_key, "FeedList", &[], None).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn firma_deterministica_y_formato() {
        // Mismos parámetros + misma clave => misma firma hex de 64 chars (SHA-256).
        let mut p = BTreeMap::new();
        p.insert("Action".to_string(), "FeedList".to_string());
        p.insert("Format".to_string(), "XML".to_string());
        p.insert("UserID".to_string(), "test@seller.cl".to_string());
        p.insert("Version".to_string(), "1.0".to_string());
        p.insert("Timestamp".to_string(), "2015-07-01T11:11:11+00:00".to_string());
        let f1 = firmar(&p, "clave-secreta");
        assert_eq!(f1, firmar(&p, "clave-secreta")); // determinista
        assert_eq!(f1.len(), 64);
        assert!(f1.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
        // Known-answer: coincide con una implementación HMAC-SHA256 de referencia (Python)
        // sobre la MISMA cadena RFC3986, confirmando que la firma es un HMAC estándar correcto.
        assert_eq!(f1, "d8627513d701d75dab048788c106dcabd30dc0fec3ec68bca62d1d2ece0d74e5");
        // La cadena a firmar va ordenada y RFC3986 (el '@' y ':' se codifican, '+' también).
        let qs = query_string(&p);
        assert!(qs.starts_with("Action=FeedList&Format=XML&Timestamp=2015-07-01T11%3A11%3A11%2B00%3A00&UserID=test%40seller.cl&Version=1.0"));
    }
}
