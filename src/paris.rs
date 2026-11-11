//! Cliente del **Marketplace de Cencosud (Paris)**. REST + JSON.
//!
//! Auth: `POST /v1/auth/apiKey` con header `Authorization: <apiKey>` devuelve
//! `{ accessToken, expiresIn }` (≈4 h). Luego cada request va con
//! `Authorization: Bearer <accessToken>`. El token se cachea por API key y se
//! renueva solo antes de vencer, para no pegarle al auth en cada llamada.

use crate::errors::{ApiError, ApiResult};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const BASE_PROD: &str = "https://api-developers.ecomm.cencosud.com";
const BASE_STG: &str = "https://api-developers.ecomm-stg.cencosud.com";

#[derive(Clone)]
pub struct ParisClient {
    http: reqwest::Client,
    /// key (`sandbox|apiKey`) -> (accessToken, vencimiento)
    tokens: Arc<Mutex<HashMap<String, (String, Instant)>>>,
}

impl Default for ParisClient {
    fn default() -> Self {
        Self::new()
    }
}

impl ParisClient {
    pub fn new() -> Self {
        ParisClient { http: reqwest::Client::new(), tokens: Arc::new(Mutex::new(HashMap::new())) }
    }

    fn base(sandbox: bool) -> &'static str {
        if sandbox {
            BASE_STG
        } else {
            BASE_PROD
        }
    }

    /// Access Token válido (lo reutiliza del caché o lo renueva con la API key).
    async fn token(&self, sandbox: bool, api_key: &str) -> ApiResult<String> {
        let ckey = format!("{sandbox}|{api_key}");
        if let Ok(g) = self.tokens.lock() {
            if let Some((tok, exp)) = g.get(&ckey) {
                if *exp > Instant::now() {
                    return Ok(tok.clone());
                }
            }
        }
        let url = format!("{}/v1/auth/apiKey", Self::base(sandbox));
        let res = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {api_key}"))
            .header("Content-Type", "application/json")
            .send()
            .await
            .map_err(|e| ApiError::internal(format!("Paris no respondió (auth): {e}")))?;
        let status = res.status();
        let body: Value = res.json().await.unwrap_or(Value::Null);
        if !status.is_success() {
            return Err(ApiError::bad_request(format!("Paris rechazó la API key ({status}): {body}")));
        }
        let tok = body
            .get("accessToken")
            .and_then(|t| t.as_str())
            .ok_or_else(|| ApiError::internal("Paris no devolvió accessToken"))?
            .to_string();
        let secs = body.get("expiresIn").and_then(|e| e.as_i64()).unwrap_or(14400).max(120) as u64;
        if let Ok(mut g) = self.tokens.lock() {
            // Renovamos 60 s antes del vencimiento real.
            g.insert(ckey, (tok.clone(), Instant::now() + Duration::from_secs(secs.saturating_sub(60))));
        }
        Ok(tok)
    }

    /// Solicitud genérica autenticada; devuelve el JSON crudo.
    async fn req(&self, sandbox: bool, api_key: &str, method: reqwest::Method, path: &str, body: Option<&Value>) -> ApiResult<Value> {
        let token = self.token(sandbox, api_key).await?;
        let url = format!("{}{}", Self::base(sandbox), path);
        let mut rb = self.http.request(method, &url).bearer_auth(&token).header("Content-Type", "application/json");
        if let Some(b) = body {
            rb = rb.json(b);
        }
        let res = rb.send().await.map_err(|e| ApiError::internal(format!("Paris no respondió: {e}")))?;
        let status = res.status();
        let data: Value = res.json().await.unwrap_or(Value::Null);
        if !status.is_success() {
            return Err(ApiError::bad_request(format!("Paris respondió {status}: {data}")));
        }
        Ok(data)
    }

    // ---------------- Taxonomía v2 (las v1 están deprecadas → 503) ----------------
    pub async fn get_families(&self, sandbox: bool, api_key: &str) -> ApiResult<Value> {
        self.req(sandbox, api_key, reqwest::Method::GET, "/v2/families", None).await
    }
    pub async fn get_categories(&self, sandbox: bool, api_key: &str, family: &str) -> ApiResult<Value> {
        self.req(sandbox, api_key, reqwest::Method::GET, &format!("/v2/categories/family/{family}"), None).await
    }
    pub async fn get_attributes(&self, sandbox: bool, api_key: &str, family_id: &str) -> ApiResult<Value> {
        // Atributos de PRODUCTO (ficha): van en product.attributes.
        self.req(sandbox, api_key, reqwest::Method::GET, &format!("/v2/attributes/product/family/{family_id}"), None).await
    }
    pub async fn get_variant_attributes(&self, sandbox: bool, api_key: &str, family_id: &str) -> ApiResult<Value> {
        // Atributos de VARIANTE (color/talla…): van en variants[].attributes (requeridos).
        self.req(sandbox, api_key, reqwest::Method::GET, &format!("/v2/attributes/variant/family/{family_id}"), None).await
    }

    // ---------------- Productos (v2; v1 está deprecado → 404) ----------------
    pub async fn product_create(&self, sandbox: bool, api_key: &str, body: &Value) -> ApiResult<Value> {
        self.req(sandbox, api_key, reqwest::Method::POST, "/v2/products", Some(body)).await
    }
    pub async fn product_update(&self, sandbox: bool, api_key: &str, sku: &str, body: &Value) -> ApiResult<Value> {
        self.req(sandbox, api_key, reqwest::Method::PATCH, &format!("/v2/products/{sku}"), Some(body)).await
    }
    pub async fn variant_create(&self, sandbox: bool, api_key: &str, body: &Value) -> ApiResult<Value> {
        self.req(sandbox, api_key, reqwest::Method::POST, "/v1/variant", Some(body)).await
    }

    // ---------------- Precio / stock (v2) ----------------
    /// Tipos de precio de la cuenta (para `storePrice`): `{ results: [{ id, name }] }`.
    pub async fn get_price_types(&self, sandbox: bool, api_key: &str) -> ApiResult<Value> {
        self.req(sandbox, api_key, reqwest::Method::GET, "/v2/price-types", None).await
    }
    pub async fn update_price(&self, sandbox: bool, api_key: &str, sku: &str, body: &Value) -> ApiResult<Value> {
        self.req(sandbox, api_key, reqwest::Method::POST, &format!("/v2/prices/product/{sku}"), Some(body)).await
    }
    /// Stock v2: `body = { "skus": [ { "sku": "...", "quantity": N } ] }`.
    pub async fn update_stock_sku_seller(&self, sandbox: bool, api_key: &str, body: &Value) -> ApiResult<Value> {
        self.req(sandbox, api_key, reqwest::Method::POST, "/v2/stock", Some(body)).await
    }

    // ---------------- Órdenes ----------------
    pub async fn get_orders(&self, sandbox: bool, api_key: &str, query: &str) -> ApiResult<Value> {
        let path = if query.is_empty() { "/v1/orders".to_string() } else { format!("/v1/orders?{query}") };
        self.req(sandbox, api_key, reqwest::Method::GET, &path, None).await
    }
    pub async fn get_order(&self, sandbox: bool, api_key: &str, order_number: &str) -> ApiResult<Value> {
        self.req(sandbox, api_key, reqwest::Method::GET, &format!("/v1/orders/{order_number}"), None).await
    }
    pub async fn get_sub_orders(&self, sandbox: bool, api_key: &str, query: &str) -> ApiResult<Value> {
        let path = if query.is_empty() { "/v1/sub-orders".to_string() } else { format!("/v1/sub-orders?{query}") };
        self.req(sandbox, api_key, reqwest::Method::GET, &path, None).await
    }
}
