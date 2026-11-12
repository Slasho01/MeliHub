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
    /// Todas las imágenes de la publicación: [{id, url, secure_url, ...}, ...].
    #[serde(default)]
    pub pictures: serde_json::Value,
    /// Id del video de YouTube de la publicación (si tiene).
    #[serde(default)]
    pub video_id: Option<String>,
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
    /// Ruta completa de categorías de ML (nombres, de raíz a hoja) desde `path_from_root`;
    /// la rellena la sync y se persiste en `datos`. Permite jerarquía en los canales.
    #[serde(default, skip_deserializing)]
    pub categoria_ruta: Vec<String>,
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
        let attrs = "id,title,price,available_quantity,status,permalink,thumbnail,pictures,video_id,category_id,currency_id,variations,attributes,catalog_product_id,catalog_listing,shipping,user_product_id";
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

    /// Preguntas recibidas SIN responder de un vendedor (las más antiguas primero).
    pub async fn questions_unanswered(&self, access_token: &str, seller_id: i64, offset: i64) -> ApiResult<serde_json::Value> {
        self.get(
            access_token,
            &format!("/questions/search?seller_id={seller_id}&status=UNANSWERED&sort=date_asc&api_version=4&offset={offset}&limit=50"),
        )
        .await
    }

    /// Responde una pregunta de un comprador en Mercado Libre.
    pub async fn answer_question(&self, access_token: &str, question_id: i64, text: &str) -> ApiResult<()> {
        let res = self
            .http
            .post(format!("{}/answers", self.cfg.api_base))
            .bearer_auth(access_token)
            .header("Accept", "application/json")
            .json(&serde_json::json!({ "question_id": question_id, "text": text }))
            .send()
            .await
            .map_err(|e| ApiError::internal(format!("Mercado Libre no respondió: {e}")))?;
        let status = res.status();
        if !status.is_success() {
            let text = res.text().await.unwrap_or_default();
            return Err(ApiError::bad_request(format!("Mercado Libre rechazó la respuesta ({status}): {text}")));
        }
        Ok(())
    }

    // ---------------- Reclamos (post-venta) ----------------

    /// Busca los reclamos del vendedor (los más nuevos primero). El vendedor va implícito
    /// en el token. Devuelve `{ data: [...], paging: {...} }`.
    pub async fn claims_search(&self, access_token: &str, offset: i64) -> ApiResult<serde_json::Value> {
        self.get(access_token, &format!("/post-purchase/v1/claims/search?sort=date_desc&offset={offset}&limit=50")).await
    }

    /// Detalle de un reclamo.
    pub async fn claim_detail(&self, access_token: &str, claim_id: i64) -> ApiResult<serde_json::Value> {
        self.get(access_token, &format!("/post-purchase/v1/claims/{claim_id}")).await
    }

    /// Mensajes (conversación) de un reclamo.
    pub async fn claim_messages(&self, access_token: &str, claim_id: i64) -> ApiResult<serde_json::Value> {
        self.get(access_token, &format!("/post-purchase/v1/claims/{claim_id}/messages")).await
    }

    /// Envía un mensaje al comprador dentro de un reclamo.
    pub async fn claim_send_message(&self, access_token: &str, claim_id: i64, text: &str) -> ApiResult<()> {
        let res = self
            .http
            .post(format!("{}/post-purchase/v1/claims/{claim_id}/actions/send-message", self.cfg.api_base))
            .bearer_auth(access_token)
            .header("Accept", "application/json")
            .json(&serde_json::json!({ "message": text }))
            .send()
            .await
            .map_err(|e| ApiError::internal(format!("Mercado Libre no respondió: {e}")))?;
        let status = res.status();
        if !status.is_success() {
            let t = res.text().await.unwrap_or_default();
            return Err(ApiError::bad_request(format!("Mercado Libre rechazó el mensaje ({status}): {t}")));
        }
        Ok(())
    }

    // ---------------- Catálogo (competencia / Buy Box) ----------------

    /// IDs de las publicaciones del vendedor que son de **catálogo** (`catalog_listing=true`).
    /// Devuelve `{ results: [item_ids], paging: {...} }`.
    pub async fn catalog_item_ids(&self, access_token: &str, seller_id: i64, offset: i64) -> ApiResult<serde_json::Value> {
        self.get(access_token, &format!("/users/{seller_id}/items/search?catalog_listing=true&offset={offset}&limit=50")).await
    }

    /// Estado de competencia de un ítem de catálogo: si gana/pierde la Buy Box, el precio del
    /// ganador y el precio necesario para ganar. `GET /items/{id}/price_to_win?version=v2`.
    pub async fn price_to_win(&self, access_token: &str, item_id: &str) -> ApiResult<serde_json::Value> {
        self.get(access_token, &format!("/items/{item_id}/price_to_win?version=v2")).await
    }

    /// Detalle de un producto de catálogo (nombre, atributos). `GET /products/{id}`.
    pub async fn catalog_product(&self, access_token: &str, product_id: &str) -> Option<serde_json::Value> {
        self.get(access_token, &format!("/products/{product_id}")).await.ok()
    }

    /// Detalle de un envío: `logistic_type` (fulfillment/self_service/…) y `status`.
    pub async fn get_shipment(&self, access_token: &str, shipment_id: i64) -> Option<serde_json::Value> {
        self.get(access_token, &format!("/shipments/{shipment_id}")).await.ok()
    }

    /// Descarga en PDF las etiquetas de uno o varios envíos (Mercado Envíos Flex/Colecta).
    /// Falla si algún envío no tiene etiqueta imprimible (p. ej. FULL).
    pub async fn shipment_labels(&self, access_token: &str, shipment_ids: &[i64]) -> ApiResult<Vec<u8>> {
        if shipment_ids.is_empty() {
            return Err(ApiError::bad_request("Sin envíos para la etiqueta"));
        }
        let ids = shipment_ids.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(",");
        let res = self
            .http
            .get(format!("{}/shipment_labels?shipment_ids={ids}&response_type=pdf", self.cfg.api_base))
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| ApiError::internal(format!("Mercado Libre no respondió: {e}")))?;
        let status = res.status();
        if !status.is_success() {
            let t = res.text().await.unwrap_or_default();
            return Err(ApiError::bad_request(format!("Mercado Libre no entregó la etiqueta ({status}): {t}")));
        }
        let bytes = res.bytes().await.map_err(|e| ApiError::internal(format!("No se pudo leer el PDF de ML: {e}")))?;
        Ok(bytes.to_vec())
    }

    /// Nombre de una categoría de Mercado Libre (p. ej. "MLC1234" -> "Taladros").
    /// Best-effort: None si falla.
    /// Ruta de categoría de ML (nombres, de raíz a hoja) vía `path_from_root`.
    /// Vacío si falla. La hoja es el último elemento.
    pub async fn get_category_ruta(&self, access_token: &str, cat_id: &str) -> Vec<String> {
        let v: serde_json::Value = match self.get(access_token, &format!("/categories/{cat_id}")).await {
            Ok(v) => v,
            Err(_) => return vec![],
        };
        if let Some(arr) = v.get("path_from_root").and_then(|p| p.as_array()) {
            let ruta: Vec<String> = arr
                .iter()
                .filter_map(|n| n.get("name").and_then(|x| x.as_str()))
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if !ruta.is_empty() {
                return ruta;
            }
        }
        // Fallback: solo el nombre hoja.
        v.get("name")
            .and_then(|n| n.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .map(|s| vec![s])
            .unwrap_or_default()
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
