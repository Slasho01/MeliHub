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
    #[serde(skip_serializing)]
    pub sv: i32, // versión de sesión actual del actor
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
    #[serde(default)]
    pub codigo: Option<String>,
}

/// Resultado del login: sesión lista, o se requiere el código 2FA.
pub enum LoginOutcome {
    Ok(Box<Sesion>),
    TwoFaRequerido,
}

pub fn validar_password(p: &str) -> ApiResult<()> {
    if p.len() < 8 {
        return Err(ApiError::bad_request("La contraseña debe tener al menos 8 caracteres"));
    }
    if p.len() > 128 {
        return Err(ApiError::bad_request("La contraseña es demasiado larga (máx. 128)"));
    }
    let tiene_letra = p.chars().any(|c| c.is_alphabetic());
    let tiene_numero = p.chars().any(|c| c.is_ascii_digit());
    if !tiene_letra || !tiene_numero {
        return Err(ApiError::bad_request("La contraseña debe incluir al menos una letra y un número"));
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

fn sesion_vendedor(v: Vendedor) -> Sesion {
    Sesion {
        org_id: v.id,
        actor_id: v.id,
        nombre: v.nombre,
        email: v.email,
        rol: v.rol,
        rol_equipo: "owner".to_string(),
        plan: v.plan,
        sv: v.session_version,
    }
}

fn sesion_miembro(org: &Vendedor, m: Miembro) -> Sesion {
    Sesion {
        org_id: org.id,
        actor_id: m.id,
        nombre: m.nombre,
        email: m.email,
        rol: "vendedor".to_string(),
        rol_equipo: m.rol,
        plan: org.plan.clone(),
        sv: m.session_version,
    }
}

/// Completa el login tras un 2FA válido. Se llama SOLO con un token temporal de 2FA
/// (emitido por `login` después de validar la contraseña), así que aquí NO se revisa la
/// contraseña: solo se verifica el código TOTP. Evita reenviar email+password en el 2º paso.
pub fn completar_2fa(conn: &mut PgConnection, email: &str, codigo: &str) -> ApiResult<Box<Sesion>> {
    let email = email.trim().to_lowercase();
    let codigo = codigo.trim();

    if let Some(v) = vendedores::table
        .filter(vendedores::email.eq(&email))
        .select(Vendedor::as_select())
        .first(conn)
        .optional()?
    {
        if !v.activo {
            return Err(ApiError::forbidden("Tu cuenta está desactivada"));
        }
        if !v.twofa_enabled {
            return Err(ApiError::unauthorized("Este usuario no tiene 2FA activo"));
        }
        let secret = crate::crypto::descifrar(v.twofa_secret.as_deref().unwrap_or(""));
        if !crate::twofa::verificar(&secret, codigo) {
            return Err(ApiError::unauthorized("Código de verificación inválido"));
        }
        return Ok(Box::new(sesion_vendedor(v)));
    }

    if let Some(m) = miembros::table
        .filter(miembros::email.eq(&email))
        .filter(miembros::estado.eq("activo"))
        .select(Miembro::as_select())
        .first(conn)
        .optional()?
    {
        if !m.twofa_enabled {
            return Err(ApiError::unauthorized("Este usuario no tiene 2FA activo"));
        }
        let org: Vendedor = vendedores::table.find(m.vendedor_id).select(Vendedor::as_select()).first(conn)?;
        if !org.activo {
            return Err(ApiError::forbidden("La cuenta de tu organización está desactivada"));
        }
        let secret = crate::crypto::descifrar(m.twofa_secret.as_deref().unwrap_or(""));
        if !crate::twofa::verificar(&secret, codigo) {
            return Err(ApiError::unauthorized("Código de verificación inválido"));
        }
        return Ok(Box::new(sesion_miembro(&org, m)));
    }

    Err(ApiError::unauthorized("Sesión inválida. Vuelve a iniciar sesión."))
}

pub fn login(conn: &mut PgConnection, req: LoginRequest) -> ApiResult<LoginOutcome> {
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
        // 2FA: si está activo, exige el código TOTP.
        if v.twofa_enabled {
            match req.codigo.as_deref() {
                None => return Ok(LoginOutcome::TwoFaRequerido),
                Some(c) => {
                    let secret = crate::crypto::descifrar(v.twofa_secret.as_deref().unwrap_or(""));
                    if !crate::twofa::verificar(&secret, c) {
                        return Err(ApiError::unauthorized("Código de verificación inválido"));
                    }
                }
            }
        }
        return Ok(LoginOutcome::Ok(Box::new(sesion_vendedor(v))));
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
        if m.twofa_enabled {
            match req.codigo.as_deref() {
                None => return Ok(LoginOutcome::TwoFaRequerido),
                Some(c) => {
                    let secret = crate::crypto::descifrar(m.twofa_secret.as_deref().unwrap_or(""));
                    if !crate::twofa::verificar(&secret, c) {
                        return Err(ApiError::unauthorized("Código de verificación inválido"));
                    }
                }
            }
        }
        return Ok(LoginOutcome::Ok(Box::new(sesion_miembro(&org, m))));
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
    pub twofa_enabled: bool,
}

pub fn identidad(conn: &mut PgConnection, org_id: Uuid, actor_id: Uuid) -> ApiResult<Identidad> {
    if actor_id == org_id {
        let v = get(conn, org_id)?;
        Ok(Identidad { nombre: v.nombre, email: v.email, plan: v.plan, twofa_enabled: v.twofa_enabled })
    } else {
        let plan: String = vendedores::table.find(org_id).select(vendedores::plan).first(conn)?;
        let m: Miembro = miembros::table.find(actor_id).select(Miembro::as_select()).first(conn)?;
        Ok(Identidad { nombre: m.nombre, email: m.email, plan, twofa_enabled: m.twofa_enabled })
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
    Ok(Identidad { nombre, email, plan: ident.plan, twofa_enabled: ident.twofa_enabled })
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
            .set((vendedores::password_hash.eq(hash), vendedores::session_version.eq(vendedores::session_version + 1), vendedores::updated_at.eq(crate::services::common::now())))
            .execute(conn)?;
    } else {
        let m: Miembro = miembros::table.find(actor_id).select(Miembro::as_select()).first(conn)?;
        let actual = m.password_hash.as_deref().unwrap_or("");
        if !bcrypt::verify(&req.actual, actual).unwrap_or(false) {
            return Err(ApiError::unauthorized("La contraseña actual no es correcta"));
        }
        diesel::update(miembros::table.find(actor_id))
            .set((miembros::password_hash.eq(hash), miembros::session_version.eq(miembros::session_version + 1), miembros::updated_at.eq(crate::services::common::now())))
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
pub fn set_password(conn: &mut PgConnection, id: Uuid, pwf: &str, nueva: &str) -> ApiResult<()> {
    validar_password(nueva)?;
    // Uso único: la huella del enlace debe coincidir con el hash actual. Si ya se
    // reseteó (u otro cambio de contraseña), la huella no coincide y se rechaza.
    let actual: String = vendedores::table.find(id).select(vendedores::password_hash).first(conn)?;
    if crate::auth::huella_pw(&actual) != pwf {
        return Err(ApiError::unauthorized("El enlace ya fue utilizado o no es válido."));
    }
    let hash = bcrypt::hash(nueva, bcrypt::DEFAULT_COST)
        .map_err(|e| ApiError::internal(format!("No se pudo cifrar la contraseña: {e}")))?;
    diesel::update(vendedores::table.find(id))
        .set((vendedores::password_hash.eq(hash), vendedores::session_version.eq(vendedores::session_version + 1), vendedores::updated_at.eq(crate::services::common::now())))
        .execute(conn)?;
    Ok(())
}

/// Cierra la sesión en todos los dispositivos (sube la versión → invalida todos los tokens).
pub fn logout_all(conn: &mut PgConnection, es_miembro: bool, actor_id: Uuid) -> ApiResult<()> {
    let now = crate::services::common::now();
    if es_miembro {
        diesel::update(miembros::table.find(actor_id))
            .set((miembros::session_version.eq(miembros::session_version + 1), miembros::updated_at.eq(now)))
            .execute(conn)?;
    } else {
        diesel::update(vendedores::table.find(actor_id))
            .set((vendedores::session_version.eq(vendedores::session_version + 1), vendedores::updated_at.eq(now)))
            .execute(conn)?;
    }
    Ok(())
}

/// Estado 2FA del actor (dueño en `vendedores`, miembro en `miembros`).
fn twofa_estado(conn: &mut PgConnection, es_miembro: bool, actor_id: Uuid) -> ApiResult<(String, Option<String>, bool)> {
    if es_miembro {
        let m: Miembro = miembros::table.find(actor_id).select(Miembro::as_select()).first(conn)?;
        Ok((m.email, m.twofa_secret, m.twofa_enabled))
    } else {
        let v = get(conn, actor_id)?;
        Ok((v.email, v.twofa_secret, v.twofa_enabled))
    }
}

fn twofa_set(conn: &mut PgConnection, es_miembro: bool, actor_id: Uuid, secret: Option<String>, enabled: bool) -> ApiResult<()> {
    let now = crate::services::common::now();
    if es_miembro {
        diesel::update(miembros::table.find(actor_id))
            .set((miembros::twofa_secret.eq(secret), miembros::twofa_enabled.eq(enabled), miembros::updated_at.eq(now)))
            .execute(conn)?;
    } else {
        diesel::update(vendedores::table.find(actor_id))
            .set((vendedores::twofa_secret.eq(secret), vendedores::twofa_enabled.eq(enabled), vendedores::updated_at.eq(now)))
            .execute(conn)?;
    }
    Ok(())
}

/// Genera el secreto 2FA (sin activar aún). Devuelve (secreto, url_otpauth, qr_png_base64).
pub fn twofa_setup(conn: &mut PgConnection, es_miembro: bool, actor_id: Uuid) -> ApiResult<(String, String, String)> {
    let (email, _, _) = twofa_estado(conn, es_miembro, actor_id)?;
    let secret = crate::twofa::generar_secret();
    let url = crate::twofa::url(&secret, &email)?;
    let qr = crate::twofa::qr(&secret, &email)?;
    twofa_set(conn, es_miembro, actor_id, Some(crate::crypto::cifrar(&secret)), false)?;
    Ok((secret, url, qr))
}

pub fn twofa_enable(conn: &mut PgConnection, es_miembro: bool, actor_id: Uuid, codigo: &str) -> ApiResult<()> {
    let (_, enc, _) = twofa_estado(conn, es_miembro, actor_id)?;
    let enc = enc.ok_or_else(|| ApiError::bad_request("Primero genera el 2FA"))?;
    let secret = crate::crypto::descifrar(&enc);
    if !crate::twofa::verificar(&secret, codigo) {
        return Err(ApiError::bad_request("Código de verificación inválido"));
    }
    twofa_set(conn, es_miembro, actor_id, Some(enc), true)
}

pub fn twofa_disable(conn: &mut PgConnection, es_miembro: bool, actor_id: Uuid, codigo: &str) -> ApiResult<()> {
    let (_, enc, enabled) = twofa_estado(conn, es_miembro, actor_id)?;
    if enabled {
        let secret = crate::crypto::descifrar(enc.as_deref().unwrap_or(""));
        if !crate::twofa::verificar(&secret, codigo) {
            return Err(ApiError::bad_request("Código de verificación inválido"));
        }
    }
    twofa_set(conn, es_miembro, actor_id, None, false)
}
