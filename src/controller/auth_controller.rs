use crate::auth::{crear_reset_token, crear_token, verificar_reset_token, AuthVendedor};
use crate::config::Config;
use crate::db::{self, DbPool};
use crate::errors::ApiResult;
use crate::mailer::Mailer;
use crate::services::auth_services::{self as svc, LoginRequest, PasswordUpdate, PerfilUpdate, RegistroRequest};
use actix_web::{web, HttpResponse};
use serde::Deserialize;
use serde_json::json;

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

#[derive(Deserialize)]
pub struct ForgotBody {
    pub email: String,
}

/// Solicita recuperar contraseña: envía un correo con enlace (si el email existe).
pub async fn forgot(
    body: web::Json<ForgotBody>,
    pool: web::Data<DbPool>,
    cfg: web::Data<Config>,
    mailer: web::Data<Mailer>,
) -> ApiResult<HttpResponse> {
    let email = body.into_inner().email;
    let email2 = email.clone();
    let found = db::run(&pool, move |conn| svc::buscar_por_email(conn, &email2)).await?;
    if let Some(v) = found {
        if let Ok(token) = crear_reset_token(v.id, &v.password_hash, &cfg.jwt_secret) {
            let link = format!("{}/reset?token={}", cfg.app_url, token);
            let html = format!(
                "<p>Hola {},</p><p>Para restablecer tu contraseña de MeliHub, haz clic en el botón:</p>\
                 <p><a href=\"{link}\" style=\"background:#111;color:#fff;padding:10px 16px;border-radius:8px;text-decoration:none\">Restablecer contraseña</a></p>\
                 <p>El enlace vence en 1 hora. Si no lo pediste, ignora este correo.</p>",
                esc(&v.nombre)
            );
            if let Err(e) = mailer.enviar(&v.email, "Restablecer contraseña · MeliHub", &html).await {
                log::warn!("No se pudo enviar el correo de reseteo: {e}");
            }
        }
    }
    // Siempre 200 para no revelar si el correo existe (anti-enumeración).
    Ok(HttpResponse::Ok().json(json!({ "ok": true, "message": "Si el correo está registrado, te enviamos un enlace." })))
}

#[derive(Deserialize)]
pub struct ResetBody {
    pub token: String,
    pub nueva: String,
}

/// Restablece la contraseña usando el token del correo.
pub async fn reset(body: web::Json<ResetBody>, pool: web::Data<DbPool>, cfg: web::Data<Config>) -> ApiResult<HttpResponse> {
    let ResetBody { token, nueva } = body.into_inner();
    let (id, pwf) = verificar_reset_token(&token, &cfg.jwt_secret)?;
    db::run(&pool, move |conn| svc::set_password(conn, id, &pwf, &nueva)).await?;
    Ok(HttpResponse::Ok().json(json!({ "ok": true })))
}

pub async fn registrar(
    body: web::Json<RegistroRequest>,
    pool: web::Data<DbPool>,
    cfg: web::Data<Config>,
) -> ApiResult<HttpResponse> {
    let v = db::run(&pool, move |conn| svc::registrar(conn, body.into_inner())).await?;
    let token = crear_token(v.id, &v.rol, v.id, "owner", v.session_version, &cfg.jwt_secret)?;
    Ok(HttpResponse::Created().json(json!({ "token": token, "vendedor": v })))
}

pub async fn login(
    req: actix_web::HttpRequest,
    body: web::Json<LoginRequest>,
    pool: web::Data<DbPool>,
    cfg: web::Data<Config>,
    limiter: web::Data<crate::auth::LoginLimiter>,
) -> ApiResult<HttpResponse> {
    // IP real (considera X-Forwarded-For detrás de ngrok/Render).
    let ip = req.connection_info().realip_remote_addr().unwrap_or("desconocida").to_string();
    if !limiter.permitido(&ip) {
        return Ok(HttpResponse::TooManyRequests()
            .json(json!({ "error": "Demasiados intentos. Espera unos minutos e intenta de nuevo." })));
    }
    match db::run(&pool, move |conn| svc::login(conn, body.into_inner())).await {
        Ok(svc::LoginOutcome::TwoFaRequerido) => {
            limiter.exito(&ip);
            Ok(HttpResponse::Ok().json(json!({ "twofa_required": true })))
        }
        Ok(svc::LoginOutcome::Ok(s)) => {
            limiter.exito(&ip);
            let token = crear_token(s.org_id, &s.rol, s.actor_id, &s.rol_equipo, s.sv, &cfg.jwt_secret)?;
            Ok(HttpResponse::Ok().json(json!({ "token": token, "me": s })))
        }
        Err(e) => {
            limiter.fallo(&ip);
            Err(e)
        }
    }
}

/// Cierra la sesión en todos los dispositivos (invalida todos los tokens del actor).
pub async fn logout_all(user: AuthVendedor, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    let (es_m, actor) = (user.es_miembro(), user.actor_id);
    db::run(&pool, move |conn| svc::logout_all(conn, es_m, actor)).await?;
    Ok(HttpResponse::Ok().json(json!({ "ok": true })))
}

pub async fn me(user: AuthVendedor, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    let (org, actor) = (user.id, user.actor_id);
    let ident = db::run(&pool, move |conn| svc::identidad(conn, org, actor)).await?;
    Ok(HttpResponse::Ok().json(json!({
        "nombre": ident.nombre,
        "email": ident.email,
        "plan": ident.plan,
        "twofa_enabled": ident.twofa_enabled,
        "rol": user.rol,
        "rol_equipo": user.rol_equipo,
        "es_admin": user.es_admin(),
        "es_owner": user.es_owner(),
        "es_miembro": user.es_miembro(),
    })))
}

#[derive(Deserialize)]
pub struct CodigoBody {
    pub codigo: String,
}

/// Genera el secreto 2FA (aún sin activar) y devuelve QR + secreto para configurar.
pub async fn twofa_setup(user: AuthVendedor, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    let (es_m, actor) = (user.es_miembro(), user.actor_id);
    let (secret, url, qr) = db::run(&pool, move |conn| svc::twofa_setup(conn, es_m, actor)).await?;
    Ok(HttpResponse::Ok().json(json!({ "secret": secret, "url": url, "qr": qr })))
}

/// Activa el 2FA tras verificar un código.
pub async fn twofa_enable(user: AuthVendedor, body: web::Json<CodigoBody>, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    let (es_m, actor, codigo) = (user.es_miembro(), user.actor_id, body.into_inner().codigo);
    db::run(&pool, move |conn| svc::twofa_enable(conn, es_m, actor, &codigo)).await?;
    Ok(HttpResponse::Ok().json(json!({ "ok": true })))
}

/// Desactiva el 2FA.
pub async fn twofa_disable(user: AuthVendedor, body: web::Json<CodigoBody>, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    let (es_m, actor, codigo) = (user.es_miembro(), user.actor_id, body.into_inner().codigo);
    db::run(&pool, move |conn| svc::twofa_disable(conn, es_m, actor, &codigo)).await?;
    Ok(HttpResponse::Ok().json(json!({ "ok": true })))
}

/// Actualiza el perfil (nombre/email) del vendedor autenticado.
pub async fn actualizar_perfil(user: AuthVendedor, body: web::Json<PerfilUpdate>, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    let (org, actor) = (user.id, user.actor_id);
    let v = db::run(&pool, move |conn| svc::actualizar_perfil(conn, org, actor, body.into_inner())).await?;
    Ok(HttpResponse::Ok().json(v))
}

/// Cambia la contraseña del vendedor autenticado.
pub async fn cambiar_password(user: AuthVendedor, body: web::Json<PasswordUpdate>, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    let (org, actor) = (user.id, user.actor_id);
    db::run(&pool, move |conn| svc::cambiar_password(conn, org, actor, body.into_inner())).await?;
    Ok(HttpResponse::Ok().json(json!({ "ok": true })))
}
