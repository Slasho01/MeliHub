//! Kits / combos: definir productos compuestos y ver su disponibilidad.

use crate::auth::AuthVendedor;
use crate::db::{self, DbPool};
use crate::errors::{ApiError, ApiResult};
use crate::ml::MlClient;
use crate::services::kit_services;
use actix_web::{web, HttpResponse};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct ComponenteReq {
    pub producto_id: Uuid,
    pub cantidad: i32,
}

#[derive(Deserialize)]
pub struct DefinirReq {
    pub kit_producto_id: Uuid,
    pub componentes: Vec<ComponenteReq>,
}

/// Lista los kits del vendedor con componentes y disponibilidad calculada.
pub async fn listar(user: AuthVendedor, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    let vid = user.id;
    let kits = db::run(&pool, move |conn| kit_services::kits_de(conn, vid)).await?;
    Ok(HttpResponse::Ok().json(json!({ "kits": kits })))
}

/// Define (o redefine) los componentes de un kit y empuja el stock resultante a ML.
pub async fn definir(
    user: AuthVendedor,
    body: web::Json<DefinirReq>,
    pool: web::Data<DbPool>,
    client: web::Data<MlClient>,
) -> ApiResult<HttpResponse> {
    user.exigir_edicion()?;
    let DefinirReq { kit_producto_id, componentes } = body.into_inner();
    let comps: Vec<(Uuid, i32)> = componentes.into_iter().map(|c| (c.producto_id, c.cantidad)).collect();
    let vid = user.id;
    let venta = db::run(&pool, move |conn| kit_services::definir_kit(conn, vid, kit_producto_id, comps)).await?;
    if let Some(v) = venta {
        crate::sync::empujar_stock_ml(&pool, &client, &v).await;
    }
    Ok(HttpResponse::Ok().json(json!({ "ok": true })))
}

#[derive(Deserialize)]
pub struct CrearPubReq {
    pub cuenta_id: Uuid,
    pub base_producto_id: Uuid,
    pub titulo: String,
    pub precio: i64,
    pub componentes: Vec<ComponenteReq>,
}

/// Crea el kit como una publicación NUEVA en Mercado Libre: hereda categoría/atributos/fotos
/// de un componente "base" y le pone el título y precio del kit. Luego lo enlaza como kit.
pub async fn crear_publicacion(
    user: AuthVendedor,
    body: web::Json<CrearPubReq>,
    pool: web::Data<DbPool>,
    client: web::Data<MlClient>,
) -> ApiResult<HttpResponse> {
    use crate::services::{clon_services, ml_services, producto_services as prod};
    user.exigir_edicion()?;
    let (vid, admin) = (user.id, user.es_admin());
    let CrearPubReq { cuenta_id, base_producto_id, titulo, precio, componentes } = body.into_inner();
    let titulo = titulo.trim().to_string();
    if titulo.is_empty() {
        return Err(ApiError::bad_request("Ponle un título al kit."));
    }
    if precio <= 0 {
        return Err(ApiError::bad_request("Ponle un precio válido al kit."));
    }
    if componentes.is_empty() {
        return Err(ApiError::bad_request("Agrega al menos un componente."));
    }
    let comps: Vec<(Uuid, i32)> = componentes.iter().map(|c| (c.producto_id, c.cantidad.max(1))).collect();
    let cuenta = db::run(&pool, move |conn| ml_services::get_cuenta(conn, cuenta_id, vid, admin)).await?;
    // Base + componentes deben ser de esa cuenta.
    let mut ids: Vec<Uuid> = comps.iter().map(|c| c.0).collect();
    ids.push(base_producto_id);
    let prods = db::run(&pool, move |conn| prod::por_ids_de_cuenta(conn, cuenta_id, &ids)).await?;
    let base = prods
        .iter()
        .find(|p| p.id == base_producto_id)
        .cloned()
        .ok_or_else(|| ApiError::bad_request("El componente base no pertenece a esa cuenta."))?;
    // Disponibilidad del kit = mínimo de (stock de cada componente ÷ cantidad).
    let mut disponible = i32::MAX;
    for (pid, cant) in &comps {
        let stock = prods.iter().find(|p| p.id == *pid).map(|p| p.stock).unwrap_or(0);
        disponible = disponible.min(stock / cant.max(&1));
    }
    if disponible == i32::MAX {
        disponible = 0;
    }
    // Payload heredado del base, con el título/precio/stock del kit y sin variaciones.
    let armar = |con_familia: bool| -> serde_json::Value {
        let mut p = clon_services::payload(&base, con_familia);
        p["price"] = json!(precio.max(0));
        if let Some(o) = p.as_object_mut() {
            o.remove("variations");
        }
        p["available_quantity"] = json!(disponible.max(0));
        if con_familia {
            p["family_name"] = json!(titulo.clone());
        } else {
            p["title"] = json!(titulo.clone());
        }
        p
    };
    let token = crate::sync::access_token_valido(&pool, &client, &cuenta).await?;
    let mut res = client.crear_item(&token, &armar(false)).await;
    if let Err(e) = &res {
        if e.to_string().contains("family_name") {
            res = client.crear_item(&token, &armar(true)).await;
        }
    }
    let creado = res?;
    let new_id = creado.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
    if new_id.is_empty() {
        return Err(ApiError::internal("Mercado Libre no devolvió el ID de la publicación."));
    }
    // Importa la publicación nueva al hub (crea su producto) y la enlaza como kit.
    crate::sync::refrescar_item(&pool, &client, &cuenta, &new_id).await?;
    let nid = new_id.clone();
    let kit_pid = db::run(&pool, move |conn| ml_services::producto_id_por_item(conn, cuenta_id, &nid))
        .await?
        .ok_or_else(|| ApiError::internal("No se pudo registrar la publicación del kit en el hub."))?;
    let venta = db::run(&pool, move |conn| kit_services::definir_kit(conn, vid, kit_pid, comps)).await?;
    if let Some(v) = venta {
        crate::sync::empujar_stock_ml(&pool, &client, &v).await;
    }
    let permalink = creado.get("permalink").and_then(|v| v.as_str()).map(|s| s.to_string());
    Ok(HttpResponse::Ok().json(json!({ "ok": true, "item_id": new_id, "permalink": permalink })))
}

/// Deshace un kit (quita sus componentes).
pub async fn eliminar(user: AuthVendedor, path: web::Path<Uuid>, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    user.exigir_edicion()?;
    let (vid, kid) = (user.id, path.into_inner());
    db::run(&pool, move |conn| kit_services::eliminar_kit(conn, vid, kid)).await?;
    Ok(HttpResponse::Ok().json(json!({ "ok": true })))
}
