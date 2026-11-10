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
