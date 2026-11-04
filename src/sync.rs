//! Lógica de sincronización reutilizable por el controlador (manual), el
//! webhook (en vivo) y el job periódico (red de seguridad).

use crate::db::{self, DbPool};
use crate::errors::ApiResult;
use crate::ml::MlClient;
use crate::models::MlCuenta;
use crate::services::common::now;
use crate::services::{ml_services as svc, producto_services as prod_svc};
use actix_web::web;
use chrono::Duration;

/// Devuelve un access_token válido, renovándolo (y persistiéndolo) si expira pronto.
pub async fn access_token_valido(pool: &web::Data<DbPool>, client: &MlClient, cuenta: &MlCuenta) -> ApiResult<String> {
    if cuenta.token_expira_at > now() + Duration::seconds(60) {
        return Ok(cuenta.access_token.clone());
    }
    let tokens = client.refresh(&cuenta.refresh_token).await?;
    let expira = now() + Duration::seconds(tokens.expires_in.max(0));
    let refresh = if tokens.refresh_token.is_empty() { cuenta.refresh_token.clone() } else { tokens.refresh_token.clone() };
    let access = tokens.access_token.clone();
    let cuenta_id = cuenta.id;
    let (a, r) = (access.clone(), refresh.clone());
    db::run(pool, move |conn| svc::actualizar_tokens(conn, cuenta_id, &a, &r, expira)).await?;
    Ok(access)
}

/// Importa/actualiza TODAS las publicaciones de una cuenta. Devuelve cuántas.
pub async fn sincronizar_cuenta(pool: &web::Data<DbPool>, client: &MlClient, cuenta: &MlCuenta) -> ApiResult<usize> {
    let token = access_token_valido(pool, client, cuenta).await?;
    let mut offset = 0i64;
    let mut total = 0usize;
    loop {
        let search = client.list_item_ids(&token, cuenta.ml_user_id, offset).await?;
        if search.results.is_empty() {
            break;
        }
        for chunk in search.results.chunks(20) {
            let items = client.get_items(&token, chunk).await?;
            let cuenta_id = cuenta.id;
            total += db::run(pool, move |conn| prod_svc::upsert_items(conn, cuenta_id, &items)).await?;
        }
        offset += search.results.len() as i64;
        if offset >= search.paging.total || search.results.len() < 50 {
            break;
        }
    }
    let cuenta_id = cuenta.id;
    db::run(pool, move |conn| svc::marcar_sync(conn, cuenta_id)).await?;
    Ok(total)
}

/// Actualiza una sola publicación (usado por el webhook).
pub async fn refrescar_item(pool: &web::Data<DbPool>, client: &MlClient, cuenta: &MlCuenta, item_id: &str) -> ApiResult<usize> {
    let token = access_token_valido(pool, client, cuenta).await?;
    let items = client.get_items(&token, &[item_id.to_string()]).await?;
    let cuenta_id = cuenta.id;
    db::run(pool, move |conn| prod_svc::upsert_items(conn, cuenta_id, &items)).await
}
