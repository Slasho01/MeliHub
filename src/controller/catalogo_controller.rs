//! Publicaciones de catálogo de Mercado Libre: estado de competencia (Buy Box) de los ítems
//! de catálogo del vendedor, y acción para igualar el precio y ganar. No crea publicaciones.

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
pub struct CatalogoQuery {
    /// Filtra a una sola cuenta ML; si falta, trae las de todas.
    pub cuenta: Option<Uuid>,
}

fn num(v: &Value) -> Option<f64> {
    v.as_f64().or_else(|| v.as_i64().map(|n| n as f64))
}

/// Lista las publicaciones de catálogo del vendedor con su estado de competencia (si gana o
/// pierde la Buy Box, precio del ganador y precio para ganar). Best-effort por cuenta/ítem.
pub async fn listar(
    user: AuthVendedor,
    q: web::Query<CatalogoQuery>,
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
        let Ok(v) = client.catalog_item_ids(&token, cuenta.ml_user_id, 0).await else { continue };
        let Some(ids) = v.get("results").and_then(|x| x.as_array()) else { continue };
        for id in ids {
            let Some(item_id) = id.as_str() else { continue };
            // Estado de competencia del ítem (si falla, lo incluimos con estado desconocido).
            let pw = client.price_to_win(&token, item_id).await.unwrap_or(Value::Null);
            let estado = pw.get("status").and_then(|x| x.as_str()).unwrap_or("unknown").to_string();
            let mi_precio = pw.get("current_price").and_then(num);
            let precio_ganador = pw.pointer("/winner/price").and_then(num).or_else(|| pw.get("price_to_win").and_then(num));
            let precio_para_ganar = pw.get("price_to_win").and_then(num);
            item_ids.push(item_id.to_string());
            salida.push(json!({
                "item_id": item_id,
                "cuenta_id": cuenta.id,
                "cuenta": cuenta.nickname.clone(),
                "estado": estado,
                "mi_precio": mi_precio,
                "precio_ganador": precio_ganador,
                "precio_para_ganar": precio_para_ganar,
                "catalog_product_id": pw.get("catalog_product_id").and_then(|x| x.as_str()),
            }));
        }
    }

    // Nombre + foto del producto desde la base local (sin llamar a ML por cada ítem).
    let ids = item_ids.clone();
    let fichas = db::run(&pool, move |conn| Ok::<_, ApiError>(producto_services::fichas_por_item(conn, vid, &ids))).await?;
    for row in salida.iter_mut() {
        let item = row.get("item_id").and_then(|x| x.as_str()).unwrap_or("").to_string();
        match fichas.get(&item) {
            Some((titulo, thumb)) => {
                row["producto"] = json!(titulo);
                row["thumbnail"] = json!(thumb);
            }
            None => row["producto"] = json!(item),
        }
    }
    Ok(HttpResponse::Ok().json(salida))
}

#[derive(Deserialize)]
pub struct IgualarBody {
    pub cuenta_id: Uuid,
    pub item_id: String,
    /// Precio al que se quiere dejar la publicación (normalmente el "precio para ganar").
    pub precio: i64,
}

/// Cambia el precio de una publicación de catálogo en ML (p. ej. para igualar al ganador).
/// Es un update de precio — NO crea publicaciones. Requiere permiso de edición.
pub async fn igualar_precio(
    user: AuthVendedor,
    body: web::Json<IgualarBody>,
    pool: web::Data<DbPool>,
    client: web::Data<MlClient>,
    cfg: web::Data<Config>,
) -> ApiResult<HttpResponse> {
    user.exigir_edicion()?;
    if !cfg.ml.configurado() {
        return Err(ApiError::bad_request("Mercado Libre no está configurado."));
    }
    let IgualarBody { cuenta_id, item_id, precio } = body.into_inner();
    if precio <= 0 {
        return Err(ApiError::bad_request("Precio inválido."));
    }
    let (vid, admin) = (user.id, user.es_admin());
    let cuenta = db::run(&pool, move |conn| ml_services::get_cuenta(conn, cuenta_id, vid, admin)).await?;
    let token = crate::sync::access_token_valido(&pool, &client, &cuenta).await?;
    client.update_price(&token, &item_id, precio).await?;
    Ok(HttpResponse::Ok().json(json!({ "ok": true })))
}
