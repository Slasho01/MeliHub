use crate::auth::AuthVendedor;
use crate::config::Config;
use crate::db::{self, DbPool};
use crate::errors::{ApiError, ApiResult};
use crate::ml::MlClient;
use crate::services::common::now;
use crate::services::ml_services as svc;
use actix_web::{web, HttpResponse};
use chrono::Duration;
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

fn exigir_ml(cfg: &Config) -> ApiResult<()> {
    if !cfg.ml.configurado() {
        return Err(ApiError::bad_request(
            "Mercado Libre no está configurado. Define ML_CLIENT_ID y ML_CLIENT_SECRET en el servidor.",
        ));
    }
    Ok(())
}

/// Inicia la conexión: devuelve la URL de autorización de Mercado Libre.
pub async fn conectar(
    user: AuthVendedor,
    pool: web::Data<DbPool>,
    client: web::Data<MlClient>,
    cfg: web::Data<Config>,
) -> ApiResult<HttpResponse> {
    user.exigir_owner()?;
    exigir_ml(&cfg)?;
    let vid = user.id;
    let estado = db::run(&pool, move |conn| svc::crear_estado(conn, vid)).await?;
    Ok(HttpResponse::Ok().json(json!({ "authorize_url": client.authorize_url(&estado) })))
}

#[derive(Deserialize)]
pub struct CallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

/// Callback de OAuth: Mercado Libre redirige aquí tras autorizar.
pub async fn callback(
    q: web::Query<CallbackQuery>,
    pool: web::Data<DbPool>,
    client: web::Data<MlClient>,
    cfg: web::Data<Config>,
) -> ApiResult<HttpResponse> {
    exigir_ml(&cfg)?;
    let q = q.into_inner();
    if let Some(err) = q.error {
        return Ok(pagina(false, &format!("Autorización cancelada: {}", q.error_description.unwrap_or(err))));
    }
    let code = q.code.ok_or_else(|| ApiError::bad_request("Falta el parámetro 'code'"))?;
    let estado = q.state.ok_or_else(|| ApiError::bad_request("Falta el parámetro 'state'"))?;

    // 1) Validar el estado anti-CSRF y saber de qué vendedor es.
    let vendedor_id = db::run(&pool, move |conn| svc::consumir_estado(conn, &estado)).await?;

    // 2) Intercambiar el code por tokens.
    let tokens = client.exchange_code(&code).await?;
    let expira = now() + Duration::seconds(tokens.expires_in.max(0));

    // 3) Datos de la cuenta (nickname, sitio).
    let info = client.get_user(&tokens.access_token, tokens.user_id).await.unwrap_or_default();
    let ml_user_id = if tokens.user_id > 0 { tokens.user_id } else { info.id };
    let sitio = info.site_id.unwrap_or_else(|| "MLC".into());

    // 4) Guardar la cuenta.
    let access = tokens.access_token.clone();
    let refresh = tokens.refresh_token.clone();
    let nickname = info.nickname.clone();
    let cuenta = db::run(&pool, move |conn| {
        crate::services::saas_services::permitir_cuenta_nueva(conn, vendedor_id, ml_user_id)?; // límite del plan
        svc::upsert_cuenta(conn, vendedor_id, ml_user_id, nickname, access, refresh, expira, sitio)
    })
    .await?;

    Ok(pagina(
        true,
        &format!("Cuenta {} conectada. Ya puedes cerrar esta ventana.", cuenta.nickname.as_deref().unwrap_or("de Mercado Libre")),
    ))
}

/// Lista las cuentas ML conectadas por el vendedor.
pub async fn listar_cuentas(user: AuthVendedor, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    let (vid, admin) = (user.id, user.es_admin());
    let cuentas = db::run(&pool, move |conn| svc::listar_cuentas(conn, vid, admin)).await?;
    Ok(HttpResponse::Ok().json(cuentas))
}

/// Desconecta (elimina) una cuenta ML y sus productos.
pub async fn desconectar(
    path: web::Path<Uuid>,
    user: AuthVendedor,
    pool: web::Data<DbPool>,
) -> ApiResult<HttpResponse> {
    user.exigir_owner()?;
    let (cid, vid, admin) = (path.into_inner(), user.id, user.es_admin());
    db::run(&pool, move |conn| svc::desconectar(conn, cid, vid, admin)).await?;
    Ok(HttpResponse::NoContent().finish())
}

/// Sincroniza (importa/actualiza) las publicaciones de una cuenta.
pub async fn sincronizar(
    path: web::Path<Uuid>,
    user: AuthVendedor,
    pool: web::Data<DbPool>,
    client: web::Data<MlClient>,
    cfg: web::Data<Config>,
) -> ApiResult<HttpResponse> {
    user.exigir_edicion()?;
    exigir_ml(&cfg)?;
    let (cid, vid, admin) = (path.into_inner(), user.id, user.es_admin());
    let cuenta = db::run(&pool, move |conn| svc::get_cuenta(conn, cid, vid, admin)).await?;
    let total = crate::sync::sincronizar_cuenta(&pool, &client, &cuenta).await?;
    Ok(HttpResponse::Ok().json(json!({ "sincronizados": total })))
}

#[derive(Deserialize)]
pub struct WebhookNotif {
    #[serde(default)]
    pub topic: Option<String>,
    #[serde(default)]
    pub resource: Option<String>,
    #[serde(default)]
    pub user_id: Option<i64>,
}

/// Webhook de Mercado Libre: notificaciones de cambios (items, etc.).
/// No autenticado (lo llama ML); se valida por el user_id contra las cuentas conectadas.
/// Responde 200 siempre y rápido; el procesamiento ocurre en segundo plano.
pub async fn webhook(
    body: web::Json<WebhookNotif>,
    pool: web::Data<DbPool>,
    client: web::Data<MlClient>,
) -> ApiResult<HttpResponse> {
    let n = body.into_inner();
    let (pool2, client2) = (pool.clone(), client.clone());
    actix_web::rt::spawn(async move {
        if let Err(e) = procesar_notificacion(&pool2, &client2, n).await {
            log::warn!("Webhook ML: {e}");
        }
    });
    Ok(HttpResponse::Ok().finish())
}

async fn procesar_notificacion(pool: &web::Data<DbPool>, client: &web::Data<MlClient>, n: WebhookNotif) -> ApiResult<()> {
    let topic = n.topic.unwrap_or_default();
    let resource = n.resource.unwrap_or_default();
    let user_id = n.user_id.unwrap_or(0);
    if user_id == 0 {
        return Ok(());
    }
    let cuenta = db::run(pool, move |conn| svc::cuenta_por_ml_user(conn, user_id)).await?;
    let Some(cuenta) = cuenta else { return Ok(()) }; // notificación de una cuenta que no gestionamos
    // Para items: resource = "/items/MLCxxxx" -> refrescamos ese ítem.
    if topic == "items" || resource.starts_with("/items/") {
        if let Some(item_id) = resource.rsplit('/').next().filter(|s| !s.is_empty()) {
            crate::sync::refrescar_item(pool, client, &cuenta, item_id).await?;
        }
    } else {
        // Otros tópicos (orders, questions...) se manejarán en fases siguientes.
        log::info!("Webhook ML tópico '{topic}' recibido (sin acción aún)");
    }
    Ok(())
}

/// Página HTML mínima para el callback del navegador.
fn pagina(ok: bool, msg: &str) -> HttpResponse {
    let color = if ok { "#16a34a" } else { "#d93025" };
    let titulo = if ok { "✓ Conectado" } else { "No se pudo conectar" };
    let html = format!(
        "<!doctype html><meta charset=\"utf-8\"><title>MeliHub</title>\
         <body style=\"font-family:system-ui;display:grid;place-items:center;height:100vh;margin:0;background:#f4f5f7\">\
         <div style=\"text-align:center;padding:32px;background:#fff;border-radius:16px;box-shadow:0 10px 40px rgba(0,0,0,.08)\">\
         <h1 style=\"color:{color};margin:0 0 8px\">{titulo}</h1><p style=\"color:#444;margin:0\">{}</p></div></body>",
        html_escape(msg)
    );
    HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html)
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}
