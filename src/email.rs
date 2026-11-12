//! Envío de correos con dos proveedores: **Resend** (HTTP, funciona en Render) o **SMTP** propio
//! (lettre; requiere host sin bloqueo de SMTP saliente, p. ej. un VPS). La config vive en la BD.

use crate::errors::{ApiError, ApiResult};
use serde_json::json;

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
