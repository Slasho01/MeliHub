pub mod admin_controller;
pub mod api_key_controller;
pub mod auth_controller;
pub mod catalogo_controller;
pub mod auditoria_controller;
pub mod correo_controller;
pub mod cupon_controller;
pub mod dte_controller;
pub mod facturacion_controller;
pub mod equipo_controller;
pub mod falabella_controller;
pub mod ml_controller;
pub mod paris_controller;
pub mod reclamo_controller;
pub mod salud_controller;
pub mod preguntas_controller;
pub mod producto_controller;
pub mod pub_controller;
pub mod saas_controller;
pub mod suscripcion_controller;
pub mod venta_controller;

use actix_web::HttpResponse;
use serde_json::json;

pub async fn health() -> HttpResponse {
    HttpResponse::Ok().json(json!({ "status": "ok", "service": "melihub", "version": env!("CARGO_PKG_VERSION") }))
}
