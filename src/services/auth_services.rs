use crate::errors::{ApiError, ApiResult, OrNotFound};
use crate::models::{Miembro, NewVendedor, Vendedor};
use crate::schema::{miembros, vendedores};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Resultado de un login: identifica al actor y su organización/rol.
#[derive(Serialize)]
pub struct Sesion {
    pub org_id: Uuid,
    pub actor_id: Uuid,
    pub nombre: String,
    pub email: String,
    pub rol: String,        // súper-admin de plataforma: admin | vendedor
    pub rol_equipo: String, // owner | operador | lectura
    pub plan: String,
}

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

pub fn validar_password(p: &str) -> ApiResult<()> {
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

pub fn login(conn: &mut PgConnection, req: LoginRequest) -> ApiResult<Sesion> {
    let email = req.email.trim().to_lowercase();

    // 1) ¿Es un vendedor (dueño de la organización)?
    let v: Option<Vendedor> = vendedores::table
        .filter(vendedores::email.eq(&email))
        .select(Vendedor::as_select())
        .first(conn)
        .optional()?;
    if let Some(v) = v {
        if !bcrypt::verify(&req.password, &v.password_hash).unwrap_or(false) {
            return Err(ApiError::unauthorized("Credenciales inválidas"));
        }
        if !v.activo {
            return Err(ApiError::forbidden("Tu cuenta está desactivada"));
        }
        return Ok(Sesion {
            org_id: v.id,
            actor_id: v.id,
            nombre: v.nombre,
            email: v.email,
            rol: v.rol,
            rol_equipo: "owner".to_string(),
            plan: v.plan,
        });
    }

    // 2) ¿Es un miembro del equipo (invitación aceptada)?
    let m: Option<Miembro> = miembros::table
        .filter(miembros::email.eq(&email)) // el email se guarda en minúsculas al invitar
        .filter(miembros::estado.eq("activo"))
        .select(Miembro::as_select())
        .first(conn)
        .optional()?;
    if let Some(m) = m {
        let hash = m.password_hash.as_deref().unwrap_or("");
        if hash.is_empty() || !bcrypt::verify(&req.password, hash).unwrap_or(false) {
            return Err(ApiError::unauthorized("Credenciales inválidas"));
        }
        let org: Vendedor = vendedores::table.find(m.vendedor_id).select(Vendedor::as_select()).first(conn)?;
        if !org.activo {
            return Err(ApiError::forbidden("La cuenta de tu organización está desactivada"));
        }
        return Ok(Sesion {
            org_id: org.id,
            actor_id: m.id,
            nombre: m.nombre,
            email: m.email,
            rol: "vendedor".to_string(),
            rol_equipo: m.rol,
            plan: org.plan,
        });
    }

    Err(ApiError::unauthorized("Credenciales inválidas"))
}

pub fn get(conn: &mut PgConnection, id: Uuid) -> ApiResult<Vendedor> {
    vendedores::table
        .find(id)
        .select(Vendedor::as_select())
        .first(conn)
        .optional()?
        .or_not_found("Vendedor")
}

/// Identidad del actor (dueño o miembro) + plan de su organización — para /auth/me y perfil.
#[derive(Serialize)]
pub struct Identidad {
    pub nombre: String,
    pub email: String,
    pub plan: String,
}

pub fn identidad(conn: &mut PgConnection, org_id: Uuid, actor_id: Uuid) -> ApiResult<Identidad> {
    let plan: String = vendedores::table.find(org_id).select(vendedores::plan).first(conn)?;
    if actor_id == org_id {
        let v = get(conn, org_id)?;
        Ok(Identidad { nombre: v.nombre, email: v.email, plan })
    } else {
        let m: Miembro = miembros::table.find(actor_id).select(Miembro::as_select()).first(conn)?;
        Ok(Identidad { nombre: m.nombre, email: m.email, plan })
    }
}

#[derive(Deserialize)]
pub struct PerfilUpdate {
    pub nombre: Option<String>,
    pub email: Option<String>,
}

/// Actualiza nombre y/o email del actor (su propia ficha: dueño en `vendedores`, miembro en `miembros`).
pub fn actualizar_perfil(conn: &mut PgConnection, org_id: Uuid, actor_id: Uuid, req: PerfilUpdate) -> ApiResult<Identidad> {
    let ident = identidad(conn, org_id, actor_id)?;
    let nombre = match req.nombre {
        Some(n) => {
            let n = n.trim().to_string();
            if n.is_empty() || n.len() > 255 {
                return Err(ApiError::bad_request("El nombre es obligatorio"));
            }
            n
        }
        None => ident.nombre,
    };
    let email = match req.email {
        Some(e) => normalizar_email(&e)?,
        None => ident.email,
    };
    let dup = |e: ApiError| match e {
        ApiError::Conflict(_) => ApiError::conflict("Ya existe una cuenta con ese email"),
        other => other,
    };
    if actor_id == org_id {
        diesel::update(vendedores::table.find(org_id))
            .set((vendedores::nombre.eq(&nombre), vendedores::email.eq(&email), vendedores::updated_at.eq(crate::services::common::now())))
            .execute(conn)
            .map_err(|e| dup(ApiError::from(e)))?;
    } else {
        diesel::update(miembros::table.find(actor_id))
            .set((miembros::nombre.eq(&nombre), miembros::email.eq(&email), miembros::updated_at.eq(crate::services::common::now())))
            .execute(conn)
            .map_err(|e| dup(ApiError::from(e)))?;
    }
    Ok(Identidad { nombre, email, plan: ident.plan })
}

#[derive(Deserialize)]
pub struct PasswordUpdate {
    pub actual: String,
    pub nueva: String,
}

/// Cambia la contraseña del actor verificando la actual.
pub fn cambiar_password(conn: &mut PgConnection, org_id: Uuid, actor_id: Uuid, req: PasswordUpdate) -> ApiResult<()> {
    validar_password(&req.nueva)?;
    let hash = bcrypt::hash(&req.nueva, bcrypt::DEFAULT_COST)
        .map_err(|e| ApiError::internal(format!("No se pudo cifrar la contraseña: {e}")))?;
    if actor_id == org_id {
        let v = get(conn, org_id)?;
        if !bcrypt::verify(&req.actual, &v.password_hash).unwrap_or(false) {
            return Err(ApiError::unauthorized("La contraseña actual no es correcta"));
        }
        diesel::update(vendedores::table.find(org_id))
            .set((vendedores::password_hash.eq(hash), vendedores::updated_at.eq(crate::services::common::now())))
            .execute(conn)?;
    } else {
        let m: Miembro = miembros::table.find(actor_id).select(Miembro::as_select()).first(conn)?;
        let actual = m.password_hash.as_deref().unwrap_or("");
        if !bcrypt::verify(&req.actual, actual).unwrap_or(false) {
            return Err(ApiError::unauthorized("La contraseña actual no es correcta"));
        }
        diesel::update(miembros::table.find(actor_id))
            .set((miembros::password_hash.eq(hash), miembros::updated_at.eq(crate::services::common::now())))
            .execute(conn)?;
    }
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
