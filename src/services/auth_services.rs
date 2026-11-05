use crate::errors::{ApiError, ApiResult, OrNotFound};
use crate::models::{NewVendedor, Vendedor};
use crate::schema::vendedores;
use diesel::prelude::*;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct RegistroRequest {
    pub email: String,
    pub password: String,
    pub nombre: String,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

fn validar_password(p: &str) -> ApiResult<()> {
    if p.len() < 8 {
        return Err(ApiError::bad_request("La contraseña debe tener al menos 8 caracteres"));
    }
    Ok(())
}

fn normalizar_email(e: &str) -> ApiResult<String> {
    let e = e.trim().to_lowercase();
    if !e.contains('@') || e.len() > 255 {
        return Err(ApiError::bad_request("Email inválido"));
    }
    Ok(e)
}

pub fn registrar(conn: &mut PgConnection, req: RegistroRequest) -> ApiResult<Vendedor> {
    let email = normalizar_email(&req.email)?;
    validar_password(&req.password)?;
    let nombre = req.nombre.trim();
    if nombre.is_empty() || nombre.len() > 255 {
        return Err(ApiError::bad_request("El nombre es obligatorio"));
    }
    let hash = bcrypt::hash(&req.password, bcrypt::DEFAULT_COST)
        .map_err(|e| ApiError::internal(format!("No se pudo cifrar la contraseña: {e}")))?;

    // El primer vendedor registrado es admin del hub y queda en Business; el
    // resto arranca en Free (y puede mejorar su plan suscribiéndose).
    let total: i64 = vendedores::table.count().get_result(conn)?;
    let (rol, plan) = if total == 0 { ("admin", "business") } else { ("vendedor", "free") };

    diesel::insert_into(vendedores::table)
        .values(NewVendedor { email, password_hash: hash, nombre: nombre.to_string(), rol: rol.to_string(), plan: plan.to_string() })
        .returning(Vendedor::as_returning())
        .get_result(conn)
        .map_err(|e| match ApiError::from(e) {
            ApiError::Conflict(_) => ApiError::conflict("Ya existe una cuenta con ese email"),
            other => other,
        })
}

pub fn login(conn: &mut PgConnection, req: LoginRequest) -> ApiResult<Vendedor> {
    let email = req.email.trim().to_lowercase();
    let v: Option<Vendedor> = vendedores::table
        .filter(vendedores::email.eq(&email))
        .select(Vendedor::as_select())
        .first(conn)
        .optional()?;
    let v = v.ok_or_else(|| ApiError::unauthorized("Credenciales inválidas"))?;
    let ok = bcrypt::verify(&req.password, &v.password_hash).unwrap_or(false);
    if !ok {
        return Err(ApiError::unauthorized("Credenciales inválidas"));
    }
    if !v.activo {
        return Err(ApiError::forbidden("Tu cuenta está desactivada"));
    }
    Ok(v)
}

pub fn get(conn: &mut PgConnection, id: Uuid) -> ApiResult<Vendedor> {
    vendedores::table
        .find(id)
        .select(Vendedor::as_select())
        .first(conn)
        .optional()?
        .or_not_found("Vendedor")
}

#[derive(Deserialize)]
pub struct PerfilUpdate {
    pub nombre: Option<String>,
    pub email: Option<String>,
}

/// Actualiza nombre y/o email del vendedor autenticado.
pub fn actualizar_perfil(conn: &mut PgConnection, id: Uuid, req: PerfilUpdate) -> ApiResult<Vendedor> {
    let v = get(conn, id)?;
    let nombre = match req.nombre {
        Some(n) => {
            let n = n.trim().to_string();
            if n.is_empty() || n.len() > 255 {
                return Err(ApiError::bad_request("El nombre es obligatorio"));
            }
            n
        }
        None => v.nombre,
    };
    let email = match req.email {
        Some(e) => normalizar_email(&e)?,
        None => v.email,
    };
    diesel::update(vendedores::table.find(id))
        .set((
            vendedores::nombre.eq(&nombre),
            vendedores::email.eq(&email),
            vendedores::updated_at.eq(crate::services::common::now()),
        ))
        .returning(Vendedor::as_returning())
        .get_result(conn)
        .map_err(|e| match ApiError::from(e) {
            ApiError::Conflict(_) => ApiError::conflict("Ya existe una cuenta con ese email"),
            other => other,
        })
}

#[derive(Deserialize)]
pub struct PasswordUpdate {
    pub actual: String,
    pub nueva: String,
}

/// Cambia la contraseña verificando la actual.
pub fn cambiar_password(conn: &mut PgConnection, id: Uuid, req: PasswordUpdate) -> ApiResult<()> {
    let v = get(conn, id)?;
    if !bcrypt::verify(&req.actual, &v.password_hash).unwrap_or(false) {
        return Err(ApiError::unauthorized("La contraseña actual no es correcta"));
    }
    validar_password(&req.nueva)?;
    let hash = bcrypt::hash(&req.nueva, bcrypt::DEFAULT_COST)
        .map_err(|e| ApiError::internal(format!("No se pudo cifrar la contraseña: {e}")))?;
    diesel::update(vendedores::table.find(id))
        .set((vendedores::password_hash.eq(hash), vendedores::updated_at.eq(crate::services::common::now())))
        .execute(conn)?;
    Ok(())
}

/// Busca un vendedor por email (para recuperar contraseña). None si no existe.
pub fn buscar_por_email(conn: &mut PgConnection, email: &str) -> ApiResult<Option<Vendedor>> {
    let email = email.trim().to_lowercase();
    vendedores::table
        .filter(vendedores::email.eq(&email))
        .select(Vendedor::as_select())
        .first(conn)
        .optional()
        .map_err(ApiError::from)
}

/// Fija una nueva contraseña directamente (usado tras verificar el token de reseteo).
pub fn set_password(conn: &mut PgConnection, id: Uuid, nueva: &str) -> ApiResult<()> {
    validar_password(nueva)?;
    let hash = bcrypt::hash(nueva, bcrypt::DEFAULT_COST)
        .map_err(|e| ApiError::internal(format!("No se pudo cifrar la contraseña: {e}")))?;
    diesel::update(vendedores::table.find(id))
        .set((vendedores::password_hash.eq(hash), vendedores::updated_at.eq(crate::services::common::now())))
        .execute(conn)?;
    Ok(())
}
