//! Cliente de Mercado Pago para **suscripciones** (preapproval). El dueño de
//! MeliHub cobra con su `MP_ACCESS_TOKEN`. Contrato aislado aquí.

use crate::errors::{ApiError, ApiResult};
use serde::Deserialize;

#[derive(Clone)]
pub struct MercadoPago {
    http: reqwest::Client,
    access_token: String,
    base: String,
}

/// Suscripción (preapproval) de Mercado Pago.
#[derive(Deserialize, Debug, Clone)]
pub struct Preapproval {
    pub id: String,
    #[serde(default)]
    pub init_point: Option<String>,
    #[serde(default)]
    pub status: Option<String>, // pending | authorized | paused | cancelled
    #[serde(default)]
    pub external_reference: Option<String>,
}

impl MercadoPago {
    pub fn new(access_token: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            access_token,
            base: "https://api.mercadopago.com".to_string(),
        }
    }

    pub fn configurado(&self) -> bool {
        !self.access_token.trim().is_empty()
    }

    /// Crea una suscripción mensual y devuelve el `init_point` (URL de checkout).
    #[allow(clippy::too_many_arguments)]
    pub async fn crear_preapproval(
        &self,
        reason: &str,
        monto: i64,
        moneda: &str,
        payer_email: &str,
        back_url: &str,
        notification_url: &str,
        external_reference: &str,
        frequency: i64,
        frequency_type: &str,
    ) -> ApiResult<Preapproval> {
        let body = serde_json::json!({
            "reason": reason,
            "auto_recurring": {
                "frequency": frequency,
                "frequency_type": frequency_type,
                "transaction_amount": monto,
                "currency_id": moneda
            },
            "back_url": back_url,
            "notification_url": notification_url,
            "payer_email": payer_email,
            "external_reference": external_reference,
            "status": "pending"
        });
        let res = self
            .http
            .post(format!("{}/preapproval", self.base))
            .bearer_auth(&self.access_token)
            .json(&body)
            .send()
            .await
            .map_err(|e| ApiError::internal(format!("Mercado Pago no respondió: {e}")))?;
        let status = res.status();
        let text = res.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(ApiError::bad_request(format!("Mercado Pago rechazó la suscripción ({status}): {text}")));
        }
        serde_json::from_str(&text).map_err(|e| ApiError::internal(format!("Respuesta de Mercado Pago no reconocida: {e}")))
    }

    /// Estado actual de una suscripción.
    pub async fn get_preapproval(&self, id: &str) -> ApiResult<Preapproval> {
        let res = self
            .http
            .get(format!("{}/preapproval/{id}", self.base))
            .bearer_auth(&self.access_token)
            .send()
            .await
            .map_err(|e| ApiError::internal(format!("Mercado Pago no respondió: {e}")))?;
        let status = res.status();
        let text = res.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(ApiError::bad_request(format!("Mercado Pago ({status}): {text}")));
        }
        serde_json::from_str(&text).map_err(|e| ApiError::internal(format!("Respuesta de Mercado Pago no reconocida: {e}")))
    }

    /// Cancela una suscripción.
    pub async fn cancel_preapproval(&self, id: &str) -> ApiResult<()> {
        let res = self
            .http
            .put(format!("{}/preapproval/{id}", self.base))
            .bearer_auth(&self.access_token)
            .json(&serde_json::json!({ "status": "cancelled" }))
            .send()
            .await
            .map_err(|e| ApiError::internal(format!("Mercado Pago no respondió: {e}")))?;
        if !res.status().is_success() {
            let t = res.text().await.unwrap_or_default();
            return Err(ApiError::bad_request(format!("Mercado Pago no pudo cancelar: {t}")));
        }
        Ok(())
    }
}
