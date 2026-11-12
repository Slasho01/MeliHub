//! Repricing automático de Buy Box (catálogo de Mercado Libre): para cada publicación de
//! catálogo que NO va ganando, iguala el precio al "precio para ganar" SIEMPRE que se respete
//! el margen mínimo configurado por el vendedor. Reutiliza el catálogo + price_to_win + update_price.

use crate::db::{self, DbPool};
use crate::errors::{ApiError, ApiResult};
use crate::ml::MlClient;
use crate::services::{inventario_services as inv, ml_services};
use actix_web::web;
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize, Default)]
pub struct RepriceResumen {
    pub revisados: i64,
    pub repriced: i64,
    pub omitidos_margen: i64,
}

/// Margen % a un precio dado, descontando comisión ML (variable + fija) y costo.
fn margen_pct(precio: i64, costo: i64, com_pct: f64, com_fija: i64) -> f64 {
    if precio <= 0 {
        return 0.0;
    }
    let comision = precio as f64 * com_pct / 100.0 + com_fija as f64;
    (precio as f64 - comision - costo as f64) / precio as f64 * 100.0
}

/// Corre una pasada de repricing para un vendedor. Best-effort (no corta por un error puntual).
pub async fn reprice_vendedor(pool: &web::Data<DbPool>, client: &MlClient, vendedor_id: Uuid) -> ApiResult<RepriceResumen> {
    let (margen_min, com_pct, com_fija, costos, cuentas) = db::run(pool, move |conn| {
        let cfg = inv::config(conn, vendedor_id)?;
        let (_activo, margen_min) = inv::repricing_cfg(conn, vendedor_id)?;
        let costos = inv::costos_por_item(conn, vendedor_id)?;
        let cuentas = ml_services::listar_cuentas(conn, vendedor_id, false)?;
        Ok::<_, ApiError>((margen_min, cfg.comision_ml_pct, cfg.comision_ml_fija, costos, cuentas))
    })
    .await?;

    let mut res = RepriceResumen::default();
    for cuenta in &cuentas {
        let Ok(token) = crate::sync::access_token_valido(pool, client, cuenta).await else { continue };
        let Ok(v) = client.catalog_item_ids(&token, cuenta.ml_user_id, 0).await else { continue };
        let Some(ids) = v.get("results").and_then(|x| x.as_array()) else { continue };
        for id in ids {
            let Some(item_id) = id.as_str() else { continue };
            let Ok(pw) = client.price_to_win(&token, item_id).await else { continue };
            // Si ya gana, no tocar.
            if pw.get("status").and_then(|s| s.as_str()) == Some("winning") {
                continue;
            }
            let ptw = pw.get("price_to_win").and_then(|x| x.as_f64().or_else(|| x.as_i64().map(|n| n as f64)));
            let Some(ptw) = ptw else { continue };
            let ptw = ptw.round() as i64;
            if ptw <= 0 {
                continue;
            }
            res.revisados += 1;
            let costo = costos.get(item_id).copied().unwrap_or(0);
            // Solo bajamos el precio si el "precio para ganar" aún respeta el margen mínimo.
            if margen_pct(ptw, costo, com_pct, com_fija) + 1e-6 >= margen_min {
                if client.update_price(&token, item_id, ptw).await.is_ok() {
                    res.repriced += 1;
                }
            } else {
                res.omitidos_margen += 1;
            }
        }
    }
    Ok(res)
}
