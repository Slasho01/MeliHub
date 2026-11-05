//! Equipos: miembros adicionales de una organización (el vendedor dueño) con un
//! rol (operador | lectura). El dueño no está en esta tabla (es el vendedor).

use crate::errors::{ApiError, ApiResult};
use crate::models::{Miembro, NewMiembro};
use crate::schema::{miembros, vendedores};
use crate::services::common::now;
use chrono::{Duration, NaiveDateTime};
use diesel::prelude::*;
use rand::Rng;
use uuid::Uuid;

const ROLES: [&str; 2] = ["operador", "lectura"];

fn validar_rol(rol: &str) -> ApiResult<()> {
    if ROLES.contains(&rol) {
        Ok(())
    } else {
        Err(ApiError::bad_request("Rol inválido (operador | lectura)"))
    }
}

fn token_random() -> String {
    let mut rng = rand::thread_rng();
    (0..48).map(|_| format!("{:x}", rng.gen_range(0..16))).collect()
}

/// Crea una invitación de miembro. Devuelve (miembro, token para el enlace del correo).
pub fn invitar(conn: &mut PgConnection, org_id: Uuid, nombre: &str, email: &str, rol: &str) -> ApiResult<(Miembro, String)> {
    validar_rol(rol)?;
    let nombre = nombre.trim();
    if nombre.is_empty() || nombre.len() > 255 {
        return Err(ApiError::bad_request("El nombre es obligatorio"));
    }
    let email = email.trim().to_lowercase();
    if !email.contains('@') || email.len() > 255 {
        return Err(ApiError::bad_request("Email inválido"));
    }
    // El email no puede ser el de un vendedor (dueño), para no romper el login.
    let existe: i64 = vendedores::table.filter(vendedores::email.eq(&email)).count().get_result(conn)?;
    if existe > 0 {
        return Err(ApiError::conflict("Ese email ya tiene su propia cuenta en MeliHub"));
    }
    let token = token_random();
    let expira = now() + Duration::days(7);
    let m = diesel::insert_into(miembros::table)
        .values(NewMiembro {
            vendedor_id: org_id,
            nombre: nombre.to_string(),
            email,
            rol: rol.to_string(),
            estado: "invitado".to_string(),
            invite_token: Some(token.clone()),
            invite_expira: Some(expira),
        })
        .returning(Miembro::as_returning())
        .get_result(conn)
        .map_err(|e| match ApiError::from(e) {
            ApiError::Conflict(_) => ApiError::conflict("Ese email ya fue invitado"),
            other => other,
        })?;
    Ok((m, token))
}

/// Acepta una invitación: fija la contraseña y activa al miembro.
pub fn aceptar(conn: &mut PgConnection, token: &str, password: &str) -> ApiResult<()> {
    crate::services::auth_services::validar_password(password)?;
    let m: Option<Miembro> = miembros::table
        .filter(miembros::invite_token.eq(token))
        .select(Miembro::as_select())
        .first(conn)
        .optional()?;
    let m = m.ok_or_else(|| ApiError::bad_request("Invitación inválida o ya utilizada"))?;
    if let Some(exp) = m.invite_expira {
        if exp < now() {
            return Err(ApiError::bad_request("La invitación expiró; pide una nueva"));
        }
    }
    let hash = bcrypt::hash(password, bcrypt::DEFAULT_COST)
        .map_err(|e| ApiError::internal(format!("No se pudo cifrar la contraseña: {e}")))?;
    diesel::update(miembros::table.find(m.id))
        .set((
            miembros::password_hash.eq(hash),
            miembros::estado.eq("activo"),
            miembros::invite_token.eq::<Option<String>>(None),
            miembros::invite_expira.eq::<Option<NaiveDateTime>>(None),
            miembros::updated_at.eq(now()),
        ))
        .execute(conn)?;
    Ok(())
}

pub fn listar(conn: &mut PgConnection, org_id: Uuid) -> ApiResult<Vec<Miembro>> {
    miembros::table
        .filter(miembros::vendedor_id.eq(org_id))
        .order(miembros::created_at.asc())
        .select(Miembro::as_select())
        .load(conn)
        .map_err(ApiError::from)
}

pub fn cambiar_rol(conn: &mut PgConnection, org_id: Uuid, miembro_id: Uuid, rol: &str) -> ApiResult<()> {
    validar_rol(rol)?;
    let n = diesel::update(miembros::table.filter(miembros::id.eq(miembro_id)).filter(miembros::vendedor_id.eq(org_id)))
        .set((miembros::rol.eq(rol), miembros::updated_at.eq(now())))
        .execute(conn)?;
    if n == 0 {
        return Err(ApiError::not_found("Miembro"));
    }
    Ok(())
}

pub fn eliminar(conn: &mut PgConnection, org_id: Uuid, miembro_id: Uuid) -> ApiResult<()> {
    let n = diesel::delete(miembros::table.filter(miembros::id.eq(miembro_id)).filter(miembros::vendedor_id.eq(org_id))).execute(conn)?;
    if n == 0 {
        return Err(ApiError::not_found("Miembro"));
    }
    Ok(())
}

/// Reemite el token de invitación de un miembro aún pendiente (para reenviar el correo).
pub fn reinvitar(conn: &mut PgConnection, org_id: Uuid, miembro_id: Uuid) -> ApiResult<(Miembro, String)> {
    let token = token_random();
    let expira = now() + Duration::days(7);
    let m: Miembro = diesel::update(
        miembros::table
            .filter(miembros::id.eq(miembro_id))
            .filter(miembros::vendedor_id.eq(org_id))
            .filter(miembros::estado.eq("invitado")),
    )
    .set((miembros::invite_token.eq(&token), miembros::invite_expira.eq(expira), miembros::updated_at.eq(now())))
    .returning(Miembro::as_returning())
    .get_result(conn)
    .optional()?
    .ok_or_else(|| ApiError::not_found("Invitación pendiente"))?;
    Ok((m, token))
}
