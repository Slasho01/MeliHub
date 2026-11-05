use crate::auth::AuthVendedor;
use crate::config::Config;
use crate::db::{self, DbPool};
use crate::errors::{ApiError, ApiResult};
use crate::mp::MercadoPago;
use crate::services::suscripcion_services as subs;
use actix_web::{web, HttpRequest, HttpResponse};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;

#[derive(Deserialize)]
pub struct SuscribirBody {
    pub plan: String,
}

/// Inicia la suscripción a un plan: crea el preapproval en MP y devuelve la URL de checkout.
pub async fn suscribir(
    user: AuthVendedor,
    body: web::Json<SuscribirBody>,
    pool: web::Data<DbPool>,
    mp: web::Data<MercadoPago>,
    cfg: web::Data<Config>,
) -> ApiResult<HttpResponse> {
    user.exigir_owner()?;
    if !mp.configurado() {
        return Err(ApiError::bad_request("Los pagos no están configurados (falta MP_ACCESS_TOKEN)."));
    }
    let plan = crate::planes::plan(&body.into_inner().plan);
    if plan.id == "free" || plan.precio_mensual <= 0 {
        return Err(ApiError::bad_request("Ese plan no requiere pago."));
    }
    let vid = user.id;
    let email = db::run(&pool, move |conn| subs::email_de(conn, vid)).await?;
    let back = format!("{}/plan", cfg.app_url);
    let notif = format!("{}/api/suscripcion/webhook", cfg.app_url);
    let ext = format!("{}:{}", vid, plan.id);
    let reason = format!("MeliHub - Plan {}", plan.nombre);
    let pre = mp.crear_preapproval(&reason, plan.precio_mensual, "CLP", &email, &back, &notif, &ext).await?;

    let (pid, monto, plan_id) = (pre.id.clone(), plan.precio_mensual, plan.id.to_string());
    db::run(&pool, move |conn| subs::registrar(conn, vid, &plan_id, monto, &pid)).await?;
    Ok(HttpResponse::Ok().json(json!({ "init_point": pre.init_point })))
}

/// Estado de la suscripción del vendedor.
pub async fn estado(user: AuthVendedor, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    let vid = user.id;
    let sub = db::run(&pool, move |conn| subs::actual(conn, vid)).await?;
    Ok(HttpResponse::Ok().json(json!({ "suscripcion": sub })))
}

/// Cancela la suscripción (en MeliHub y en Mercado Pago).
pub async fn cancelar(user: AuthVendedor, pool: web::Data<DbPool>, mp: web::Data<MercadoPago>) -> ApiResult<HttpResponse> {
    user.exigir_owner()?;
    let vid = user.id;
    let pid = db::run(&pool, move |conn| subs::cancelar_local(conn, vid)).await?;
    if let Some(pid) = pid {
        let _ = mp.cancel_preapproval(&pid).await; // best-effort
    }
    Ok(HttpResponse::Ok().json(json!({ "ok": true })))
}

/// Webhook de Mercado Pago: al cambiar el estado de la suscripción, ajusta el plan.
pub async fn webhook(
    req: HttpRequest,
    body: web::Bytes,
    pool: web::Data<DbPool>,
    mp: web::Data<MercadoPago>,
    mailer: web::Data<crate::mailer::Mailer>,
    cfg: web::Data<Config>,
) -> HttpResponse {
    // El id del preapproval puede venir por query (?id= / ?data.id=) o en el body.
    let q: HashMap<String, String> = web::Query::<HashMap<String, String>>::from_query(req.query_string())
        .map(|x| x.into_inner())
        .unwrap_or_default();
    let body_json: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
    let topic = q
        .get("topic")
        .or_else(|| q.get("type"))
        .cloned()
        .or_else(|| body_json.get("type").and_then(|v| v.as_str()).map(String::from))
        .unwrap_or_default();
    let id = q
        .get("id")
        .or_else(|| q.get("data.id"))
        .cloned()
        .or_else(|| body_json.pointer("/data/id").and_then(|v| v.as_str()).map(String::from));

    if let Some(id) = id {
        // Solo nos interesan las suscripciones (preapproval).
        if topic.is_empty() || topic.contains("preapproval") || topic.contains("subscription") {
            if let Ok(pre) = mp.get_preapproval(&id).await {
                let estado = pre.status.unwrap_or_default();
                let (id_c, estado_c) = (id.clone(), estado.clone());
                let _ = db::run(&pool, move |conn| subs::aplicar_estado(conn, &id_c, &estado_c)).await;
                // Correo de confirmación al activarse la suscripción.
                if estado == "authorized" && mailer.configurado() {
                    if let Ok(Some((email, plan))) = db::run(&pool, move |conn| subs::email_y_plan(conn, &id)).await {
                        let html = format!(
                            "<p>¡Tu suscripción al plan <b>{plan}</b> de MeliHub está activa!</p>\
                             <p>Gracias por suscribirte. Puedes gestionarla desde <a href=\"{}/plan\">tu panel</a>.</p>",
                            cfg.app_url
                        );
                        let _ = mailer.enviar(&email, "Suscripción activada · MeliHub", &html).await;
                    }
                }
            }
        }
    }
    HttpResponse::Ok().finish() // siempre 200 para que MP no reintente de más
}
