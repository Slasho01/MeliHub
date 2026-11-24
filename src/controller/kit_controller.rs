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
    use crate::services::{ml_services, producto_services as prod};
    user.exigir_edicion()?;
    let (vid, admin) = (user.id, user.es_admin());
    let CrearPubReq { cuenta_id, base_producto_id: _, titulo, precio, componentes } = body.into_inner();
    let titulo = titulo.trim().to_string();
    if titulo.is_empty() {
        return Err(ApiError::bad_request("Ponle un título al kit."));
    }
    if componentes.len() < 2 {
        return Err(ApiError::bad_request("Un kit de Mercado Libre necesita al menos 2 productos distintos."));
    }
    if componentes.len() > 6 {
        return Err(ApiError::bad_request("Un kit de Mercado Libre admite máximo 6 productos distintos."));
    }
    let comps: Vec<(Uuid, i32)> = componentes.iter().map(|c| (c.producto_id, c.cantidad.clamp(1, 10))).collect();
    let cuenta = db::run(&pool, move |conn| ml_services::get_cuenta(conn, cuenta_id, vid, admin)).await?;
    let ids: Vec<Uuid> = comps.iter().map(|c| c.0).collect();
    let prods = db::run(&pool, move |conn| prod::por_ids_de_cuenta(conn, cuenta_id, &ids)).await?;
    // Componentes como (ml_item_id, cantidad) para el kit NATIVO de ML.
    let mut componentes_ml: Vec<(String, i32)> = Vec::new();
    for (pid, cant) in &comps {
        let p = prods
            .iter()
            .find(|p| p.id == *pid)
            .ok_or_else(|| ApiError::bad_request("Un componente no pertenece a esa cuenta."))?;
        componentes_ml.push((p.ml_item_id.clone(), *cant));
    }
    let token = crate::sync::access_token_valido(&pool, &client, &cuenta).await?;
    let precio_opt = if precio > 0 { Some(precio) } else { None };
    // Crea el kit nativo: ML genera título/descripción y sincroniza su stock desde los componentes.
    let creado = client.crear_kit(&token, &titulo, &componentes_ml, precio_opt).await?;
    let new_id = creado.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
    if new_id.is_empty() {
        return Err(ApiError::internal("Mercado Libre no devolvió el ID del kit."));
    }
    // Importa el kit al hub (ML ya maneja su stock nativo) y lo registra como kit para verlo
    // y propagarlo a otros canales. No se empuja stock a ML: lo gestiona ML nativamente.
    let _ = crate::sync::refrescar_item(&pool, &client, &cuenta, &new_id).await;
    let nid = new_id.clone();
    if let Ok(Some(kit_pid)) = db::run(&pool, move |conn| ml_services::producto_id_por_item(conn, cuenta_id, &nid)).await {
        let _ = db::run(&pool, move |conn| kit_services::definir_kit(conn, vid, kit_pid, comps)).await;
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
