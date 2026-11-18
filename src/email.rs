//! Envío de correos con dos proveedores: **Resend** (HTTP, funciona en Render) o **SMTP** propio
//! (lettre; requiere host sin bloqueo de SMTP saliente, p. ej. un VPS). La config vive en la BD.

use crate::db::{self, DbPool};
use crate::errors::{ApiError, ApiResult};
use actix_web::web;
use serde_json::json;

/// Envuelve el contenido en una plantilla HTML con la marca Ventoki (header violeta + footer).
/// Estilos inline + layout simple para máxima compatibilidad con clientes de correo.
pub fn plantilla(contenido: &str) -> String {
    format!(
        "<!doctype html><html><body style=\"margin:0;padding:0;background:#f4f5f8\">\
         <div style=\"background:#f4f5f8;padding:32px 16px;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Arial,sans-serif;\">\
           <div style=\"max-width:480px;margin:0 auto;background:#ffffff;border-radius:16px;overflow:hidden;border:1px solid #e8ebf2;box-shadow:0 6px 24px rgba(25,20,60,.06)\">\
             <div style=\"background:#5B4FE0;padding:26px 24px;text-align:center\">\
               <span style=\"color:#ffffff;font-size:23px;font-weight:800;letter-spacing:-.6px\">Ventoki</span>\
             </div>\
             <div style=\"padding:30px 30px 10px;color:#1f2733;font-size:15px;line-height:1.6\">{contenido}</div>\
             <div style=\"padding:18px 30px 26px;color:#9aa0b0;font-size:12px;line-height:1.5;text-align:center;border-top:1px solid #f0f2f6;margin-top:14px\">\
               Ventoki · Hub multicanal para Mercado Libre y tu e-commerce<br>No afiliado a Mercado Libre.\
             </div>\
           </div>\
         </div></body></html>"
    )
}

/// Botón de acción con la marca (violeta). Centrado.
pub fn boton(texto: &str, url: &str) -> String {
    format!(
        "<div style=\"text-align:center;margin:24px 0\">\
         <a href=\"{url}\" style=\"display:inline-block;background:#5B4FE0;color:#ffffff;padding:13px 28px;border-radius:10px;text-decoration:none;font-weight:700;font-size:15px\">{texto}</a>\
         </div>"
    )
}

/// Datos de correo ya descifrados, listos para enviar.
pub struct DatosCorreo {
    pub proveedor: String,
    pub resend_api_key: String,
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_usuario: String,
    pub smtp_password: String,
    pub smtp_tls: bool,
    pub remitente: String,
}

impl DatosCorreo {
    /// ¿Tiene lo mínimo para enviar? (Resend con key, o SMTP con host.)
    pub fn usable(&self) -> bool {
        if self.proveedor == "smtp" {
            !self.smtp_host.trim().is_empty()
        } else {
            !self.resend_api_key.trim().is_empty()
        }
    }
    /// Config de Resend tomada del entorno (fallback si el panel no está configurado).
    pub fn desde_env(resend_key: &str, remitente: &str) -> Self {
        DatosCorreo {
            proveedor: "resend".to_string(),
            resend_api_key: resend_key.to_string(),
            remitente: remitente.to_string(),
            smtp_host: String::new(),
            smtp_port: 587,
            smtp_usuario: String::new(),
            smtp_password: String::new(),
            smtp_tls: true,
        }
    }
}

/// Resuelve la configuración de correo: primero la del panel admin (BD `config_correo`);
/// si no hay o no es usable, cae al entorno (`RESEND_API_KEY` / `MAIL_FROM`).
pub async fn resolver(pool: &web::Data<DbPool>, resend_env: &str, from_env: &str) -> Option<DatosCorreo> {
    if let Ok(Some(d)) = db::run(pool, crate::services::correo_services::datos).await {
        if d.usable() {
            return Some(d);
        }
    }
    let env = DatosCorreo::desde_env(resend_env, from_env);
    env.usable().then_some(env)
}

/// ¿Hay alguna configuración de correo activa (panel o entorno)?
pub async fn hay_config(pool: &web::Data<DbPool>, resend_env: &str, from_env: &str) -> bool {
    resolver(pool, resend_env, from_env).await.is_some()
}

/// Como `enviar_auto` pero leyendo el entorno por su cuenta (para contextos sin `Config` a
/// mano, p. ej. el job de fondo o `sync.rs`). Igual prioriza el panel admin (BD).
pub async fn enviar_sistema(pool: &web::Data<DbPool>, to: &str, subject: &str, html: &str) -> ApiResult<()> {
    let resend = std::env::var("RESEND_API_KEY").unwrap_or_default();
    let from = std::env::var("MAIL_FROM").unwrap_or_default();
    enviar_auto(pool, &resend, &from, to, subject, html).await
}

/// Envía un correo resolviendo la config automáticamente (panel admin o entorno).
pub async fn enviar_auto(pool: &web::Data<DbPool>, resend_env: &str, from_env: &str, to: &str, subject: &str, html: &str) -> ApiResult<()> {
    let d = resolver(pool, resend_env, from_env)
        .await
        .ok_or_else(|| ApiError::internal("El envío de correos no está configurado (configura Resend en el panel o define RESEND_API_KEY)."))?;
    enviar(&d, to, subject, html).await
}

/// Envía un correo HTML según el proveedor configurado.
pub async fn enviar(d: &DatosCorreo, to: &str, subject: &str, html: &str) -> ApiResult<()> {
    if d.remitente.trim().is_empty() {
        return Err(ApiError::bad_request("Falta el remitente del correo."));
    }
    if d.proveedor == "smtp" {
        enviar_smtp(d, to, subject, html).await
    } else {
        enviar_resend(d, to, subject, html).await
    }
}

async fn enviar_resend(d: &DatosCorreo, to: &str, subject: &str, html: &str) -> ApiResult<()> {
    if d.resend_api_key.trim().is_empty() {
        return Err(ApiError::bad_request("Falta la API key de Resend."));
    }
    let body = json!({ "from": d.remitente, "to": [to], "subject": subject, "html": html });
    let res = reqwest::Client::new()
        .post("https://api.resend.com/emails")
        .bearer_auth(&d.resend_api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| ApiError::internal(format!("Resend no respondió: {e}")))?;
    if !res.status().is_success() {
        let t = res.text().await.unwrap_or_default();
        return Err(ApiError::bad_request(format!("Resend rechazó el envío: {t}")));
    }
    Ok(())
}

async fn enviar_smtp(d: &DatosCorreo, to: &str, subject: &str, html: &str) -> ApiResult<()> {
    use lettre::message::header::ContentType;
    use lettre::transport::smtp::authentication::Credentials;
    use lettre::transport::smtp::client::{Tls, TlsParameters};
    use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

    if d.smtp_host.trim().is_empty() {
        return Err(ApiError::bad_request("Falta el host SMTP."));
    }
    let from: lettre::message::Mailbox = d.remitente.parse().map_err(|_| ApiError::bad_request("Remitente inválido (usa 'Nombre <correo@dominio>')."))?;
    let para: lettre::message::Mailbox = to.parse().map_err(|_| ApiError::bad_request("Destinatario inválido."))?;
    let msg = Message::builder()
        .from(from)
        .to(para)
        .subject(subject)
        .header(ContentType::TEXT_HTML)
        .body(html.to_string())
        .map_err(|e| ApiError::internal(format!("No se pudo armar el correo: {e}")))?;

    let creds = Credentials::new(d.smtp_usuario.clone(), d.smtp_password.clone());
    let mut builder = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(d.smtp_host.as_str()).port(d.smtp_port).credentials(creds);
    if d.smtp_tls {
        let tls = TlsParameters::new(d.smtp_host.clone()).map_err(|e| ApiError::internal(format!("TLS SMTP: {e}")))?;
        // 465 = TLS implícito (wrapper); 587/otros = STARTTLS requerido.
        builder = if d.smtp_port == 465 { builder.tls(Tls::Wrapper(tls)) } else { builder.tls(Tls::Required(tls)) };
    }
    let mailer = builder.build();
    mailer.send(msg).await.map_err(|e| ApiError::bad_request(format!("SMTP rechazó el envío: {e}")))?;
    Ok(())
}
