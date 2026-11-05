pub mod api_key_controller;
pub mod auth_controller;
pub mod ml_controller;
pub mod producto_controller;
pub mod pub_controller;
pub mod venta_controller;

use actix_web::HttpResponse;
use serde_json::json;

pub async fn health() -> HttpResponse {
    HttpResponse::Ok().json(json!({ "status": "ok", "service": "melihub", "version": env!("CARGO_PKG_VERSION") }))
}
