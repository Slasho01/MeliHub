//! Cliente mínimo de LibreDTE para emitir DTE (boleta 39 / factura 33).
//!
//! Flujo hosted de LibreDTE: 1) emitir DTE **temporal** → 2) **generar** el DTE real (timbrado
//! con folio/CAF). Autenticación HTTP Basic con el token (hash) del usuario como "usuario".
//!
//! NOTA: el formato exacto del payload/endpoints de LibreDTE se valida con un token real; la
//! lógica está aislada aquí para ajustarla sin tocar el resto.

use crate::errors::{ApiError, ApiResult};
use serde_json::{json, Value};

const BASE: &str = "https://libredte.cl/api";

pub struct DteClient {
    http: reqwest::Client,
}

impl Default for DteClient {
    fn default() -> Self {
        Self::new()
    }
}

impl DteClient {
    pub fn new() -> Self {
        Self { http: reqwest::Client::new() }
    }

    async fn post(&self, token: &str, path: &str, body: &Value) -> ApiResult<Value> {
        let res = self
            .http
            .post(format!("{BASE}{path}"))
            .basic_auth(token, None::<&str>)
            .header("Accept", "application/json")
            .json(body)
            .send()
            .await
            .map_err(|e| ApiError::internal(format!("LibreDTE no respondió: {e}")))?;
        let status = res.status();
        let txt = res.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(ApiError::bad_request(format!("LibreDTE rechazó la operación ({status}): {txt}")));
        }
        serde_json::from_str(&txt).map_err(|e| ApiError::internal(format!("Respuesta de LibreDTE inválida: {e}")))
    }

    /// Emite un DTE: primero lo crea temporal, luego lo genera (real/timbrado).
    /// `documento` es el DTE en formato SII ({Encabezado, Detalle}).
    pub async fn emitir(&self, token: &str, documento: &Value, tipo: i32, emisor_rut: &str, receptor_rut: &str) -> ApiResult<Value> {
        // 1) DTE temporal.
        let tmp = self.post(token, "/dte/dte_tmps/emitir", documento).await?;
        let codigo = tmp
            .get("codigo")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ApiError::bad_request(format!("LibreDTE no devolvió el código temporal: {tmp}")))?;
        // 2) Generar el DTE real (LibreDTE usa el RUT sin dígito verificador).
        let body = json!({
            "emisor": rut_sin_dv(emisor_rut),
            "receptor": rut_sin_dv(receptor_rut),
            "dte": tipo,
            "codigo": codigo,
        });
        self.post(token, "/dte/dte_emitidos/generar", &body).await
    }

    /// URL del PDF de un DTE ya emitido.
    pub fn pdf_url(&self, emisor_rut: &str, tipo: i32, folio: i64) -> String {
        format!("{BASE}/dte/dte_emitidos/pdf/{tipo}/{folio}/{}", rut_sin_dv(emisor_rut))
    }
}

/// RUT sin puntos ni dígito verificador (LibreDTE lo pide así en varios endpoints).
fn rut_sin_dv(rut: &str) -> String {
    rut.replace('.', "").split('-').next().unwrap_or(rut).trim().to_string()
}

/// Receptor de un DTE (datos del cliente).
#[derive(serde::Deserialize, Default)]
pub struct Receptor {
    #[serde(default)]
    pub rut: String,
    #[serde(default)]
    pub razon_social: String,
    #[serde(default)]
    pub giro: String,
    #[serde(default)]
    pub direccion: String,
    #[serde(default)]
    pub comuna: String,
}

/// Una línea de la boleta/factura.
#[derive(serde::Deserialize)]
pub struct LineaDte {
    pub nombre: String,
    pub cantidad: f64,
    pub precio: i64,
}

/// Construye el documento SII ({Encabezado, Detalle}) a partir de los datos del cotizador.
pub fn construir_documento(tipo: i32, fecha: &str, emisor_rut: &str, receptor: &Receptor, detalle: &[LineaDte], descuento_pct: f64) -> Value {
    let rut_recep = if receptor.rut.trim().is_empty() { "66666666-6".to_string() } else { receptor.rut.trim().to_string() };
    let mut recep = serde_json::Map::new();
    recep.insert("RUTRecep".into(), json!(rut_recep));
    if !receptor.razon_social.trim().is_empty() {
        recep.insert("RznSocRecep".into(), json!(receptor.razon_social.trim()));
    }
    // La factura (33) exige más datos del receptor; la boleta (39) no.
    if tipo == 33 {
        if !receptor.giro.trim().is_empty() {
            recep.insert("GiroRecep".into(), json!(receptor.giro.trim()));
        }
        if !receptor.direccion.trim().is_empty() {
            recep.insert("DirRecep".into(), json!(receptor.direccion.trim()));
        }
        if !receptor.comuna.trim().is_empty() {
            recep.insert("CmnaRecep".into(), json!(receptor.comuna.trim()));
        }
    }
    let items: Vec<Value> = detalle
        .iter()
        .enumerate()
        .map(|(i, l)| {
            let mut it = serde_json::Map::new();
            it.insert("NroLinDet".into(), json!(i + 1));
            it.insert("NmbItem".into(), json!(l.nombre.trim()));
            it.insert("QtyItem".into(), json!(l.cantidad));
            it.insert("PrcItem".into(), json!(l.precio));
            if descuento_pct > 0.0 {
                it.insert("DescuentoPct".into(), json!(descuento_pct));
            }
            Value::Object(it)
        })
        .collect();
    json!({
        "Encabezado": {
            "IdDoc": { "TipoDTE": tipo, "FchEmis": fecha },
            "Emisor": { "RUTEmisor": emisor_rut },
            "Receptor": Value::Object(recep),
        },
        "Detalle": items,
    })
}
