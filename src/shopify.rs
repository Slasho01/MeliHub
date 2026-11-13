//! Cliente de la Admin API de Shopify (REST). Auth por `X-Shopify-Access-Token` (token de una
//! "custom app" de la tienda). Base: https://{shop}.myshopify.com/admin/api/{ver}.

use crate::errors::{ApiError, ApiResult};
use serde_json::{json, Value};

const API_VER: &str = "2026-10";

pub struct ShopifyClient {
    http: reqwest::Client,
}

impl Default for ShopifyClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Normaliza el dominio (sin esquema ni barra final).
pub fn dominio(shop: &str) -> String {
    shop.trim().trim_end_matches('/').trim_start_matches("https://").trim_start_matches("http://").to_string()
}

impl ShopifyClient {
    pub fn new() -> Self {
        Self { http: reqwest::Client::new() }
    }

    async fn req(&self, shop: &str, token: &str, method: reqwest::Method, path: &str, body: Option<&Value>) -> ApiResult<Value> {
        let url = format!("https://{}/admin/api/{API_VER}{path}", dominio(shop));
        let mut rb = self.http.request(method, url).header("X-Shopify-Access-Token", token).header("Accept", "application/json");
        if let Some(b) = body {
            rb = rb.json(b);
        }
        let res = rb.send().await.map_err(|e| ApiError::internal(format!("Shopify no respondió: {e}")))?;
        let status = res.status();
        let txt = res.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(ApiError::bad_request(format!("Shopify rechazó la operación ({status}): {txt}")));
        }
        if txt.trim().is_empty() {
            return Ok(Value::Null);
        }
        serde_json::from_str(&txt).map_err(|e| ApiError::internal(format!("Respuesta de Shopify inválida: {e}")))
    }

    /// OAuth: intercambia el `code` del callback por el access token de la tienda.
    pub async fn exchange_code(&self, shop: &str, client_id: &str, client_secret: &str, code: &str) -> ApiResult<String> {
        let url = format!("https://{}/admin/oauth/access_token", dominio(shop));
        let body = json!({ "client_id": client_id, "client_secret": client_secret, "code": code });
        let res = self.http.post(url).json(&body).send().await.map_err(|e| ApiError::internal(format!("Shopify no respondió: {e}")))?;
        let status = res.status();
        let txt = res.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(ApiError::bad_request(format!("Shopify rechazó el intercambio OAuth ({status}): {txt}")));
        }
        serde_json::from_str::<Value>(&txt)
            .ok()
            .and_then(|v| v.get("access_token").and_then(|x| x.as_str()).map(|s| s.to_string()))
            .ok_or_else(|| ApiError::internal("Shopify no devolvió access_token"))
    }

    /// Verifica la conexión (datos de la tienda).
    pub async fn verificar(&self, shop: &str, token: &str) -> ApiResult<Value> {
        self.req(shop, token, reqwest::Method::GET, "/shop.json", None).await
    }

    /// Id de la ubicación principal (necesaria para fijar inventario).
    pub async fn ubicacion_principal(&self, shop: &str, token: &str) -> ApiResult<Option<i64>> {
        let v = self.req(shop, token, reqwest::Method::GET, "/locations.json", None).await?;
        Ok(v.get("locations").and_then(|l| l.as_array()).and_then(|a| a.first()).and_then(|x| x.get("id")).and_then(|x| x.as_i64()))
    }

    pub async fn crear_producto(&self, shop: &str, token: &str, body: &Value) -> ApiResult<Value> {
        self.req(shop, token, reqwest::Method::POST, "/products.json", Some(body)).await
    }

    pub async fn actualizar_variante_precio(&self, shop: &str, token: &str, variant_id: &str, precio: i64) -> ApiResult<Value> {
        let body = json!({ "variant": { "id": variant_id.parse::<i64>().unwrap_or(0), "price": precio.to_string() } });
        self.req(shop, token, reqwest::Method::PUT, &format!("/variants/{variant_id}.json"), Some(&body)).await
    }

    pub async fn set_inventario(&self, shop: &str, token: &str, inventory_item_id: &str, location_id: i64, qty: i32) -> ApiResult<Value> {
        let body = json!({ "location_id": location_id, "inventory_item_id": inventory_item_id.parse::<i64>().unwrap_or(0), "available": qty.max(0) });
        self.req(shop, token, reqwest::Method::POST, "/inventory_levels/set.json", Some(&body)).await
    }

    pub async fn get_orders(&self, shop: &str, token: &str) -> ApiResult<Value> {
        self.req(shop, token, reqwest::Method::GET, "/orders.json?status=any&limit=50", None).await
    }
}

/// Cuerpo para crear un producto en Shopify desde un producto del hub.
pub fn build_producto(titulo: &str, descripcion_html: &str, precio: i64, sku: &str, stock: i32, imagenes: &[String]) -> Value {
    let imgs: Vec<Value> = imagenes.iter().filter(|u| !u.trim().is_empty()).map(|u| json!({ "src": u })).collect();
    json!({
        "product": {
            "title": titulo,
            "body_html": descripcion_html,
            "status": "active",
            "images": imgs,
            "variants": [ {
                "price": precio.to_string(),
                "sku": sku,
                "inventory_management": "shopify",
                "inventory_quantity": stock.max(0),
            } ],
        }
    })
}
