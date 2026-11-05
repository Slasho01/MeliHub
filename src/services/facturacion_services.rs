//! Datos de facturación por organización (vendedor), 1:1 con upsert. Se exigen
//! al suscribirse a un plan de pago.

use crate::errors::{ApiError, ApiResult};
use crate::models::{Facturacion, NewFacturacion};
use crate::schema::facturacion;
use crate::services::common::now;
use diesel::prelude::*;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct FacturacionInput {
    pub tipo_documento: String,
    pub rut: String,
    pub razon_social: String,
    #[serde(default)]
    pub giro: Option<String>,
    pub direccion: String,
    pub comuna: String,
    #[serde(default)]
    pub ciudad: Option<String>,
    #[serde(default)]
    pub telefono: Option<String>,
    #[serde(default)]
    pub pais: Option<String>,
}

fn opt(s: Option<String>) -> Option<String> {
    s.map(|x| x.trim().to_string()).filter(|x| !x.is_empty())
}

pub fn get(conn: &mut PgConnection, org_id: Uuid) -> ApiResult<Option<Facturacion>> {
    facturacion::table.find(org_id).select(Facturacion::as_select()).first(conn).optional().map_err(ApiError::from)
}

/// ¿Están completos los datos mínimos para facturar?
pub fn completo(conn: &mut PgConnection, org_id: Uuid) -> ApiResult<bool> {
    Ok(get(conn, org_id)?
        .map(|f| {
            !f.rut.is_empty()
                && !f.razon_social.is_empty()
                && !f.direccion.is_empty()
                && !f.comuna.is_empty()
                && (f.tipo_documento != "factura" || f.giro.as_deref().map(|g| !g.is_empty()).unwrap_or(false))
        })
        .unwrap_or(false))
}

pub fn upsert(conn: &mut PgConnection, org_id: Uuid, inp: FacturacionInput) -> ApiResult<Facturacion> {
    let tipo = if inp.tipo_documento == "factura" { "factura" } else { "boleta" };
    let rut = inp.rut.trim().to_string();
    let razon = inp.razon_social.trim().to_string();
    let dir = inp.direccion.trim().to_string();
    let comuna = inp.comuna.trim().to_string();
    if rut.is_empty() || razon.is_empty() || dir.is_empty() || comuna.is_empty() {
        return Err(ApiError::bad_request("Completa RUT, razón social, dirección y comuna"));
    }
    let giro = opt(inp.giro);
    if tipo == "factura" && giro.is_none() {
        return Err(ApiError::bad_request("Para factura, el giro es obligatorio"));
    }
    let new = NewFacturacion {
        vendedor_id: org_id,
        tipo_documento: tipo.to_string(),
        rut,
        razon_social: razon,
        giro,
        direccion: dir,
        comuna,
        ciudad: opt(inp.ciudad),
        telefono: opt(inp.telefono),
        pais: opt(inp.pais).unwrap_or_else(|| "Chile".to_string()),
    };
    diesel::insert_into(facturacion::table)
        .values(&new)
        .on_conflict(facturacion::vendedor_id)
        .do_update()
        .set((
            facturacion::tipo_documento.eq(&new.tipo_documento),
            facturacion::rut.eq(&new.rut),
            facturacion::razon_social.eq(&new.razon_social),
            facturacion::giro.eq(&new.giro),
            facturacion::direccion.eq(&new.direccion),
            facturacion::comuna.eq(&new.comuna),
            facturacion::ciudad.eq(&new.ciudad),
            facturacion::telefono.eq(&new.telefono),
            facturacion::pais.eq(&new.pais),
            facturacion::updated_at.eq(now()),
        ))
        .returning(Facturacion::as_returning())
        .get_result(conn)
        .map_err(ApiError::from)
}
