use crate::auth::AuthVendedor;
use crate::config::Config;
use crate::db::{self, DbPool};
use crate::errors::{ApiError, ApiResult};
use crate::falabella::FalabellaClient;
use crate::ml::MlClient;
use crate::paris::ParisClient;
use crate::services::clon_services;
use crate::services::common::now;
use crate::services::ml_services as svc;
use crate::services::producto_services as prod;
use actix_web::{web, HttpResponse};
use chrono::Duration;
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct ClonarBody {
    pub origen_cuenta_id: Uuid,
    pub destino_cuenta_id: Uuid,
    pub producto_ids: Vec<Uuid>,
}

/// Clona publicaciones de una cuenta de ML a otra (crea cada una con `POST /items`
/// en la cuenta destino). Devuelve un reporte por producto (ok/permalink o error).
pub async fn clonar(
    user: AuthVendedor,
    body: web::Json<ClonarBody>,
    pool: web::Data<DbPool>,
    client: web::Data<MlClient>,
) -> ApiResult<HttpResponse> {
    // Por ahora la clonación es solo para el administrador.
    if !user.es_admin() {
        return Err(ApiError::forbidden("La clonación de publicaciones está disponible solo para el administrador por ahora."));
    }
    let (vid, admin) = (user.id, user.es_admin());
    let ClonarBody { origen_cuenta_id, destino_cuenta_id, producto_ids } = body.into_inner();
    if origen_cuenta_id == destino_cuenta_id {
        return Err(ApiError::bad_request("La cuenta destino debe ser distinta de la de origen."));
    }
    if producto_ids.is_empty() {
        return Err(ApiError::bad_request("Selecciona al menos un producto para clonar."));
    }
    if producto_ids.len() > 200 {
        return Err(ApiError::bad_request("Máximo 200 productos por operación."));
    }
    // Ambas cuentas deben pertenecer al vendedor.
    let destino = db::run(&pool, move |conn| svc::get_cuenta(conn, destino_cuenta_id, vid, admin)).await?;
    db::run(&pool, move |conn| svc::get_cuenta(conn, origen_cuenta_id, vid, admin)).await?;
    let ids = producto_ids.clone();
    let productos = db::run(&pool, move |conn| prod::por_ids_de_cuenta(conn, origen_cuenta_id, &ids)).await?;
    if productos.is_empty() {
        return Err(ApiError::bad_request("No se encontraron esos productos en la cuenta de origen."));
    }
    let token = crate::sync::access_token_valido(&pool, &client, &destino).await?;

    let mut resultados = Vec::new();
    let mut creados = 0usize;
    for p in productos {
        // Intento 1: modo normal (con título). Si ML pide family_name (categoría de
        // catálogo), reintento en modo familia (family_name, sin título).
        let mut res = client.crear_item(&token, &clon_services::payload(&p, false)).await;
        if let Err(e) = &res {
            if e.to_string().contains("family_name") {
                res = client.crear_item(&token, &clon_services::payload(&p, true)).await;
            }
        }
        match res {
            Ok(creado) => {
                let new_id = creado.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let permalink = creado.get("permalink").and_then(|v| v.as_str()).map(|s| s.to_string());
                if let Some(desc) = p.datos.get("descripcion").and_then(|v| v.as_str()).filter(|s| !s.trim().is_empty()) {
                    let _ = client.set_item_description(&token, &new_id, desc).await;
                }
                // Enlaza el clon al grupo de stock del original → compartirán stock entre cuentas.
                if !new_id.is_empty() {
                    let (pid_origen, dest, nid) = (p.id, destino_cuenta_id, new_id.clone());
                    let _ = db::run(&pool, move |conn| {
                        let g = prod::asegurar_grupo(conn, pid_origen)?;
                        prod::registrar_grupo_pendiente(conn, dest, &nid, g)
                    })
                    .await;
                }
                creados += 1;
                resultados.push(json!({ "producto": p.titulo, "ok": true, "item_id": new_id, "permalink": permalink }));
            }
            Err(e) => resultados.push(json!({ "producto": p.titulo, "ok": false, "error": e.to_string() })),
        }
    }
    Ok(HttpResponse::Ok().json(json!({ "total": resultados.len(), "creados": creados, "resultados": resultados })))
}

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
    let actor = user.actor_id;
    let _ = db::run(&pool, move |conn| crate::services::auditoria_services::log(conn, vid, actor, "ml_desconectar".into(), cid.to_string())).await;
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
    body: web::Bytes,
    pool: web::Data<DbPool>,
    client: web::Data<MlClient>,
    fala: web::Data<FalabellaClient>,
    paris: web::Data<ParisClient>,
) -> ApiResult<HttpResponse> {
    // Log de recepción CRUDA: confirma que ML está llegando pase lo que pase con el parseo.
    let crudo: String = String::from_utf8_lossy(&body).chars().take(300).collect();
    log::info!("[Webhook ML] POST recibido ({} bytes): {crudo}", body.len());
    // Parseo tolerante (no exige content-type): si falla, seguimos con campos vacíos.
    let n: WebhookNotif = serde_json::from_slice(&body).unwrap_or(WebhookNotif { topic: None, resource: None, user_id: None });
    let (pool2, client2, fala2, paris2) = (pool.clone(), client.clone(), fala.clone(), paris.clone());
    actix_web::rt::spawn(async move {
        if let Err(e) = procesar_notificacion(&pool2, &client2, &fala2, &paris2, n).await {
            log::warn!("Webhook ML: {e}");
        }
    });
    Ok(HttpResponse::Ok().finish())
}

async fn procesar_notificacion(pool: &web::Data<DbPool>, client: &web::Data<MlClient>, fala: &web::Data<FalabellaClient>, paris: &web::Data<ParisClient>, n: WebhookNotif) -> ApiResult<()> {
    let topic = n.topic.unwrap_or_default();
    let resource = n.resource.unwrap_or_default();
    let user_id = n.user_id.unwrap_or(0);
    log::info!("[Webhook ML] recibido topic='{topic}' resource='{resource}' user={user_id}");
    if user_id == 0 {
        return Ok(());
    }
    let cuenta = db::run(pool, move |conn| svc::cuenta_por_ml_user(conn, user_id)).await?;
    let Some(cuenta) = cuenta else {
        log::warn!("[Webhook ML] NO hay cuenta conectada con ml_user_id={user_id} — notificación ignorada");
        return Ok(());
    };
    // Para items: resource = "/items/MLCxxxx" -> refrescamos ese ítem.
    if topic == "items" || resource.starts_with("/items/") {
        if let Some(item_id) = resource.rsplit('/').next().filter(|s| !s.is_empty()) {
            // 1) Trae el ítem de ML y actualiza el stock/precio en el hub.
            let n = crate::sync::refrescar_item(pool, client, &cuenta, item_id).await?;
            log::info!("[Webhook ML] item {item_id} refrescado desde ML ({n} fila(s) actualizada(s))");
            // 2) Propaga ese producto a los demás canales en segundos (sin esperar el ciclo).
            let (cid, iid) = (cuenta.id, item_id.to_string());
            let pid_opt = db::run(pool, move |conn| svc::producto_id_por_item(conn, cid, &iid)).await.ok().flatten();
            log::info!("[Webhook ML] producto local para {item_id}: {pid_opt:?}");
            if let Some(pid) = pid_opt {
                // Si este producto es componente de kits, recomputa su disponibilidad y empújalos a ML.
                let kits: Vec<(Uuid, crate::services::pub_services::VentaAplicada)> = db::run(pool, move |conn| {
                    let mut out = Vec::new();
                    for k in crate::services::kit_services::kits_que_usan(conn, pid)? {
                        if let Some(v) = crate::services::kit_services::sincronizar_stock_kit(conn, k)? {
                            out.push((k, v));
                        }
                    }
                    Ok::<_, ApiError>(out)
                })
                .await
                .unwrap_or_default();
                for (_, v) in &kits {
                    crate::sync::empujar_stock_ml(pool, client, v).await;
                }
                // Empuja el producto y los kits afectados a Falabella/Paris/Shopify.
                crate::sync::empujar_producto_a_canales(pool, pid, fala, paris).await;
                for (kid, _) in &kits {
                    crate::sync::empujar_producto_a_canales(pool, *kid, fala, paris).await;
                }
                // Espeja el stock a publicaciones hermanas (mismo SELLER_SKU en otra cuenta de ML,
                // típicamente clones) para no sobrevender entre cuentas.
                crate::sync::espejar_hermanos(pool, client, pid).await;
            }
        }
    } else if topic == "questions" || resource.starts_with("/questions/") {
        // Nueva pregunta de un comprador: avisamos al vendedor (se responde en /ml/preguntas).
        let vid = cuenta.vendedor_id;
        let nick = cuenta.nickname.clone().unwrap_or_default();
        db::run(pool, move |conn| {
            let cuerpo = if nick.is_empty() {
                "Tienes una pregunta sin responder en Mercado Libre.".to_string()
            } else {
                format!("Cuenta {nick}: tienes una pregunta sin responder en Mercado Libre.")
            };
            crate::services::saas_services::notificar(conn, vid, "pregunta", "Nueva pregunta en Mercado Libre", Some(cuerpo));
            Ok::<(), ApiError>(())
        })
        .await?;
    } else {
        // Otros tópicos (orders…) se manejarán en fases siguientes.
        log::info!("Webhook ML tópico '{topic}' recibido (sin acción aún)");
    }
    Ok(())
}

/// Página HTML mínima para el callback del navegador.
fn pagina(ok: bool, msg: &str) -> HttpResponse {
    let color = if ok { "#16a34a" } else { "#d93025" };
    let titulo = if ok { "✓ Conectado" } else { "No se pudo conectar" };
    let html = format!(
        "<!doctype html><meta charset=\"utf-8\"><title>Ventoki</title>\
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
