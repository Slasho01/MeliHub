//! Cupones de descuento globales (los administra el súper-admin). Un cupón reduce
//! el precio de la suscripción (porcentaje o monto fijo).

use crate::errors::{ApiError, ApiResult};
use crate::models::{Cupon, NewCupon};
use crate::schema::cupones;
use crate::services::common::now;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize)]
pub struct Descuento {
    pub codigo: String,
    pub tipo: String,
    pub valor: i64,
    pub descripcion: String,
}

fn normalizar(codigo: &str) -> String {
    codigo.trim().to_uppercase()
}

fn descripcion(c: &Cupon) -> String {
    if c.tipo == "porcentaje" {
        format!("{}% de descuento", c.valor)
    } else {
        format!("{} CLP de descuento", c.valor)
    }
}

/// Valida un cupón por código (activo, no vencido, con usos disponibles).
pub fn validar(conn: &mut PgConnection, codigo: &str) -> ApiResult<Cupon> {
    let codigo = normalizar(codigo);
    if codigo.is_empty() {
        return Err(ApiError::bad_request("Código vacío"));
    }
    let c: Option<Cupon> = cupones::table
        .filter(cupones::codigo.eq(&codigo))
        .select(Cupon::as_select())
        .first(conn)
        .optional()?;
    let c = c.ok_or_else(|| ApiError::bad_request("Código de descuento inválido"))?;
    if !c.activo {
        return Err(ApiError::bad_request("Ese código ya no está activo"));
    }
    if let Some(v) = c.vence {
        if v < now() {
            return Err(ApiError::bad_request("Ese código ya venció"));
        }
    }
    if let Some(max) = c.usos_max {
        if c.usos >= max {
            return Err(ApiError::bad_request("Ese código alcanzó su límite de usos"));
        }
    }
    Ok(c)
}

/// Aplica el descuento del cupón a un precio (nunca baja de 0).
pub fn aplicar(precio: i64, c: &Cupon) -> i64 {
    let desc = if c.tipo == "porcentaje" {
        precio * c.valor.clamp(0, 100) / 100
    } else {
        c.valor.max(0)
    };
    (precio - desc).max(0)
}

/// Valida un cupón y devuelve su info (para mostrar en el panel).
pub fn info(conn: &mut PgConnection, codigo: &str) -> ApiResult<Descuento> {
    let c = validar(conn, codigo)?;
    Ok(Descuento { codigo: c.codigo.clone(), tipo: c.tipo.clone(), valor: c.valor, descripcion: descripcion(&c) })
}

/// Marca un uso del cupón (tras concretar el pago).
pub fn consumir(conn: &mut PgConnection, cupon_id: Uuid) -> ApiResult<()> {
    diesel::update(cupones::table.find(cupon_id))
        .set((cupones::usos.eq(cupones::usos + 1), cupones::updated_at.eq(now())))
        .execute(conn)?;
    Ok(())
}

// ----------------------------- CRUD súper-admin -----------------------------

pub fn listar(conn: &mut PgConnection) -> ApiResult<Vec<Cupon>> {
    cupones::table.order(cupones::created_at.desc()).select(Cupon::as_select()).load(conn).map_err(ApiError::from)
}

pub fn crear(conn: &mut PgConnection, codigo: &str, tipo: &str, valor: i64, vence: Option<NaiveDateTime>, usos_max: Option<i32>) -> ApiResult<Cupon> {
    let codigo = normalizar(codigo);
    if codigo.is_empty() || codigo.len() > 40 {
        return Err(ApiError::bad_request("Código inválido"));
    }
    if tipo != "porcentaje" && tipo != "monto" {
        return Err(ApiError::bad_request("Tipo inválido (porcentaje | monto)"));
    }
    if valor <= 0 || (tipo == "porcentaje" && valor > 100) {
        return Err(ApiError::bad_request("Valor inválido"));
    }
    diesel::insert_into(cupones::table)
        .values(NewCupon { codigo, tipo: tipo.to_string(), valor, vence, usos_max })
        .returning(Cupon::as_returning())
        .get_result(conn)
        .map_err(|e| match ApiError::from(e) {
            ApiError::Conflict(_) => ApiError::conflict("Ya existe un cupón con ese código"),
            other => other,
        })
}

pub fn set_activo(conn: &mut PgConnection, id: Uuid, activo: bool) -> ApiResult<()> {
    let n = diesel::update(cupones::table.find(id))
        .set((cupones::activo.eq(activo), cupones::updated_at.eq(now())))
        .execute(conn)?;
    if n == 0 {
        return Err(ApiError::not_found("Cupón"));
    }
    Ok(())
}

pub fn eliminar(conn: &mut PgConnection, id: Uuid) -> ApiResult<()> {
    let n = diesel::delete(cupones::table.find(id)).execute(conn)?;
    if n == 0 {
        return Err(ApiError::not_found("Cupón"));
    }
    Ok(())
}
