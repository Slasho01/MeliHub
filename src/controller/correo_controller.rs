//! Configuración de correo (solo admin): elegir proveedor (Resend / SMTP), guardar y probar.

use crate::auth::AuthVendedor;
use crate::db::{self, DbPool};
use crate::errors::{ApiError, ApiResult};
use crate::services::correo_services::{self, CorreoInput};
use actix_web::{web, HttpResponse};
use serde::Deserialize;
use serde_json::json;

fn exigir_admin(user: &AuthVendedor) -> ApiResult<()> {
    if !user.es_admin() {
        return Err(ApiError::forbidden("Solo el administrador puede configurar el correo."));
    }
    Ok(())
}

/// Config actual, SIN secretos (solo flags de si están puestos).
pub async fn obtener(user: AuthVendedor, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    exigir_admin(&user)?;
    let c = db::run(&pool, correo_services::get).await?;
    let resp = match c {
        Some(c) => json!({
            "proveedor": c.proveedor, "smtp_host": c.smtp_host, "smtp_port": c.smtp_port,
            "smtp_usuario": c.smtp_usuario, "smtp_tls": c.smtp_tls, "remitente": c.remitente,
            "tiene_resend_key": c.resend_api_key.is_some(), "tiene_smtp_password": c.smtp_password.is_some(),
        }),
        None => json!({ "proveedor": "resend", "smtp_port": 587, "smtp_tls": true, "tiene_resend_key": false, "tiene_smtp_password": false }),
    };
    Ok(HttpResponse::Ok().json(resp))
}

#[derive(Deserialize)]
pub struct CorreoBody {
    pub proveedor: String,
    #[serde(default)]
    pub resend_api_key: Option<String>,
    #[serde(default)]
    pub smtp_host: Option<String>,
    #[serde(default)]
    pub smtp_port: Option<i32>,
    #[serde(default)]
    pub smtp_usuario: Option<String>,
    #[serde(default)]
    pub smtp_password: Option<String>,
    #[serde(default)]
    pub smtp_tls: Option<bool>,
    #[serde(default)]
    pub remitente: Option<String>,
}

pub async fn guardar(user: AuthVendedor, body: web::Json<CorreoBody>, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    exigir_admin(&user)?;
    let b = body.into_inner();
    let input = CorreoInput {
        proveedor: b.proveedor,
        resend_api_key: b.resend_api_key,
        smtp_host: b.smtp_host,
        smtp_port: b.smtp_port.unwrap_or(587),
        smtp_usuario: b.smtp_usuario,
        smtp_password: b.smtp_password,
        smtp_tls: b.smtp_tls.unwrap_or(true),
        remitente: b.remitente,
    };
    db::run(&pool, move |conn| correo_services::set(conn, input)).await?;
    Ok(HttpResponse::Ok().json(json!({ "ok": true })))
}

#[derive(Deserialize)]
pub struct TestBody {
    pub destino: String,
}

pub async fn probar(user: AuthVendedor, body: web::Json<TestBody>, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    exigir_admin(&user)?;
    let destino = body.into_inner().destino.trim().to_string();
    if destino.is_empty() {
        return Err(ApiError::bad_request("Indica un correo de destino para la prueba."));
    }
    let datos = db::run(&pool, correo_services::datos).await?.ok_or_else(|| ApiError::bad_request("Primero guarda la configuración de correo."))?;
    let html = "<div style=\"font-family:system-ui,sans-serif;color:#1f2733\"><h2 style=\"color:#13233b\">✅ Correo de prueba · MeliHub</h2><p>Si recibes esto, tu configuración de correo funciona correctamente.</p></div>";
    crate::email::enviar(&datos, &destino, "Prueba de correo · MeliHub", html).await?;
    Ok(HttpResponse::Ok().json(json!({ "ok": true })))
}
