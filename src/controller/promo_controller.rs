//! Promociones y campañas de Mercado Libre, centralizadas de todas las cuentas.

use crate::auth::AuthVendedor;
use crate::config::Config;
use crate::db::{self, DbPool};
use crate::errors::ApiResult;
use crate::ml::MlClient;
use crate::services::ml_services;
use actix_web::{web, HttpResponse};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct PromoQuery {
    pub cuenta: Option<Uuid>,
}

/// Lista las promociones/campañas en las que cada cuenta puede participar.
pub async fn listar(
    user: AuthVendedor,
    q: web::Query<PromoQuery>,
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
        let Ok(v) = client.promociones(&token, cuenta.ml_user_id).await else { continue };
        let arr = v.get("results").or_else(|| v.get("data")).and_then(|x| x.as_array());
        let Some(arr) = arr else { continue };
        for p in arr {
            salida.push(json!({
                "id": p.get("id").and_then(|x| x.as_str()),
                "cuenta_id": cuenta.id,
                "cuenta": cuenta.nickname.clone(),
                "tipo": p.get("type").and_then(|x| x.as_str()).unwrap_or(""),
                "nombre": p.get("name").or_else(|| p.get("description")).and_then(|x| x.as_str()).unwrap_or(""),
                "estado": p.get("status").and_then(|x| x.as_str()).unwrap_or(""),
                "inicio": p.get("start_date").and_then(|x| x.as_str()),
                "fin": p.get("finish_date").and_then(|x| x.as_str()),
                "deal_price": p.pointer("/benefits/type").and_then(|x| x.as_str()),
            }));
        }
    }
    Ok(HttpResponse::Ok().json(salida))
}
