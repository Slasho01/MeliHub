//! Sincronización periódica (red de seguridad, además del webhook en vivo).
//! Cada `SYNC_INTERVAL_SECS` revisa las cuentas que no se sincronizan hace
//! más de `SYNC_MAX_HORAS` y las vuelve a traer de Mercado Libre.

use crate::db::{self, DbPool};
use crate::falabella::FalabellaClient;
use crate::ml::MlClient;
use crate::services::ml_services as svc;
use actix_web::web;
use std::time::Duration;

const SYNC_MAX_HORAS: i64 = 6;

/// Empuja el precio/stock ACTUAL del hub a Falabella para todos los productos publicados
/// (sincronización continua). Best-effort: errores solo al log.
async fn sincronizar_falabella(pool: &web::Data<DbPool>, fala: &web::Data<FalabellaClient>) {
    let cuentas = match db::run(pool, |conn| crate::services::falabella_services::todas_cuentas(conn)).await {
        Ok(c) => c,
        Err(_) => return,
    };
    for cuenta in cuentas {
        let cid = cuenta.id;
        let items = match db::run(pool, move |conn| crate::services::falabella_services::publicaciones_para_sync(conn, cid)).await {
            Ok(i) if !i.is_empty() => i,
            _ => continue,
        };
        let api_key = crate::crypto::descifrar(&cuenta.api_key);
        // Falabella acepta lotes grandes; mandamos de a 500 por si acaso.
        for chunk in items.chunks(500) {
            let xml = crate::services::falabella_push_services::build_stock_precio_batch_xml(chunk);
            match fala.product_update(&cuenta.user_id, &api_key, xml).await {
                Ok(_) => log::info!("Falabella {}: {} productos sincronizados (precio/stock)", cuenta.user_id, chunk.len()),
                Err(e) => log::warn!("Falabella {}: error sync precio/stock: {e}", cuenta.user_id),
            }
        }
    }
}

pub fn start(pool: web::Data<DbPool>, client: web::Data<MlClient>, falabella: web::Data<FalabellaClient>, configurado: bool) {
    if !configurado {
        log::info!("Job de sincronización inactivo (Mercado Libre sin configurar)");
        return;
    }
    let secs = std::env::var("SYNC_INTERVAL_SECS").ok().and_then(|v| v.parse::<u64>().ok()).unwrap_or(1800).max(60);
    log::info!("Job de sincronización de Mercado Libre cada {secs}s");
    actix_web::rt::spawn(async move {
        let mut tick = actix_web::rt::time::interval(Duration::from_secs(secs));
        tick.tick().await; // consumir el tick inmediato
        loop {
            tick.tick().await;
            let cuentas = match db::run(&pool, move |conn| svc::cuentas_desactualizadas(conn, SYNC_MAX_HORAS)).await {
                Ok(c) => c,
                Err(e) => {
                    log::warn!("No se pudieron listar cuentas a sincronizar: {e}");
                    continue;
                }
            };
            for cuenta in &cuentas {
                match crate::sync::sincronizar_cuenta(&pool, &client, cuenta).await {
                    Ok(n) => log::info!("Cuenta {} sincronizada: {n} publicaciones", cuenta.ml_user_id),
                    Err(e) => {
                        log::warn!("Error sincronizando cuenta {}: {e}", cuenta.ml_user_id);
                        let (vid, msg) = (cuenta.vendedor_id, e.to_string());
                        let _ = db::run(&pool, move |conn| {
                            crate::services::saas_services::notificar(conn, vid, "sync_error", "Error de sincronización con Mercado Libre", Some(msg));
                            Ok::<_, crate::errors::ApiError>(())
                        })
                        .await;
                    }
                }
                // Órdenes (ventas) — best-effort; requiere permiso de ventas en la app ML.
                match crate::sync::sincronizar_ordenes(&pool, &client, cuenta).await {
                    Ok(n) if n > 0 => log::info!("Cuenta {}: {n} órdenes sincronizadas", cuenta.ml_user_id),
                    Ok(_) => {}
                    Err(e) => log::warn!("Error sincronizando órdenes de {}: {e}", cuenta.ml_user_id),
                }
            }
            // Tras refrescar el hub desde ML, empujamos precio/stock a Falabella.
            sincronizar_falabella(&pool, &falabella).await;
        }
    });
}
