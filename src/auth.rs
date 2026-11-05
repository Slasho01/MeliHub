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

#[derive(Serialize, Deserialize)]
struct ResetClaims {
    sub: String,
    purpose: String, // "reset"
    exp: usize,
}

/// Token para resetear contraseña (válido 1 hora).
pub fn crear_reset_token(vendedor_id: Uuid, secret: &str) -> Result<String, ApiError> {
    let exp = (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp() as usize;
    let claims = ResetClaims { sub: vendedor_id.to_string(), purpose: "reset".to_string(), exp };
    encode(&Header::new(Algorithm::HS256), &claims, &EncodingKey::from_secret(secret.as_bytes()))
        .map_err(|e| ApiError::internal(format!("No se pudo emitir el token: {e}")))
}

/// Verifica un token de reseteo y devuelve el vendedor id.
pub fn verificar_reset_token(token: &str, secret: &str) -> Result<Uuid, ApiError> {
    let data = decode::<ResetClaims>(token, &DecodingKey::from_secret(secret.as_bytes()), &Validation::new(Algorithm::HS256))
        .map_err(|_| ApiError::unauthorized("Enlace inválido o expirado"))?;
    if data.claims.purpose != "reset" {
        return Err(ApiError::unauthorized("Token inválido"));
    }
    Uuid::parse_str(&data.claims.sub).map_err(|_| ApiError::unauthorized("Token inválido"))
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

// ----------------------------- Rate limiting de login -----------------------------

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

struct Intentos {
    count: u32,
    desde: Instant,
}

/// Limita los intentos de login por IP (ventana deslizante). En memoria.
pub struct LoginLimiter {
    inner: Mutex<HashMap<String, Intentos>>,
    max: u32,
    ventana: Duration,
}

impl Default for LoginLimiter {
    fn default() -> Self {
        Self::new()
    }
}

impl LoginLimiter {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
            max: 10,                              // 10 intentos fallidos
            ventana: Duration::from_secs(300),    // por cada 5 minutos
        }
    }

    /// `false` si la IP está bloqueada por exceso de intentos.
    pub fn permitido(&self, key: &str) -> bool {
        let m = self.inner.lock().unwrap();
        match m.get(key) {
            Some(i) if i.desde.elapsed() < self.ventana && i.count >= self.max => false,
            _ => true,
        }
    }

    /// Registra un intento fallido (reinicia el contador si la ventana venció).
    pub fn fallo(&self, key: &str) {
        let mut m = self.inner.lock().unwrap();
        // Limpieza oportunista de entradas viejas para no crecer sin límite.
        if m.len() > 10_000 {
            let v = self.ventana;
            m.retain(|_, i| i.desde.elapsed() < v);
        }
        let e = m.entry(key.to_string()).or_insert(Intentos { count: 0, desde: Instant::now() });
        if e.desde.elapsed() >= self.ventana {
            e.count = 0;
            e.desde = Instant::now();
        }
        e.count += 1;
    }

    /// Limpia el contador tras un login exitoso.
    pub fn exito(&self, key: &str) {
        self.inner.lock().unwrap().remove(key);
    }
}
