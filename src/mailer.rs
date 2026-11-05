//! Envío de correos por HTTP con **Resend** (Render bloquea SMTP). Si no está
//! configurado (`RESEND_API_KEY` vacío), `enviar` devuelve error y los flujos
//! que lo usan lo tratan como best-effort.

use crate::errors::{ApiError, ApiResult};

#[derive(Clone)]
pub struct Mailer {
    http: reqwest::Client,
    api_key: String,
    from: String,
}

impl Mailer {
    pub fn new(api_key: String, from: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            api_key,
            from,
        }
    }

    pub fn configurado(&self) -> bool {
        !self.api_key.trim().is_empty()
    }

    /// Envía un correo HTML. Devuelve error si Resend no está configurado o falla.
    pub async fn enviar(&self, to: &str, subject: &str, html: &str) -> ApiResult<()> {
        if !self.configurado() {
            return Err(ApiError::internal("El envío de correos no está configurado (RESEND_API_KEY)."));
        }
        let body = serde_json::json!({
            "from": self.from,
            "to": [to],
            "subject": subject,
            "html": html
        });
        let res = self
            .http
            .post("https://api.resend.com/emails")
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| ApiError::internal(format!("Resend no respondió: {e}")))?;
        if !res.status().is_success() {
            let t = res.text().await.unwrap_or_default();
            return Err(ApiError::internal(format!("Resend rechazó el envío: {t}")));
        }
        Ok(())
    }
}
