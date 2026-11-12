//! Sincronización periódica (red de seguridad, además del webhook en vivo).
//! Cada `SYNC_INTERVAL_SECS` revisa las cuentas que no se sincronizan hace
//! más de `SYNC_MAX_HORAS` y las vuelve a traer de Mercado Libre.

use crate::db::{self, DbPool};
use crate::falabella::FalabellaClient;
use crate::ml::MlClient;
use crate::paris::ParisClient;
use crate::services::ml_services as svc;
use actix_web::web;
use std::collections::HashMap;
use std::time::Duration;

const SYNC_MAX_HORAS: i64 = 6;

fn como_lista(v: Option<&serde_json::Value>) -> Vec<serde_json::Value> {
    match v {
        Some(serde_json::Value::Array(a)) => a.clone(),
        Some(o) if o.is_object() => vec![o.clone()],
        _ => Vec::new(),
    }
}

/// Trae las órdenes nuevas de Falabella (últimas 48 h) y DESCUENTA el stock en el hub
/// (y lo empuja a ML) para evitar sobreventa. Las órdenes ya procesadas no se repiten.
async fn sincronizar_ventas_falabella(pool: &web::Data<DbPool>, ml: &web::Data<MlClient>, fala: &web::Data<FalabellaClient>, ml_ok: bool) {
    let cuentas = match db::run(pool, crate::services::falabella_services::todas_cuentas).await {
        Ok(c) => c,
        Err(_) => return,
    };
    for cuenta in cuentas {
        procesar_ventas_cuenta(pool, ml, fala, &cuenta, ml_ok).await;
    }
}

/// Procesa las órdenes recientes de UNA cuenta Falabella (descuenta stock + empuja a ML).
/// Reusable por el job periódico y por el webhook (tiempo real).
pub async fn procesar_ventas_cuenta(pool: &web::Data<DbPool>, ml: &web::Data<MlClient>, fala: &web::Data<FalabellaClient>, cuenta: &crate::models::FalabellaCuenta, ml_ok: bool) {
    {
        let api_key = crate::crypto::descifrar(&cuenta.api_key);
        let desde = (chrono::Utc::now() - chrono::Duration::hours(48)).format("%Y-%m-%dT%H:%M:%S%:z").to_string();
        let ordenes = match fala.get_orders(&cuenta.user_id, &api_key, &desde).await {
            Ok(v) => como_lista(v.pointer("/SuccessResponse/Body/Orders/Order")),
            Err(e) => {
                log::warn!("Falabella {}: no se pudieron traer órdenes: {e}", cuenta.user_id);
                return;
            }
        };
        for ord in ordenes {
            let order_id = ord
                .get("OrderId")
                .and_then(|v| v.as_i64().map(|n| n.to_string()).or_else(|| v.as_str().map(|s| s.to_string())));
            let Some(order_id) = order_id else { continue };
            let (cid, oid) = (cuenta.id, order_id.clone());
            let nueva = db::run(pool, move |conn| crate::services::falabella_services::marcar_venta_si_nueva(conn, &oid, cid)).await.unwrap_or(false);
            if !nueva {
                continue;
            }
            let items = match fala.get_order_items(&cuenta.user_id, &api_key, &order_id).await {
                Ok(v) => como_lista(v.pointer("/SuccessResponse/Body/OrderItems/OrderItem")),
                Err(_) => continue,
            };
            // Cada OrderItem suele ser 1 unidad; agrupamos por SKU.
            let mut conteo: std::collections::HashMap<String, i32> = std::collections::HashMap::new();
            for it in &items {
                if let Some(sku) = it.get("SellerSku").and_then(|v| v.as_str()).filter(|s| !s.is_empty()) {
                    *conteo.entry(sku.to_string()).or_insert(0) += 1;
                }
            }
            let vid = cuenta.vendedor_id;
            let n_items: i32 = conteo.values().sum();
            let msg = format!("Orden {order_id} · {n_items} ítem(s). Marca 'listo para enviar' en Falabella.");
            let _ = db::run(pool, move |conn| {
                crate::services::saas_services::notificar(conn, vid, "falabella_venta", "Nueva venta en Falabella", Some(msg));
                Ok::<_, crate::errors::ApiError>(())
            })
            .await;
            for (sku, qty) in conteo {
                let (cid, sku2) = (cuenta.id, sku.clone());
                let pid = db::run(pool, move |conn| crate::services::falabella_services::producto_de_sku(conn, cid, &sku2)).await.ok().flatten();
                let Some(pid) = pid else { continue };
                if let Ok(venta) = db::run(pool, move |conn| crate::services::pub_services::aplicar_venta_externa(conn, pid, qty, "venta_falabella")).await {
                    log::info!("Falabella venta {order_id}: {sku} -{qty} → stock {}", venta.nuevo_stock);
                    if ml_ok {
                        crate::sync::empujar_stock_ml(pool, ml, &venta).await;
                    }
                }
            }
        }
    }
}

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
                Err(e) => {
                    log::warn!("Falabella {}: error sync precio/stock: {e}", cuenta.user_id);
                    let (vid, msg) = (cuenta.vendedor_id, e.to_string());
                    let _ = db::run(pool, move |conn| {
                        crate::services::saas_services::notificar(conn, vid, "falabella_sync", "Error sincronizando con Falabella", Some(msg));
                        Ok::<_, crate::errors::ApiError>(())
                    })
                    .await;
                }
            }
        }
    }
}

/// Trae las órdenes nuevas de Paris (últimas 48 h) y DESCUENTA el stock en el hub
/// (y lo empuja a ML). Las órdenes ya procesadas no se repiten.
async fn procesar_ventas_paris(pool: &web::Data<DbPool>, ml: &web::Data<MlClient>, paris: &web::Data<ParisClient>, ml_ok: bool) {
    let cuentas = match db::run(pool, crate::services::paris_services::todas_cuentas).await {
        Ok(c) => c,
        Err(_) => return,
    };
    for cuenta in cuentas {
        let api_key = crate::crypto::descifrar(&cuenta.api_key);
        let desde = (chrono::Utc::now() - chrono::Duration::hours(48)).format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
        let query = format!("gteCreatedAt={desde}");
        let ordenes = match paris.get_orders(cuenta.sandbox, &api_key, &query).await {
            Ok(v) => como_lista(v.get("data")),
            Err(e) => {
                log::warn!("Paris {}: no se pudieron traer órdenes: {e}", cuenta.id);
                continue;
            }
        };
        for ord in ordenes {
            let numero = ord
                .get("originOrderNumber")
                .and_then(|v| v.as_str())
                .or_else(|| ord.get("id").and_then(|v| v.as_str()))
                .map(|s| s.to_string());
            let Some(numero) = numero else { continue };
            let (cid, num2) = (cuenta.id, numero.clone());
            let nueva = db::run(pool, move |conn| crate::services::paris_services::marcar_venta_si_nueva(conn, &num2, cid)).await.unwrap_or(false);
            if !nueva {
                continue;
            }
            let detalle = match paris.get_order(cuenta.sandbox, &api_key, &numero).await {
                Ok(v) => v,
                Err(_) => continue,
            };
            // Ítems en subOrders[].items[]; cada ítem suele ser 1 unidad → agrupamos por sellerSku.
            let mut conteo: HashMap<String, i32> = HashMap::new();
            for sub in como_lista(detalle.get("subOrders")) {
                for it in como_lista(sub.get("items")) {
                    let sku = it
                        .get("sellerSku")
                        .and_then(|v| v.as_str())
                        .or_else(|| it.get("sku").and_then(|v| v.as_str()))
                        .filter(|s| !s.is_empty());
                    if let Some(sku) = sku {
                        *conteo.entry(sku.to_string()).or_insert(0) += 1;
                    }
                }
            }
            let vid = cuenta.vendedor_id;
            let n_items: i32 = conteo.values().sum();
            let msg = format!("Orden {numero} · {n_items} ítem(s) en Paris.");
            let _ = db::run(pool, move |conn| {
                crate::services::saas_services::notificar(conn, vid, "paris_venta", "Nueva venta en Paris", Some(msg));
                Ok::<_, crate::errors::ApiError>(())
            })
            .await;
            for (sku, qty) in conteo {
                let (cid, sku2) = (cuenta.id, sku.clone());
                let pid = db::run(pool, move |conn| crate::services::paris_services::producto_de_sku(conn, cid, &sku2)).await.ok().flatten();
                let Some(pid) = pid else { continue };
                if let Ok(venta) = db::run(pool, move |conn| crate::services::pub_services::aplicar_venta_externa(conn, pid, qty, "venta_paris")).await {
                    log::info!("Paris venta {numero}: {sku} -{qty} → stock {}", venta.nuevo_stock);
                    if ml_ok {
                        crate::sync::empujar_stock_ml(pool, ml, &venta).await;
                    }
                }
            }
        }
    }
}

/// Empuja el stock ACTUAL del hub a Paris (por sku_seller) para los productos publicados.
async fn sincronizar_paris(pool: &web::Data<DbPool>, paris: &web::Data<ParisClient>) {
    let cuentas = match db::run(pool, crate::services::paris_services::todas_cuentas).await {
        Ok(c) => c,
        Err(_) => return,
    };
    for cuenta in cuentas {
        let cid = cuenta.id;
        let items = match db::run(pool, move |conn| crate::services::paris_services::publicaciones_para_sync(conn, cid)).await {
            Ok(i) if !i.is_empty() => i,
            _ => continue,
        };
        let api_key = crate::crypto::descifrar(&cuenta.api_key);
        use crate::services::paris_push_services as pp;
        // Store + tipo de precio (una vez por cuenta) para el precio.
        let store = paris.get_store_prices(cuenta.sandbox, &api_key).await.ok().and_then(|v| pp::pick_store(&v));
        let ptype = paris.get_price_types(cuenta.sandbox, &api_key).await.ok().and_then(|v| pp::pick_price_type(&v));
        // STOCK: por sku de la VARIANTE (paris_sku + "-1"); omite los que no tengan paris_sku.
        let stock_items: Vec<(String, i32)> = items
            .iter()
            .filter_map(|i| i.paris_sku.as_ref().map(|ps| (format!("{ps}-1"), i.stock)))
            .collect();
        for chunk in stock_items.chunks(200) {
            let body = pp::build_stock(chunk);
            if let Err(e) = paris.update_stock_sku_seller(cuenta.sandbox, &api_key, &body).await {
                log::warn!("Paris {}: error sync stock: {e}", cuenta.id);
            }
        }
        // PRECIO: por sku del PRODUCTO (paris_sku), con store + tipo.
        if let (Some(st), Some(pt)) = (&store, &ptype) {
            for i in &items {
                if let Some(ps) = &i.paris_sku {
                    let pb = serde_json::json!({ "prices": pp::build_precio(i.precio, Some(st.as_str()), Some(pt.as_str())) });
                    if let Err(e) = paris.update_price(cuenta.sandbox, &api_key, ps, &pb).await {
                        log::warn!("Paris {}: error sync precio {ps}: {e}", cuenta.id);
                    }
                }
            }
        }
    }
}

pub fn start(pool: web::Data<DbPool>, client: web::Data<MlClient>, falabella: web::Data<FalabellaClient>, paris: web::Data<ParisClient>, configurado: bool) {
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
            // Ventas de Falabella → descuentan stock en el hub (y se empuja a ML).
            sincronizar_ventas_falabella(&pool, &client, &falabella, true).await;
            // Tras refrescar el hub desde ML, empujamos precio/stock a Falabella.
            sincronizar_falabella(&pool, &falabella).await;
            // Paris: ventas → descuentan stock (y se empuja a ML), y empujamos stock al canal.
            procesar_ventas_paris(&pool, &client, &paris, true).await;
            sincronizar_paris(&pool, &paris).await;
            // Repricing automático de Buy Box (solo vendedores que lo activaron).
            if let Ok(vends) = db::run(&pool, |conn| crate::services::inventario_services::vendedores_con_repricing(conn)).await {
                for vid in vends {
                    if let Ok(r) = crate::repricing::reprice_vendedor(&pool, &client, vid).await {
                        if r.repriced > 0 {
                            log::info!("Repricing vendedor {vid}: {} repreciados, {} omitidos por margen", r.repriced, r.omitidos_margen);
                        }
                    }
                }
            }
        }
    });
}
