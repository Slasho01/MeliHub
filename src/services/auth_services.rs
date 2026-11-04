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

    // El primer vendedor registrado es admin del hub.
    let total: i64 = vendedores::table.count().get_result(conn)?;
    let rol = if total == 0 { "admin" } else { "vendedor" };

    diesel::insert_into(vendedores::table)
        .values(NewVendedor { email, password_hash: hash, nombre: nombre.to_string(), rol: rol.to_string() })
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
