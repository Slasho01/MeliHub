use crate::errors::{ApiError, ApiResult, OrNotFound};
use crate::models::{ApiKey, NewApiKey};
use crate::schema::api_keys;
use crate::services::common::now;
use diesel::prelude::*;
use rand::Rng;
use sha2::{Digest, Sha256};
use serde::Deserialize;
use uuid::Uuid;

/// sha256 hex de la key en claro (solo guardamos el hash).
pub fn hash(key: &str) -> String {
    let mut h = Sha256::new();
    h.update(key.as_bytes());
    format!("{:x}", h.finalize())
}

#[derive(Deserialize)]
pub struct NuevaKey {
    pub nombre: String,
    #[serde(default = "canal_default")]
    pub canal: String,
    pub ml_cuenta_id: Option<Uuid>,
}
fn canal_default() -> String { "jarstore".into() }

/// Crea una API key. Devuelve (registro, key_en_claro) — la clave solo se ve aquí.
pub fn generar(conn: &mut PgConnection, vendedor_id: Uuid, req: NuevaKey) -> ApiResult<(ApiKey, String)> {
    crate::services::saas_services::permitir_plataforma(conn, vendedor_id)?; // límite del plan
    let nombre = req.nombre.trim();
    if nombre.is_empty() || nombre.len() > 100 {
        return Err(ApiError::bad_request("Dale un nombre a la API key"));
    }
    let canal = match req.canal.trim() {
        "mercadolibre" => "mercadolibre",
        _ => "jarstore",
    };
    let key: String = {
        let mut rng = rand::thread_rng();
        let hex: String = (0..40).map(|_| format!("{:x}", rng.gen_range(0..16))).collect();
        format!("mhk_{hex}")
    };
    let registro = diesel::insert_into(api_keys::table)
        .values(NewApiKey {
            vendedor_id,
            nombre: nombre.to_string(),
            token_hash: hash(&key),
            canal: canal.to_string(),
            ml_cuenta_id: req.ml_cuenta_id,
        })
        .returning(ApiKey::as_returning())
        .get_result(conn)?;
    Ok((registro, key))
}

pub fn listar(conn: &mut PgConnection, vendedor_id: Uuid) -> ApiResult<Vec<ApiKey>> {
    api_keys::table
        .filter(api_keys::vendedor_id.eq(vendedor_id))
        .select(ApiKey::as_select())
        .order(api_keys::created_at.desc())
        .load(conn)
        .map_err(ApiError::from)
}

pub fn revocar(conn: &mut PgConnection, id: Uuid, vendedor_id: Uuid, es_admin: bool) -> ApiResult<()> {
    let k: ApiKey = api_keys::table.find(id).select(ApiKey::as_select()).first(conn).optional()?.or_not_found("API key")?;
    if !es_admin && k.vendedor_id != vendedor_id {
        return Err(ApiError::forbidden("Esa API key no te pertenece"));
    }
    diesel::delete(api_keys::table.find(id)).execute(conn)?;
    Ok(())
}

/// Autentica una key en claro: devuelve el registro si está activa, y marca uso.
pub fn autenticar(conn: &mut PgConnection, key: &str) -> ApiResult<ApiKey> {
    let h = hash(key);
    let k: Option<ApiKey> = api_keys::table
        .filter(api_keys::token_hash.eq(&h).and(api_keys::activa.eq(true)))
        .select(ApiKey::as_select())
        .first(conn)
        .optional()?;
    let k = k.ok_or_else(|| ApiError::unauthorized("API key inválida"))?;
    let _ = diesel::update(api_keys::table.find(k.id)).set(api_keys::ultima_uso_at.eq(now())).execute(conn);
    Ok(k)
}
