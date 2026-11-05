use crate::config::Config;
use crate::errors::ApiError;
use actix_web::{dev::Payload, web, FromRequest, HttpRequest};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use uuid::Uuid;

fn rol_owner() -> String {
    "owner".to_string()
}

#[derive(Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // org id (= vendedor_id dueño) — igual que antes, para filtrar datos
    pub rol: String, // súper-admin de la plataforma: admin | vendedor
    #[serde(default)]
    pub aid: String, // actor id (miembro o dueño); vacío (tokens viejos) = el dueño (sub)
    #[serde(default = "rol_owner")]
    pub req: String, // rol de equipo: owner | operador | lectura
    pub exp: usize,
}

/// Emite un JWT (válido 7 días). `org_id` es la organización (vendedor dueño);
/// `actor_id` es quién inició sesión; `rol_equipo` su rol dentro de la org.
pub fn crear_token(org_id: Uuid, rol: &str, actor_id: Uuid, rol_equipo: &str, secret: &str) -> Result<String, ApiError> {
    let exp = (chrono::Utc::now() + chrono::Duration::days(7)).timestamp() as usize;
    let claims = Claims {
        sub: org_id.to_string(),
        rol: rol.to_string(),
        aid: actor_id.to_string(),
        req: rol_equipo.to_string(),
        exp,
    };
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
    pub id: Uuid,           // org id (vendedor dueño) — para filtrar datos
    pub actor_id: Uuid,     // quién inició sesión (dueño o miembro)
    pub rol: String,        // súper-admin de la plataforma: admin | vendedor
    pub rol_equipo: String, // owner | operador | lectura
}

impl AuthVendedor {
    /// Súper-admin de la plataforma (el dueño de MeliHub).
    pub fn es_admin(&self) -> bool {
        self.rol == "admin"
    }
    /// Dueño de la organización (o súper-admin).
    pub fn es_owner(&self) -> bool {
        self.rol_equipo == "owner" || self.es_admin()
    }
    /// Es un miembro invitado (no el dueño de la cuenta).
    pub fn es_miembro(&self) -> bool {
        self.actor_id != self.id
    }
    /// Puede modificar datos operativos (dueño u operador).
    pub fn puede_editar(&self) -> bool {
        self.es_owner() || self.rol_equipo == "operador"
    }
    /// Exige rol de dueño para acciones sensibles (cuentas ML, facturación, equipo, api-keys).
    pub fn exigir_owner(&self) -> Result<(), ApiError> {
        if self.es_owner() {
            Ok(())
        } else {
            Err(ApiError::forbidden("Solo el dueño de la cuenta puede hacer esto"))
        }
    }
    /// Exige permiso de edición (dueño u operador); rechaza a los de solo lectura.
    pub fn exigir_edicion(&self) -> Result<(), ApiError> {
        if self.puede_editar() {
            Ok(())
        } else {
            Err(ApiError::forbidden("Tu rol es de solo lectura"))
        }
    }
}

impl FromRequest for AuthVendedor {
    type Error = ApiError;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let req = req.clone();
        Box::pin(async move {
            let auth = extraer(&req)?;
            // El dueño no se revalida (sin costo extra). A los miembros sí los
            // revalidamos contra la BD: así la expulsión y los cambios de rol
            // surten efecto de inmediato (el JWT por sí solo no se puede revocar).
            if !auth.es_miembro() {
                return Ok(auth);
            }
            let pool = req
                .app_data::<web::Data<crate::db::DbPool>>()
                .ok_or_else(|| ApiError::internal("Pool no disponible"))?
                .clone();
            let (aid, org) = (auth.actor_id, auth.id);
            let rol_db: Option<String> = crate::db::run(&pool, move |conn| {
                use crate::schema::miembros::dsl as m;
                use diesel::prelude::*;
                m::miembros
                    .filter(m::id.eq(aid))
                    .filter(m::vendedor_id.eq(org))
                    .filter(m::estado.eq("activo"))
                    .select(m::rol)
                    .first::<String>(conn)
                    .optional()
                    .map_err(ApiError::from)
            })
            .await?;
            match rol_db {
                Some(rol) => Ok(AuthVendedor { rol_equipo: rol, ..auth }),
                None => Err(ApiError::unauthorized("Tu acceso fue revocado")),
            }
        })
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
    // Tokens antiguos no traen `aid`/`req`: por defecto el actor es el dueño.
    let actor_id = Uuid::parse_str(&data.claims.aid).unwrap_or(id);
    let rol_equipo = if data.claims.req.is_empty() { "owner".to_string() } else { data.claims.req };
    Ok(AuthVendedor { id, actor_id, rol: data.claims.rol, rol_equipo })
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
