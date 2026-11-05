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
    /// Ficha técnica de Mercado Libre: [{id, name, value_name, ...}, ...]
    #[serde(default)]
    pub attributes: serde_json::Value,
    /// Descripción en texto plano. NO viene en el multiget (es otro endpoint);
    /// la rellena la sync y se persiste dentro de `datos`.
    #[serde(default, skip_deserializing)]
    pub descripcion: Option<String>,
    /// Nombre de la categoría de ML (resuelto desde `category_id`); lo rellena la
    /// sync y se persiste en `datos`.
    #[serde(default, skip_deserializing)]
    pub categoria_nombre: Option<String>,
    /// Id del producto de catálogo de ML. Dos publicaciones (catálogo + normal)
    /// del mismo producto comparten este id → se usa para no duplicar.
    #[serde(default)]
    pub catalog_product_id: Option<String>,
    /// Si esta publicación es la de catálogo.
    #[serde(default)]
    pub catalog_listing: Option<bool>,
    /// Datos de envío de ML; se usa `logistic_type` ("fulfillment" = FULL).
    #[serde(default)]
    pub shipping: serde_json::Value,
    /// Producto del usuario (agrupa el stock por ubicación: depósito + FULL).
    #[serde(default)]
    pub user_product_id: Option<String>,
    /// Stock en bodega propia (ubicaciones ≠ `meli_facility`); lo rellena la sync
    /// desde `/user-products/{id}/stock` y se persiste en `datos`.
    #[serde(default, skip_deserializing)]
    pub deposito_stock: Option<i64>,
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
        let attrs = "id,title,price,available_quantity,status,permalink,thumbnail,category_id,currency_id,variations,attributes,catalog_product_id,catalog_listing,shipping,user_product_id";
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

    /// Descripción en texto plano de una publicación (endpoint aparte del item).
    /// Devuelve None si no tiene o si falla (es best-effort).
    pub async fn get_description(&self, access_token: &str, item_id: &str) -> Option<String> {
        let v: serde_json::Value = self.get(access_token, &format!("/items/{item_id}/description")).await.ok()?;
        v.get("plain_text")
            .or_else(|| v.get("text"))
            .and_then(|t| t.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    }

    /// Busca órdenes (ventas) del vendedor, más recientes primero. Devuelve el
    /// objeto crudo de ML (con `results` y `paging`). Requiere permiso de lectura
    /// de ventas en la app de ML.
    pub async fn search_orders(&self, access_token: &str, seller_id: i64, offset: i64) -> ApiResult<serde_json::Value> {
        self.get(
            access_token,
            &format!("/orders/search?seller={seller_id}&sort=date_desc&offset={offset}&limit=50"),
        )
        .await
    }

    /// Nombre de una categoría de Mercado Libre (p. ej. "MLC1234" -> "Taladros").
    /// Best-effort: None si falla.
    pub async fn get_category(&self, access_token: &str, cat_id: &str) -> Option<String> {
        let v: serde_json::Value = self.get(access_token, &format!("/categories/{cat_id}")).await.ok()?;
        v.get("name")
            .and_then(|n| n.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    }

    /// Stock en la **bodega propia** del vendedor (suma de ubicaciones que no son
    /// `meli_facility`/FULL) desde `/user-products/{id}/stock`. None si falla.
    pub async fn get_deposito_stock(&self, access_token: &str, user_product_id: &str) -> Option<i64> {
        let v: serde_json::Value = self.get(access_token, &format!("/user-products/{user_product_id}/stock")).await.ok()?;
        let locs = v.get("locations")?.as_array()?;
        let dep: i64 = locs
            .iter()
            .filter(|l| l.get("type").and_then(|t| t.as_str()) != Some("meli_facility"))
            .filter_map(|l| l.get("quantity").and_then(|q| q.as_i64()))
            .sum();
        Some(dep)
    }

    /// Actualiza el stock disponible de una publicación en Mercado Libre.
    pub async fn update_stock(&self, access_token: &str, item_id: &str, qty: i32) -> ApiResult<()> {
        let res = self
            .http
            .put(format!("{}/items/{item_id}", self.cfg.api_base))
            .bearer_auth(access_token)
            .header("Accept", "application/json")
            .json(&serde_json::json!({ "available_quantity": qty.max(0) }))
            .send()
            .await
            .map_err(|e| ApiError::internal(format!("Mercado Libre no respondió: {e}")))?;
        let status = res.status();
        if !status.is_success() {
            let text = res.text().await.unwrap_or_default();
            return Err(ApiError::bad_request(format!("Mercado Libre rechazó actualizar stock ({status}): {text}")));
        }
        Ok(())
    }

    /// Actualiza el **precio** de una publicación en Mercado Libre.
    pub async fn update_price(&self, access_token: &str, item_id: &str, precio: i64) -> ApiResult<()> {
        self.put_item(access_token, item_id, serde_json::json!({ "price": precio.max(0) })).await
    }

    /// Pausa o reactiva una publicación (`status` = "paused" | "active").
    pub async fn set_status(&self, access_token: &str, item_id: &str, status: &str) -> ApiResult<()> {
        self.put_item(access_token, item_id, serde_json::json!({ "status": status })).await
    }

    /// PUT genérico a /items/{id} con el cuerpo dado.
    async fn put_item(&self, access_token: &str, item_id: &str, body: serde_json::Value) -> ApiResult<()> {
        let res = self
            .http
            .put(format!("{}/items/{item_id}", self.cfg.api_base))
            .bearer_auth(access_token)
            .header("Accept", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| ApiError::internal(format!("Mercado Libre no respondió: {e}")))?;
        let status = res.status();
        if !status.is_success() {
            let text = res.text().await.unwrap_or_default();
            return Err(ApiError::bad_request(format!("Mercado Libre rechazó la actualización ({status}): {text}")));
        }
        Ok(())
    }

    /// Actualiza SOLO el stock de la bodega propia (ubicación `selling_address`)
    /// de un user-product, sin tocar el de FULL (`meli_facility`).
    pub async fn update_deposito_stock(&self, access_token: &str, user_product_id: &str, qty: i32) -> ApiResult<()> {
        let body = serde_json::json!({
            "locations": [{ "type": "selling_address", "quantity": qty.max(0) }]
        });
        let res = self
            .http
            .put(format!("{}/user-products/{user_product_id}/stock", self.cfg.api_base))
            .bearer_auth(access_token)
            .header("Accept", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| ApiError::internal(format!("Mercado Libre no respondió: {e}")))?;
        let status = res.status();
        if !status.is_success() {
            let text = res.text().await.unwrap_or_default();
            return Err(ApiError::bad_request(format!("Mercado Libre rechazó actualizar stock de depósito ({status}): {text}")));
        }
        Ok(())
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
