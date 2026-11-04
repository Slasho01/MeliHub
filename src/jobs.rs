//! Sincronización periódica (red de seguridad, además del webhook en vivo).
//! Cada `SYNC_INTERVAL_SECS` revisa las cuentas que no se sincronizan hace
//! más de `SYNC_MAX_HORAS` y las vuelve a traer de Mercado Libre.

use crate::db::{self, DbPool};
use crate::ml::MlClient;
use crate::services::ml_services as svc;
use actix_web::web;
use std::time::Duration;

const SYNC_MAX_HORAS: i64 = 6;

pub fn start(pool: web::Data<DbPool>, client: web::Data<MlClient>, configurado: bool) {
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
                    Err(e) => log::warn!("Error sincronizando cuenta {}: {e}", cuenta.ml_user_id),
                }
            }
        }
    });
}
