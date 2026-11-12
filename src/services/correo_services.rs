//! Configuración de correo (global, admin). Secretos cifrados; al guardar, si un secreto viene
//! vacío se conserva el anterior (para no borrarlo al editar el resto).

use crate::email::DatosCorreo;
use crate::errors::{ApiError, ApiResult};
use crate::models::{ConfigCorreo, NewConfigCorreo};
use crate::schema::config_correo;
use diesel::prelude::*;

pub fn get(conn: &mut PgConnection) -> ApiResult<Option<ConfigCorreo>> {
    config_correo::table.select(ConfigCorreo::as_select()).first(conn).optional().map_err(ApiError::from)
}

pub struct CorreoInput {
    pub proveedor: String,
    pub resend_api_key: Option<String>,
    pub smtp_host: Option<String>,
    pub smtp_port: i32,
    pub smtp_usuario: Option<String>,
    pub smtp_password: Option<String>,
    pub smtp_tls: bool,
    pub remitente: Option<String>,
}

fn limpio(s: Option<String>) -> Option<String> {
    s.map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
}

pub fn set(conn: &mut PgConnection, input: CorreoInput) -> ApiResult<ConfigCorreo> {
    let prev = get(conn)?;
    // Secreto: si viene con valor → cifrar; si viene vacío → conservar el anterior.
    let cifrar_o_prev = |nuevo: Option<String>, anterior: Option<String>| -> Option<String> {
        match limpio(nuevo) {
            Some(s) => Some(crate::crypto::cifrar(&s)),
            None => anterior,
        }
    };
    let resend = cifrar_o_prev(input.resend_api_key, prev.as_ref().and_then(|c| c.resend_api_key.clone()));
    let smtp_pass = cifrar_o_prev(input.smtp_password, prev.as_ref().and_then(|c| c.smtp_password.clone()));
    let nuevo = NewConfigCorreo {
        proveedor: if input.proveedor == "smtp" { "smtp".into() } else { "resend".into() },
        resend_api_key: resend,
        smtp_host: limpio(input.smtp_host),
        smtp_port: input.smtp_port.clamp(1, 65535),
        smtp_usuario: limpio(input.smtp_usuario),
        smtp_password: smtp_pass,
        smtp_tls: input.smtp_tls,
        remitente: limpio(input.remitente),
    };
    // Singleton: reemplazamos la fila.
    diesel::delete(config_correo::table).execute(conn)?;
    diesel::insert_into(config_correo::table).values(&nuevo).execute(conn)?;
    get(conn)?.ok_or_else(|| ApiError::internal("No se pudo guardar la configuración de correo"))
}

/// Datos descifrados listos para enviar (None si no hay configuración).
pub fn datos(conn: &mut PgConnection) -> ApiResult<Option<DatosCorreo>> {
    Ok(get(conn)?.map(|c| DatosCorreo {
        proveedor: c.proveedor,
        resend_api_key: c.resend_api_key.map(|s| crate::crypto::descifrar(&s)).unwrap_or_default(),
        smtp_host: c.smtp_host.unwrap_or_default(),
        smtp_port: c.smtp_port.clamp(1, 65535) as u16,
        smtp_usuario: c.smtp_usuario.unwrap_or_default(),
        smtp_password: c.smtp_password.map(|s| crate::crypto::descifrar(&s)).unwrap_or_default(),
        smtp_tls: c.smtp_tls,
        remitente: c.remitente.unwrap_or_default(),
    }))
}
