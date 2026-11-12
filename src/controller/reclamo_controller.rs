//! Reclamos (post-venta) de Mercado Libre, centralizados de todas las cuentas.
//! Se leen en vivo de ML (no se almacenan) y se pueden responder desde el panel.

use crate::auth::AuthVendedor;
use crate::config::Config;
use crate::db::{self, DbPool};
use crate::errors::{ApiError, ApiResult};
use crate::ml::MlClient;
use crate::services::ml_services;
use actix_web::{web, HttpResponse};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct ReclamosQuery {
    /// Filtra a una sola cuenta ML; si falta, trae las de todas.
    pub cuenta: Option<Uuid>,
}

/// Lista los reclamos de todas las cuentas (o de una). Best-effort: una cuenta sin token
/// válido o que falle se omite en silencio, el resto sí se devuelve.
pub async fn listar(
    user: AuthVendedor,
    q: web::Query<ReclamosQuery>,
    pool: web::Data<DbPool>,
    client: web::Data<MlClient>,
    cfg: web::Data<Config>,
) -> ApiResult<HttpResponse> {
    if !cfg.ml.configurado() {
        return Ok(HttpResponse::Ok().json(json!([])));
    }
    let (vid, admin, filtro) = (user.id, user.es_admin(), q.cuenta);
    let cuentas = db::run(&pool, move |conn| ml_services::listar_cuentas(conn, vid, admin)).await?;
    let cuentas: Vec<_> = cuentas.into_iter().filter(|c| filtro.map_or(true, |f| c.id == f)).collect();

    let mut salida: Vec<Value> = Vec::new();
    for cuenta in &cuentas {
        let Ok(token) = crate::sync::access_token_valido(&pool, &client, cuenta).await else { continue };
        let Ok(v) = client.claims_search(&token, 0).await else { continue };
        // La respuesta trae los reclamos en `data` (a veces `results`).
        let arr = v.get("data").or_else(|| v.get("results")).and_then(|x| x.as_array());
        let Some(arr) = arr else { continue };
        for c in arr {
            let id = c.get("id").and_then(|x| x.as_i64()).unwrap_or(0);
            if id == 0 {
                continue;
            }
            // El comprador es el `complainant` dentro de players.
            let comprador = c
                .get("players")
                .and_then(|p| p.as_array())
                .and_then(|pl| pl.iter().find(|x| x.get("role").and_then(|r| r.as_str()) == Some("complainant")))
                .and_then(|x| x.get("user_id").and_then(|u| u.as_i64()));
            salida.push(json!({
                "id": id,
                "cuenta_id": cuenta.id,
                "cuenta": cuenta.nickname.clone(),
                "tipo": c.get("type").and_then(|x| x.as_str()).unwrap_or(""),
                "etapa": c.get("stage").and_then(|x| x.as_str()).unwrap_or(""),
                "estado": c.get("status").and_then(|x| x.as_str()).unwrap_or(""),
                "recurso": c.get("resource").and_then(|x| x.as_str()).unwrap_or(""),
                "recurso_id": c.get("resource_id").and_then(|x| x.as_i64()).or_else(|| c.get("resource_id").and_then(|x| x.as_str()).and_then(|s| s.parse().ok())),
                "motivo": c.get("reason_id").and_then(|x| x.as_str()).unwrap_or(""),
                "fecha": c.get("date_created").and_then(|x| x.as_str()).unwrap_or(""),
                "comprador_id": comprador,
            }));
        }
    }
    Ok(HttpResponse::Ok().json(salida))
}

/// Detalle de un reclamo + su conversación (mensajes).
pub async fn detalle(
    user: AuthVendedor,
    path: web::Path<(Uuid, i64)>,
    pool: web::Data<DbPool>,
    client: web::Data<MlClient>,
    cfg: web::Data<Config>,
) -> ApiResult<HttpResponse> {
    if !cfg.ml.configurado() {
        return Err(ApiError::bad_request("Mercado Libre no está configurado."));
    }
    let (cuenta_id, claim_id) = path.into_inner();
    let (vid, admin) = (user.id, user.es_admin());
    let cuenta = db::run(&pool, move |conn| ml_services::get_cuenta(conn, cuenta_id, vid, admin)).await?;
    let token = crate::sync::access_token_valido(&pool, &client, &cuenta).await?;
    let detalle = client.claim_detail(&token, claim_id).await.unwrap_or(Value::Null);
    let mensajes = client.claim_messages(&token, claim_id).await.unwrap_or(Value::Null);
    Ok(HttpResponse::Ok().json(json!({ "detalle": detalle, "mensajes": mensajes })))
}

#[derive(Deserialize)]
pub struct ResponderReclamo {
    pub cuenta_id: Uuid,
    pub claim_id: i64,
    pub texto: String,
}

/// Envía un mensaje al comprador dentro de un reclamo. Requiere permiso de edición.
pub async fn responder(
    user: AuthVendedor,
    body: web::Json<ResponderReclamo>,
    pool: web::Data<DbPool>,
    client: web::Data<MlClient>,
    cfg: web::Data<Config>,
) -> ApiResult<HttpResponse> {
    user.exigir_edicion()?;
    if !cfg.ml.configurado() {
        return Err(ApiError::bad_request("Mercado Libre no está configurado."));
    }
    let ResponderReclamo { cuenta_id, claim_id, texto } = body.into_inner();
    let texto = texto.trim().to_string();
    if texto.is_empty() {
        return Err(ApiError::bad_request("Escribe un mensaje."));
    }
    if texto.chars().count() > 2000 {
        return Err(ApiError::bad_request("El mensaje es demasiado largo (máx. 2000 caracteres)."));
    }
    let (vid, admin) = (user.id, user.es_admin());
    let cuenta = db::run(&pool, move |conn| ml_services::get_cuenta(conn, cuenta_id, vid, admin)).await?;
    let token = crate::sync::access_token_valido(&pool, &client, &cuenta).await?;
    client.claim_send_message(&token, claim_id, &texto).await?;
    Ok(HttpResponse::Ok().json(json!({ "ok": true })))
}
