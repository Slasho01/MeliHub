//! Panel de salud de conexiones: estado de cada integración (ML, Falabella, Paris) del vendedor,
//! con última sincronización y señales para avisar antes de que algo falle.

use crate::auth::AuthVendedor;
use crate::db::{self, DbPool};
use crate::errors::{ApiError, ApiResult};
use crate::schema::productos;
use crate::services::common::now;
use crate::services::{falabella_services, ml_services, paris_services};
use actix_web::{web, HttpResponse};
use diesel::prelude::*;
use serde_json::{json, Value};

fn humano(horas: i64) -> String {
    if horas < 1 {
        "menos de 1 h".into()
    } else if horas < 24 {
        format!("{horas} h")
    } else {
        format!("{} día{}", horas / 24, if horas / 24 == 1 { "" } else { "s" })
    }
}

/// Devuelve el estado de todas las conexiones del vendedor (ML + Falabella + Paris).
pub async fn conexiones(user: AuthVendedor, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    let (vid, admin) = (user.id, user.es_admin());
    let salida = db::run(&pool, move |conn| {
        let ahora = now();
        let mut out: Vec<Value> = Vec::new();

        // --- Mercado Libre ---
        for c in ml_services::listar_cuentas(conn, vid, admin)? {
            let n: i64 = productos::table.filter(productos::ml_cuenta_id.eq(c.id)).count().get_result(conn).unwrap_or(0);
            let (estado, detalle) = if !c.activa {
                ("desconectada", "Cuenta desconectada — reconéctala.".to_string())
            } else {
                match c.ultima_sync_at {
                    None => ("pendiente", "Nunca sincronizada.".to_string()),
                    Some(t) => {
                        let horas = (ahora - t).num_hours().max(0);
                        if horas > 48 {
                            ("atrasada", format!("Última sincronización hace {}.", humano(horas)))
                        } else {
                            ("ok", format!("Sincronizada hace {}.", humano(horas)))
                        }
                    }
                }
            };
            out.push(json!({
                "tipo": "Mercado Libre",
                "nombre": c.nickname.clone().unwrap_or_else(|| format!("Usuario {}", c.ml_user_id)),
                "estado": estado,
                "detalle": detalle,
                "productos": n,
                "ultima_sync_at": c.ultima_sync_at,
            }));
        }

        // --- Falabella ---
        for f in falabella_services::listar(conn, vid)? {
            out.push(json!({
                "tipo": "Falabella",
                "nombre": f.nickname.clone().unwrap_or_else(|| f.user_id.clone()),
                "estado": if f.activa { "ok" } else { "desconectada" },
                "detalle": if f.activa { "Conectada." } else { "Desconectada." },
                "productos": Value::Null,
                "ultima_sync_at": Value::Null,
            }));
        }

        // --- Paris ---
        for p in paris_services::listar(conn, vid)? {
            out.push(json!({
                "tipo": "Paris",
                "nombre": format!("{}{}", p.nombre.clone().unwrap_or_else(|| "Cuenta Paris".into()), if p.sandbox { " (sandbox)" } else { "" }),
                "estado": if p.activa { "ok" } else { "desconectada" },
                "detalle": if p.activa { "Conectada." } else { "Desconectada." },
                "productos": Value::Null,
                "ultima_sync_at": Value::Null,
            }));
        }

        Ok::<_, ApiError>(out)
    })
    .await?;
    Ok(HttpResponse::Ok().json(salida))
}
