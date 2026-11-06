//! Registro de auditoría: acciones relevantes del equipo por organización.
//! `log` es best-effort (los controladores ignoran su error para no bloquear la acción).

use crate::errors::{ApiError, ApiResult};
use crate::models::{Auditoria, NewAuditoria};
use crate::schema::{auditoria, miembros, vendedores};
use diesel::prelude::*;
use uuid::Uuid;

fn nombre_actor(conn: &mut PgConnection, org: Uuid, actor_id: Uuid) -> Option<String> {
    if actor_id == org {
        vendedores::table.find(actor_id).select(vendedores::nombre).first::<String>(conn).ok()
    } else {
        miembros::table.find(actor_id).select(miembros::nombre).first::<String>(conn).ok()
    }
}

pub fn log(conn: &mut PgConnection, org: Uuid, actor_id: Uuid, accion: String, detalle: String) -> ApiResult<()> {
    let actor = nombre_actor(conn, org, actor_id);
    let detalle = if detalle.trim().is_empty() { None } else { Some(detalle.chars().take(400).collect()) };
    diesel::insert_into(auditoria::table)
        .values(NewAuditoria { vendedor_id: org, actor_id: Some(actor_id), actor, accion, detalle })
        .execute(conn)?;
    Ok(())
}

pub fn listar(conn: &mut PgConnection, org: Uuid, limit: i64) -> ApiResult<Vec<Auditoria>> {
    auditoria::table
        .filter(auditoria::vendedor_id.eq(org))
        .order(auditoria::created_at.desc())
        .limit(limit.clamp(1, 500))
        .select(Auditoria::as_select())
        .load(conn)
        .map_err(ApiError::from)
}
