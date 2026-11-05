//! Súper-admin (dueño de la plataforma): gestiona a los vendedores/clientes.
//! Solo accesible por el rol admin.

use crate::errors::{ApiError, ApiResult};
use crate::models::Vendedor;
use crate::schema::{ml_cuentas, ordenes_ml, productos, vendedores};
use diesel::dsl::count_star;
use diesel::prelude::*;
use serde::Serialize;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Serialize)]
pub struct VendedorAdmin {
    #[serde(flatten)]
    pub vendedor: Vendedor,
    pub cuentas: i64,
    pub productos: i64,
}

/// Lista todos los vendedores con su uso (cuentas ML y productos).
pub fn listar_vendedores(conn: &mut PgConnection) -> ApiResult<Vec<VendedorAdmin>> {
    let vends: Vec<Vendedor> = vendedores::table
        .order(vendedores::created_at.asc())
        .select(Vendedor::as_select())
        .load(conn)?;

    let cuentas: Vec<(Uuid, i64)> = ml_cuentas::table
        .group_by(ml_cuentas::vendedor_id)
        .select((ml_cuentas::vendedor_id, count_star()))
        .load(conn)?;
    let cmap: HashMap<Uuid, i64> = cuentas.into_iter().collect();

    let prods: Vec<(Uuid, i64)> = productos::table
        .inner_join(ml_cuentas::table)
        .group_by(ml_cuentas::vendedor_id)
        .select((ml_cuentas::vendedor_id, count_star()))
        .load(conn)?;
    let pmap: HashMap<Uuid, i64> = prods.into_iter().collect();

    Ok(vends
        .into_iter()
        .map(|v| {
            let id = v.id;
            VendedorAdmin {
                cuentas: *cmap.get(&id).unwrap_or(&0),
                productos: *pmap.get(&id).unwrap_or(&0),
                vendedor: v,
            }
        })
        .collect())
}

/// Cambia el plan de un vendedor (free | pro | business).
pub fn set_plan(conn: &mut PgConnection, vendedor_id: Uuid, plan: &str) -> ApiResult<Vendedor> {
    if !crate::planes::PLANES.iter().any(|p| p.id == plan) {
        return Err(ApiError::bad_request("Plan inválido"));
    }
    diesel::update(vendedores::table.find(vendedor_id))
        .set((vendedores::plan.eq(plan), vendedores::updated_at.eq(crate::services::common::now())))
        .returning(Vendedor::as_returning())
        .get_result(conn)
        .map_err(ApiError::from)
}

/// Suspende o reactiva a un vendedor (si `activo=false` no puede iniciar sesión).
pub fn set_activo(conn: &mut PgConnection, vendedor_id: Uuid, activo: bool) -> ApiResult<Vendedor> {
    diesel::update(vendedores::table.find(vendedor_id))
        .set((vendedores::activo.eq(activo), vendedores::updated_at.eq(crate::services::common::now())))
        .returning(Vendedor::as_returning())
        .get_result(conn)
        .map_err(ApiError::from)
}

#[derive(Serialize)]
pub struct Metricas {
    pub vendedores: i64,
    pub cuentas: i64,
    pub productos: i64,
    pub ordenes: i64,
    pub ventas_total: i64,
    pub por_plan: HashMap<String, i64>,
}

/// Métricas globales de la plataforma.
pub fn metricas(conn: &mut PgConnection) -> ApiResult<Metricas> {
    let vendedores_n: i64 = vendedores::table.count().get_result(conn)?;
    let cuentas: i64 = ml_cuentas::table.count().get_result(conn)?;
    let productos_n: i64 = productos::table.count().get_result(conn)?;
    let ordenes: i64 = ordenes_ml::table.count().get_result(conn)?;
    let ventas: Vec<i64> = ordenes_ml::table
        .filter(ordenes_ml::estado.ne("cancelled"))
        .select(ordenes_ml::venta_total)
        .load(conn)?;
    let ventas_total: i64 = ventas.iter().sum();
    let por_plan_rows: Vec<(String, i64)> = vendedores::table
        .group_by(vendedores::plan)
        .select((vendedores::plan, count_star()))
        .load(conn)?;
    Ok(Metricas {
        vendedores: vendedores_n,
        cuentas,
        productos: productos_n,
        ordenes,
        ventas_total,
        por_plan: por_plan_rows.into_iter().collect(),
    })
}
