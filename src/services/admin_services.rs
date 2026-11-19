//! Súper-admin (dueño de la plataforma): gestiona a los vendedores/clientes.
//! Solo accesible por el rol admin.

use crate::errors::{ApiError, ApiResult};
use crate::models::{NewBoleta, Vendedor};
use crate::schema::{boletas, ml_cuentas, ordenes_ml, productos, suscripciones, vendedores};
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
// ----------------------------- Suscripciones + boletas -----------------------------

#[derive(Serialize, Clone)]
pub struct BoletaMeta {
    pub id: Uuid,
    pub periodo: String,
    pub nombre_archivo: String,
    pub subida_at: chrono::NaiveDateTime,
}

#[derive(Serialize)]
pub struct SuscripcionAdmin {
    pub vendedor_id: Uuid,
    pub email: String,
    pub nombre: String,
    pub plan: String,
    pub estado: String,
    pub ciclo: String,
    pub monto: i64,
    pub vigente_hasta: Option<chrono::NaiveDateTime>,
    pub created_at: chrono::NaiveDateTime,
    pub boletas: Vec<BoletaMeta>,
}

/// Lista todas las suscripciones (con el vendedor) y sus boletas subidas (metadatos, sin archivo).
pub fn listar_suscripciones(conn: &mut PgConnection) -> ApiResult<Vec<SuscripcionAdmin>> {
    let rows: Vec<(Uuid, String, String, String, String, String, i64, Option<chrono::NaiveDateTime>, chrono::NaiveDateTime)> = suscripciones::table
        .inner_join(vendedores::table)
        .order(suscripciones::created_at.desc())
        .select((
            suscripciones::vendedor_id,
            vendedores::email,
            vendedores::nombre,
            suscripciones::plan,
            suscripciones::estado,
            suscripciones::ciclo,
            suscripciones::monto,
            suscripciones::vigente_hasta,
            suscripciones::created_at,
        ))
        .load(conn)?;
    let bols: Vec<(Uuid, Uuid, String, String, chrono::NaiveDateTime)> = boletas::table
        .order(boletas::subida_at.desc())
        .select((boletas::id, boletas::vendedor_id, boletas::periodo, boletas::nombre_archivo, boletas::subida_at))
        .load(conn)?;
    let mut bmap: HashMap<Uuid, Vec<BoletaMeta>> = HashMap::new();
    for (id, vid, periodo, nombre_archivo, subida_at) in bols {
        bmap.entry(vid).or_default().push(BoletaMeta { id, periodo, nombre_archivo, subida_at });
    }
    Ok(rows
        .into_iter()
        .map(|(vid, email, nombre, plan, estado, ciclo, monto, vigente_hasta, created_at)| SuscripcionAdmin {
            boletas: bmap.get(&vid).cloned().unwrap_or_default(),
            vendedor_id: vid,
            email,
            nombre,
            plan,
            estado,
            ciclo,
            monto,
            vigente_hasta,
            created_at,
        })
        .collect())
}

/// Guarda una boleta/factura (archivo como data-URL base64) para un vendedor+período.
pub fn guardar_boleta(conn: &mut PgConnection, vendedor_id: Uuid, periodo: &str, nombre_archivo: &str, archivo: &str) -> ApiResult<Uuid> {
    let periodo = periodo.trim();
    if periodo.is_empty() {
        return Err(ApiError::bad_request("Indica el período (ej. 2026-11 o 2026)."));
    }
    if !archivo.starts_with("data:") {
        return Err(ApiError::bad_request("Archivo inválido."));
    }
    // El vendedor debe existir.
    let existe: bool = diesel::select(diesel::dsl::exists(vendedores::table.find(vendedor_id))).get_result(conn)?;
    if !existe {
        return Err(ApiError::not_found("Vendedor"));
    }
    diesel::insert_into(boletas::table)
        .values(NewBoleta {
            vendedor_id,
            periodo: periodo.to_string(),
            nombre_archivo: nombre_archivo.chars().take(255).collect(),
            archivo: archivo.to_string(),
        })
        .returning(boletas::id)
        .get_result(conn)
        .map_err(ApiError::from)
}

/// Devuelve (nombre_archivo, data-URL) de una boleta para descargarla.
pub fn boleta_archivo(conn: &mut PgConnection, id: Uuid) -> ApiResult<(String, String)> {
    boletas::table
        .find(id)
        .select((boletas::nombre_archivo, boletas::archivo))
        .first(conn)
        .optional()?
        .ok_or_else(|| ApiError::not_found("Boleta"))
}

pub fn eliminar_boleta(conn: &mut PgConnection, id: Uuid) -> ApiResult<()> {
    diesel::delete(boletas::table.find(id)).execute(conn)?;
    Ok(())
}

/// Diagnóstico de propagación: cuenta publicaciones de canal cuyo `producto_id` ya NO existe
/// (huérfanas) — pasa si se recrearon los productos (p. ej. reconectar una cuenta de ML),
/// lo que rompe TODA la sincronización de stock a ese canal.
pub fn diagnostico(conn: &mut PgConnection) -> ApiResult<serde_json::Value> {
    use crate::schema::{falabella_publicaciones, paris_publicaciones, productos, shopify_publicaciones};
    use std::collections::HashSet;
    let prod_ids: HashSet<Uuid> = productos::table.select(productos::id).load::<Uuid>(conn)?.into_iter().collect();
    let cuenta = |pids: Vec<Uuid>| -> (usize, usize) {
        let total = pids.len();
        let huerfanas = pids.iter().filter(|id| !prod_ids.contains(id)).count();
        (total, huerfanas)
    };
    let (f_t, f_h) = cuenta(falabella_publicaciones::table.select(falabella_publicaciones::producto_id).load(conn)?);
    let (s_t, s_h) = cuenta(shopify_publicaciones::table.select(shopify_publicaciones::producto_id).load(conn)?);
    let (p_t, p_h) = cuenta(paris_publicaciones::table.select(paris_publicaciones::producto_id).load(conn)?);
    Ok(serde_json::json!({
        "productos": prod_ids.len(),
        "falabella": { "total": f_t, "huerfanas": f_h },
        "shopify": { "total": s_t, "huerfanas": s_h },
        "paris": { "total": p_t, "huerfanas": p_h },
    }))
}

/// Correos que se dieron de baja de los envíos de promoción (lista de supresión).
pub fn bajas_emails(conn: &mut PgConnection) -> ApiResult<Vec<String>> {
    use crate::schema::promo_bajas;
    promo_bajas::table.select(promo_bajas::email).load::<String>(conn).map_err(ApiError::from)
}

/// Registra una baja (idempotente: si ya estaba, no falla).
pub fn registrar_baja(conn: &mut PgConnection, email: &str) -> ApiResult<()> {
    use crate::schema::promo_bajas;
    diesel::insert_into(promo_bajas::table)
        .values(crate::models::NewPromoBaja { email: email.trim().to_lowercase() })
        .on_conflict(promo_bajas::email)
        .do_nothing()
        .execute(conn)?;
    Ok(())
}

/// (nombre, email) de todas las cuentas activas — para envíos de promoción.
pub fn emails_promo(conn: &mut PgConnection) -> ApiResult<Vec<(String, String)>> {
    vendedores::table
        .filter(vendedores::activo.eq(true))
        .select((vendedores::nombre, vendedores::email))
        .order(vendedores::created_at.asc())
        .load::<(String, String)>(conn)
        .map_err(ApiError::from)
}

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
