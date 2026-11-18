//! Verificación de Cloudflare Turnstile (CAPTCHA) para registro y login.
//! Si el secret está vacío, el CAPTCHA queda deshabilitado (devuelve Ok siempre).

use crate::errors::{ApiError, ApiResult};

const VERIFY_URL: &str = "https://challenges.cloudflare.com/turnstile/v0/siteverify";

/// Valida el token de Turnstile contra Cloudflare. `Ok(())` si pasa (o si el CAPTCHA
/// está deshabilitado), `Err(bad_request)` si falla o falta el token.
pub async fn verificar(secret: &str, token: &str, ip: &str) -> ApiResult<()> {
    if secret.trim().is_empty() {
        return Ok(()); // CAPTCHA no configurado -> no bloquea
    }
    if token.trim().is_empty() {
        return Err(ApiError::bad_request("Completa el CAPTCHA para continuar."));
    }
    let resp = reqwest::Client::new()
        .post(VERIFY_URL)
        .form(&[("secret", secret), ("response", token), ("remoteip", ip)])
        .send()
        .await
        .map_err(|_| ApiError::internal("No se pudo validar el CAPTCHA (red)."))?;
    let ok = resp
        .json::<serde_json::Value>()
        .await
        .ok()
        .and_then(|v| v.get("success").and_then(|s| s.as_bool()))
        .unwrap_or(false);
    if ok {
        Ok(())
    } else {
        Err(ApiError::bad_request("CAPTCHA inválido. Inténtalo de nuevo."))
    }
}
