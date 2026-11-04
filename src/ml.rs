//! Cliente de la API de Mercado Libre: OAuth (authorization code + refresh) y
//! lectura de publicaciones. La base (`api_base`/`auth_base`) es configurable
//! para poder apuntar a un servidor simulado en pruebas.
//!
//! Contrato según la documentación de Mercado Libre:
//!  - Autorización: GET {auth_base}/authorization?response_type=code&client_id=...&redirect_uri=...&state=...
//!  - Token:        POST {api_base}/oauth/token  (form-urlencoded)
//!  - Ítems del usuario: GET {api_base}/users/{id}/items/search
//!  - Detalle (multiget): GET {api_base}/items?ids=ID1,ID2&attributes=...
//!
//! NOTA: el mapeo exacto de campos debe confirmarse contra la app real de ML;
//! está aislado aquí para ajustarlo en un solo lugar (como el adaptador de Blue
//! Express en JarStore).

use crate::config::MlConfig;
use crate::errors::{ApiError, ApiResult};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct MlClient {
    cfg: MlConfig,
    http: reqwest::Client,
}

#[derive(Deserialize, Debug)]
pub struct TokenResponse {
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: String,
    pub expires_in: i64,
    #[serde(default)]
    pub user_id: i64,
}

#[derive(Deserialize, Debug, Default)]
pub struct UserInfo {
    #[serde(default)]
    pub id: i64,
    #[serde(default)]
    pub nickname: Option<String>,
    #[serde(default)]
    pub site_id: Option<String>,
}

#[derive(Deserialize, Debug, Default)]
pub struct ItemSearch {
    #[serde(default)]
    pub results: Vec<String>,
    #[serde(default)]
    pub paging: Paging,
}
#[derive(Deserialize, Debug, Default)]
#[allow(dead_code)]
pub struct Paging {
    #[serde(default)]
    pub total: i64,
    #[serde(default)]
    pub offset: i64,
    #[serde(default)]
    pub limit: i64,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct MlItem {
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub price: Option<f64>,
    #[serde(default)]
    pub available_quantity: Option<i64>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub permalink: Option<String>,
    #[serde(default)]
    pub thumbnail: Option<String>,
    #[serde(default)]
    pub category_id: Option<String>,
    #[serde(default)]
    pub currency_id: Option<String>,
    #[serde(default)]
    pub variations: serde_json::Value,
}

/// Envoltorio del multiget: [{ "code": 200, "body": {item} }, ...]
#[derive(Deserialize)]
struct MultiGet {
    code: i32,
    body: serde_json::Value,
}

impl MlClient {
    pub fn new(cfg: MlConfig) -> Self {
        MlClient {
            cfg,
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .unwrap_or_default(),
        }
    }

    /// URL a la que se envía al vendedor para que autorice la app.
    pub fn authorize_url(&self, state: &str) -> String {
        format!(
            "{}/authorization?response_type=code&client_id={}&redirect_uri={}&state={}",
            self.cfg.auth_base,
            urlencoding(&self.cfg.client_id),
            urlencoding(&self.cfg.redirect_uri),
            urlencoding(state),
        )
    }

    /// Intercambia el `code` de la redirección por tokens.
    pub async fn exchange_code(&self, code: &str) -> ApiResult<TokenResponse> {
        self.token_request(&[
            ("grant_type", "authorization_code"),
            ("client_id", &self.cfg.client_id),
            ("client_secret", &self.cfg.client_secret),
            ("code", code),
            ("redirect_uri", &self.cfg.redirect_uri),
        ])
        .await
    }

    /// Renueva el access_token con el refresh_token.
    pub async fn refresh(&self, refresh_token: &str) -> ApiResult<TokenResponse> {
        self.token_request(&[
            ("grant_type", "refresh_token"),
            ("client_id", &self.cfg.client_id),
            ("client_secret", &self.cfg.client_secret),
            ("refresh_token", refresh_token),
        ])
        .await
    }

    async fn token_request(&self, form: &[(&str, &str)]) -> ApiResult<TokenResponse> {
        let res = self
            .http
            .post(format!("{}/oauth/token", self.cfg.api_base))
            .header("Accept", "application/json")
            .form(form)
            .send()
            .await
            .map_err(|e| ApiError::internal(format!("Mercado Libre no respondió: {e}")))?;
        let status = res.status();
        let text = res.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(ApiError::bad_request(format!("Mercado Libre rechazó el token ({status}): {text}")));
        }
        serde_json::from_str(&text).map_err(|e| ApiError::internal(format!("Token de ML no reconocido: {e}")))
    }

    pub async fn get_user(&self, access_token: &str, user_id: i64) -> ApiResult<UserInfo> {
        let who = if user_id > 0 { user_id.to_string() } else { "me".to_string() };
        let v: serde_json::Value = self.get(access_token, &format!("/users/{who}")).await?;
        Ok(serde_json::from_value(v).unwrap_or_default())
    }

    /// Devuelve una página de ids de publicaciones del usuario.
    pub async fn list_item_ids(&self, access_token: &str, user_id: i64, offset: i64) -> ApiResult<ItemSearch> {
        let v: serde_json::Value = self
            .get(access_token, &format!("/users/{user_id}/items/search?limit=50&offset={offset}"))
            .await?;
        Ok(serde_json::from_value(v).unwrap_or_default())
    }

    /// Trae el detalle de varias publicaciones (multiget, hasta 20 por llamada).
    pub async fn get_items(&self, access_token: &str, ids: &[String]) -> ApiResult<Vec<MlItem>> {
        if ids.is_empty() {
            return Ok(vec![]);
        }
        let attrs = "id,title,price,available_quantity,status,permalink,thumbnail,category_id,currency_id,variations";
        let v: serde_json::Value = self
            .get(access_token, &format!("/items?ids={}&attributes={attrs}", ids.join(",")))
            .await?;
        // Puede venir como multiget [{code,body}] o como lista directa de items.
        if let Ok(multi) = serde_json::from_value::<Vec<MultiGet>>(v.clone()) {
            Ok(multi
                .into_iter()
                .filter(|m| m.code == 200)
                .filter_map(|m| serde_json::from_value::<MlItem>(m.body).ok())
                .collect())
        } else {
            Ok(serde_json::from_value::<Vec<MlItem>>(v).unwrap_or_default())
        }
    }

    async fn get(&self, access_token: &str, path: &str) -> ApiResult<serde_json::Value> {
        let res = self
            .http
            .get(format!("{}{path}", self.cfg.api_base))
            .bearer_auth(access_token)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| ApiError::internal(format!("Mercado Libre no respondió: {e}")))?;
        let status = res.status();
        let text = res.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(ApiError::bad_request(format!("Mercado Libre respondió {status} en {path}: {text}")));
        }
        serde_json::from_str(&text).map_err(|e| ApiError::internal(format!("Respuesta de ML no reconocida: {e}")))
    }
}

/// Codificación mínima para parámetros de URL.
fn urlencoding(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}
