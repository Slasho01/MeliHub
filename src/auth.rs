use crate::config::Config;
use crate::errors::ApiError;
use actix_web::{dev::Payload, web, FromRequest, HttpRequest};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::future::{ready, Ready};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // vendedor id
    pub rol: String,
    pub exp: usize,
}

/// Emite un JWT para un vendedor (válido 7 días).
pub fn crear_token(vendedor_id: Uuid, rol: &str, secret: &str) -> Result<String, ApiError> {
    let exp = (chrono::Utc::now() + chrono::Duration::days(7)).timestamp() as usize;
    let claims = Claims { sub: vendedor_id.to_string(), rol: rol.to_string(), exp };
    encode(&Header::new(Algorithm::HS256), &claims, &EncodingKey::from_secret(secret.as_bytes()))
        .map_err(|e| ApiError::internal(format!("No se pudo emitir el token: {e}")))
}

/// Vendedor autenticado (extraído del header Authorization: Bearer).
#[derive(Clone)]
pub struct AuthVendedor {
    pub id: Uuid,
    pub rol: String,
}

impl AuthVendedor {
    pub fn es_admin(&self) -> bool {
        self.rol == "admin"
    }
}

impl FromRequest for AuthVendedor {
    type Error = ApiError;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        ready(extraer(req))
    }
}

fn extraer(req: &HttpRequest) -> Result<AuthVendedor, ApiError> {
    let secret = req
        .app_data::<web::Data<Config>>()
        .map(|c| c.jwt_secret.clone())
        .ok_or_else(|| ApiError::internal("Config no disponible"))?;
    let header = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| ApiError::unauthorized("Falta el token de autenticación"))?;
    let token = header
        .strip_prefix("Bearer ")
        .ok_or_else(|| ApiError::unauthorized("Formato de token inválido"))?;
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::new(Algorithm::HS256),
    )
    .map_err(|_| ApiError::unauthorized("Token inválido o expirado"))?;
    let id = Uuid::parse_str(&data.claims.sub).map_err(|_| ApiError::unauthorized("Token inválido"))?;
    Ok(AuthVendedor { id, rol: data.claims.rol })
}
