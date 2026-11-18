use crate::auth::AuthVendedor;
use crate::config::Config;
use crate::db::{self, DbPool};
use crate::errors::ApiResult;
use crate::services::equipo_services as svc;
use actix_web::{web, HttpResponse};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// Lista los miembros de la organización del actor.
pub async fn listar(user: AuthVendedor, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    let org = user.id;
    let ms = db::run(&pool, move |conn| svc::listar(conn, org)).await?;
    Ok(HttpResponse::Ok().json(json!({ "miembros": ms, "puede_gestionar": user.es_owner() })))
}

#[derive(Deserialize)]
pub struct InvitarBody {
    pub nombre: String,
    pub email: String,
    pub rol: String,
}

/// Invita a un miembro (solo dueño). Envía correo si Resend está configurado y
/// devuelve el enlace para compartir manualmente si no.
pub async fn invitar(
    user: AuthVendedor,
    body: web::Json<InvitarBody>,
    pool: web::Data<DbPool>,
    cfg: web::Data<Config>,
) -> ApiResult<HttpResponse> {
    user.exigir_owner()?;
    let org = user.id;
    let InvitarBody { nombre, email, rol } = body.into_inner();
    let (m, token) = db::run(&pool, move |conn| svc::invitar(conn, org, &nombre, &email, &rol)).await?;
    let link = format!("{}/aceptar?token={}", cfg.app_url, token);
    let correo_ok = crate::email::hay_config(&pool, &cfg.resend_api_key, &cfg.mail_from).await;
    if correo_ok {
        let contenido = format!(
            "<h2 style=\"margin:0 0 12px;color:#13233b;font-size:21px\">Te invitaron a Ventoki</h2>\
             <p>Hola {}, te invitaron a colaborar en <b>Ventoki</b>. Crea tu contraseña para entrar al panel:</p>\
             {}\
             <p style=\"color:#8d93a3;font-size:13px\">El enlace vence en 7 días.</p>",
            esc(&m.nombre),
            crate::email::boton("Aceptar invitación", &link)
        );
        let html = crate::email::plantilla(&contenido);
        let _ = crate::email::enviar_auto(&pool, &cfg.resend_api_key, &cfg.mail_from, &m.email, "Invitación a Ventoki", &html).await;
    }
    let (actor, det) = (user.actor_id, format!("{} ({})", m.email, m.rol));
    let _ = db::run(&pool, move |conn| crate::services::auditoria_services::log(conn, org, actor, "equipo_invitar".into(), det)).await;
    Ok(HttpResponse::Ok().json(json!({ "miembro": m, "enlace": link, "correo_enviado": correo_ok })))
}

/// Reenvía la invitación (nuevo token) a un miembro pendiente (solo dueño).
pub async fn reinvitar(
    user: AuthVendedor,
    path: web::Path<Uuid>,
    pool: web::Data<DbPool>,
    cfg: web::Data<Config>,
) -> ApiResult<HttpResponse> {
    user.exigir_owner()?;
    let (org, mid) = (user.id, path.into_inner());
    let (m, token) = db::run(&pool, move |conn| svc::reinvitar(conn, org, mid)).await?;
    let link = format!("{}/aceptar?token={}", cfg.app_url, token);
    let correo_ok = crate::email::hay_config(&pool, &cfg.resend_api_key, &cfg.mail_from).await;
    if correo_ok {
        let contenido = format!(
            "<h2 style=\"margin:0 0 12px;color:#13233b;font-size:21px\">Tu invitación a Ventoki</h2>\
             <p>Hola {}, aquí tienes de nuevo tu invitación para colaborar en Ventoki:</p>{}",
            esc(&m.nombre),
            crate::email::boton("Aceptar invitación", &link)
        );
        let html = crate::email::plantilla(&contenido);
        let _ = crate::email::enviar_auto(&pool, &cfg.resend_api_key, &cfg.mail_from, &m.email, "Invitación a Ventoki", &html).await;
    }
    Ok(HttpResponse::Ok().json(json!({ "enlace": link, "correo_enviado": correo_ok })))
}

#[derive(Deserialize)]
pub struct RolBody {
    pub rol: String,
}

/// Cambia el rol de un miembro (solo dueño).
pub async fn cambiar_rol(user: AuthVendedor, path: web::Path<Uuid>, body: web::Json<RolBody>, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    user.exigir_owner()?;
    let (org, mid, rol) = (user.id, path.into_inner(), body.into_inner().rol);
    let det = format!("{mid} → {rol}");
    db::run(&pool, move |conn| svc::cambiar_rol(conn, org, mid, &rol)).await?;
    let actor = user.actor_id;
    let _ = db::run(&pool, move |conn| crate::services::auditoria_services::log(conn, org, actor, "equipo_rol".into(), det)).await;
    Ok(HttpResponse::Ok().json(json!({ "ok": true })))
}

/// Elimina a un miembro (solo dueño).
pub async fn eliminar(user: AuthVendedor, path: web::Path<Uuid>, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    user.exigir_owner()?;
    let (org, mid) = (user.id, path.into_inner());
    db::run(&pool, move |conn| svc::eliminar(conn, org, mid)).await?;
    let actor = user.actor_id;
    let _ = db::run(&pool, move |conn| crate::services::auditoria_services::log(conn, org, actor, "equipo_eliminar".into(), mid.to_string())).await;
    Ok(HttpResponse::Ok().json(json!({ "ok": true })))
}

#[derive(Deserialize)]
pub struct AceptarBody {
    pub token: String,
    pub password: String,
}

/// Acepta una invitación (público): fija la contraseña y activa al miembro.
pub async fn aceptar(body: web::Json<AceptarBody>, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    let AceptarBody { token, password } = body.into_inner();
    db::run(&pool, move |conn| svc::aceptar(conn, &token, &password)).await?;
    Ok(HttpResponse::Ok().json(json!({ "ok": true })))
}
