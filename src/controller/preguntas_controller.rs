//! Preguntas de compradores en Mercado Libre, centralizadas de todas las cuentas.
//! Se leen en vivo de ML (no se almacenan) y se pueden responder desde el panel.

use crate::auth::AuthVendedor;
use crate::config::Config;
use crate::db::{self, DbPool};
use crate::errors::{ApiError, ApiResult};
use crate::ml::MlClient;
use crate::services::{ml_services, producto_services};
use actix_web::{web, HttpResponse};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct PreguntasQuery {
    /// Filtra a una sola cuenta ML; si falta, trae las de todas.
    pub cuenta: Option<Uuid>,
}

/// Lista las preguntas SIN responder de todas las cuentas (o de una), con el nombre
/// del producto resuelto desde la base local. Best-effort: una cuenta sin token válido
/// o que falle se omite en silencio, el resto sí se devuelve.
pub async fn listar(
    user: AuthVendedor,
    q: web::Query<PreguntasQuery>,
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
    let mut item_ids: Vec<String> = Vec::new();
    for cuenta in &cuentas {
        let Ok(token) = crate::sync::access_token_valido(&pool, &client, cuenta).await else { continue };
        let Ok(v) = client.questions_unanswered(&token, cuenta.ml_user_id, 0).await else { continue };
        let Some(qs) = v.get("questions").and_then(|x| x.as_array()) else { continue };
        for qn in qs {
            let id = qn.get("id").and_then(|x| x.as_i64()).unwrap_or(0);
            if id == 0 {
                continue;
            }
            let item_id = qn.get("item_id").and_then(|x| x.as_str()).unwrap_or("").to_string();
            if !item_id.is_empty() {
                item_ids.push(item_id.clone());
            }
            salida.push(json!({
                "id": id,
                "cuenta_id": cuenta.id,
                "cuenta": cuenta.nickname.clone(),
                "item_id": item_id,
                "texto": qn.get("text").and_then(|x| x.as_str()).unwrap_or(""),
                "fecha": qn.get("date_created").and_then(|x| x.as_str()).unwrap_or(""),
                "comprador_id": qn.get("from").and_then(|f| f.get("id")).and_then(|x| x.as_i64()),
            }));
        }
    }

    // Nombre del producto desde la base local (sin llamar a ML por cada ítem).
    let titulos = db::run(&pool, move |conn| {
        Ok::<_, ApiError>(producto_services::titulos_por_item(conn, vid, &item_ids))
    })
    .await?;
    for q in salida.iter_mut() {
        let item = q.get("item_id").and_then(|x| x.as_str()).unwrap_or("").to_string();
        q["producto"] = json!(titulos.get(&item).cloned().unwrap_or(item));
    }
    Ok(HttpResponse::Ok().json(salida))
}

#[derive(Deserialize)]
pub struct ResponderBody {
    pub cuenta_id: Uuid,
    pub question_id: i64,
    pub texto: String,
}

/// Responde una pregunta en Mercado Libre. Requiere permiso de edición (no solo lectura).
pub async fn responder(
    user: AuthVendedor,
    body: web::Json<ResponderBody>,
    pool: web::Data<DbPool>,
    client: web::Data<MlClient>,
    cfg: web::Data<Config>,
) -> ApiResult<HttpResponse> {
    user.exigir_edicion()?;
    if !cfg.ml.configurado() {
        return Err(ApiError::bad_request("Mercado Libre no está configurado."));
    }
    let ResponderBody { cuenta_id, question_id, texto } = body.into_inner();
    let texto = texto.trim().to_string();
    if texto.is_empty() {
        return Err(ApiError::bad_request("Escribe una respuesta."));
    }
    if texto.chars().count() > 2000 {
        return Err(ApiError::bad_request("La respuesta es demasiado larga (máx. 2000 caracteres)."));
    }
    let (vid, admin) = (user.id, user.es_admin());
    let cuenta = db::run(&pool, move |conn| ml_services::get_cuenta(conn, cuenta_id, vid, admin)).await?;
    let token = crate::sync::access_token_valido(&pool, &client, &cuenta).await?;
    client.answer_question(&token, question_id, &texto).await?;
    Ok(HttpResponse::Ok().json(json!({ "ok": true })))
}
