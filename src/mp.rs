//! Cliente de Mercado Pago para **suscripciones** (preapproval). El dueño de
//! MeliHub cobra con su `MP_ACCESS_TOKEN`. Contrato aislado aquí.

use crate::errors::{ApiError, ApiResult};
use serde::{Deserialize, Serialize};

/// Un cobro de la suscripción (para el historial de pagos).
#[derive(Serialize, Debug, Clone)]
pub struct Pago {
    pub id: String,
    pub estado: String,
    pub fecha: String,
    pub monto: i64,
}

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
        free_trial_dias: Option<i64>,
    ) -> ApiResult<Preapproval> {
        let mut auto_recurring = serde_json::json!({
            "frequency": frequency,
            "frequency_type": frequency_type,
            "transaction_amount": monto,
            "currency_id": moneda
        });
        if let Some(d) = free_trial_dias {
            if d > 0 {
                auto_recurring["free_trial"] = serde_json::json!({ "frequency": d, "frequency_type": "days" });
            }
        }
        let body = serde_json::json!({
            "reason": reason,
            "auto_recurring": auto_recurring,
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

    /// Lista los cobros autorizados de una suscripción (historial de pagos). Best-effort.
    pub async fn authorized_payments(&self, preapproval_id: &str) -> ApiResult<Vec<Pago>> {
        let res = self
            .http
            .get(format!("{}/authorized_payments/search?preapproval_id={preapproval_id}", self.base))
            .bearer_auth(&self.access_token)
            .send()
            .await
            .map_err(|e| ApiError::internal(format!("Mercado Pago no respondió: {e}")))?;
        if !res.status().is_success() {
            return Ok(vec![]);
        }
        let v: serde_json::Value = res.json().await.unwrap_or(serde_json::Value::Null);
        let results = v.get("results").and_then(|r| r.as_array()).cloned().unwrap_or_default();
        let mut out = Vec::new();
        for r in results {
            let id = r.get("id").map(|x| x.to_string().trim_matches('"').to_string()).unwrap_or_default();
            let estado = r.get("status").and_then(|s| s.as_str()).unwrap_or("").to_string();
            let fecha = r
                .get("date_created")
                .or_else(|| r.get("debit_date"))
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string();
            let monto = r.get("transaction_amount").and_then(|a| a.as_f64()).unwrap_or(0.0).round() as i64;
            out.push(Pago { id, estado, fecha, monto });
        }
        Ok(out)
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
