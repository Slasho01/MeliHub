use crate::auth::AuthVendedor;
use crate::config::Config;
use crate::db::{self, DbPool};
use crate::errors::{ApiError, ApiResult};
use crate::mp::MercadoPago;
use crate::services::{cupon_services, facturacion_services, suscripcion_services as subs};
use actix_web::{web, HttpRequest, HttpResponse};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct SuscribirBody {
    pub plan: String,
    #[serde(default)]
    pub ciclo: Option<String>,
    #[serde(default)]
    pub cupon: Option<String>,
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
    let SuscribirBody { plan: plan_in, ciclo, cupon } = body.into_inner();
    let plan = crate::planes::plan(&plan_in);
    if plan.id == "free" {
        return Err(ApiError::bad_request("Ese plan no requiere pago."));
    }
    let ciclo = crate::planes::ciclo_valido(ciclo.as_deref().unwrap_or("mensual"));
    let base = crate::planes::precio_de(plan, ciclo);
    if base <= 0 {
        return Err(ApiError::bad_request("Ese plan no tiene precio para ese ciclo."));
    }

    // Antes de cobrar, exige datos de facturación completos.
    let org = user.id;
    if !db::run(&pool, move |conn| facturacion_services::completo(conn, org)).await? {
        return Err(ApiError::bad_request("Completa tus datos de facturación en «Mi cuenta» antes de suscribirte."));
    }

    // Cupón de descuento (opcional): valida y aplica sobre el precio base.
    let codigo = cupon.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(|s| s.to_string());
    let (monto, cupon_id, cupon_norm): (i64, Option<Uuid>, Option<String>) = if let Some(code) = codigo {
        let c = db::run(&pool, move |conn| cupon_services::validar(conn, &code)).await?;
        (cupon_services::aplicar(base, &c), Some(c.id), Some(c.codigo))
    } else {
        (base, None, None)
    };
    if monto <= 0 {
        return Err(ApiError::bad_request("El descuento deja el precio en 0; usa el plan Free."));
    }

    let vid = user.id;
    let email = db::run(&pool, move |conn| subs::email_de(conn, vid)).await?;
    let back = format!("{}/suscripcion", cfg.app_url);
    let notif = format!("{}/api/suscripcion/webhook", cfg.app_url);
    let ext = format!("{}:{}:{}", vid, plan.id, ciclo);
    let reason = format!("MeliHub - Plan {} ({})", plan.nombre, ciclo);
    let (frequency, ftype) = if ciclo == "anual" { (12, "months") } else { (1, "months") };
    let pre = mp.crear_preapproval(&reason, monto, "CLP", &email, &back, &notif, &ext, frequency, ftype).await?;

    let (pid, plan_id, ciclo_s, cupon_s) = (pre.id.clone(), plan.id.to_string(), ciclo.to_string(), cupon_norm.clone());
    db::run(&pool, move |conn| subs::registrar(conn, vid, &plan_id, monto, &pid, &ciclo_s, cupon_s.as_deref())).await?;
    if let Some(cid) = cupon_id {
        let _ = db::run(&pool, move |conn| cupon_services::consumir(conn, cid)).await;
    }
    Ok(HttpResponse::Ok().json(json!({ "init_point": pre.init_point, "monto": monto })))
}

/// Estado de la suscripción del vendedor.
pub async fn estado(user: AuthVendedor, pool: web::Data<DbPool>, mp: web::Data<MercadoPago>) -> ApiResult<HttpResponse> {
    let vid = user.id;
    let mut sub = db::run(&pool, move |conn| subs::actual(conn, vid)).await?;
    // Autocuración: si hay una suscripción no finalizada con preapproval, consulta
    // el estado real en Mercado Pago y aplícalo (por si el webhook no llegó/tardó).
    if let Some(s) = &sub {
        let pendiente = s.estado == "pending" || s.estado == "paused";
        if pendiente && mp.configurado() {
            if let Some(pid) = s.preapproval_id.clone() {
                if let Ok(pre) = mp.get_preapproval(&pid).await {
                    if let Some(estado_mp) = pre.status {
                        let (pid2, est2) = (pid.clone(), estado_mp.clone());
                        let _ = db::run(&pool, move |conn| subs::aplicar_estado(conn, &pid2, &est2)).await;
                        sub = db::run(&pool, move |conn| subs::actual(conn, vid)).await?;
                    }
                }
            }
        }
    }
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

    log::info!("[MP webhook] topic={topic:?} id={id:?} query={q:?}");
    if let Some(id) = id {
        // Solo nos interesan las suscripciones (preapproval).
        if topic.is_empty() || topic.contains("preapproval") || topic.contains("subscription") {
            match mp.get_preapproval(&id).await {
                Err(e) => log::warn!("[MP webhook] no se pudo leer preapproval {id}: {e}"),
                Ok(pre) => {
                let estado = pre.status.unwrap_or_default();
                log::info!("[MP webhook] preapproval {id} estado={estado}");
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
    }
    HttpResponse::Ok().finish() // siempre 200 para que MP no reintente de más
}
