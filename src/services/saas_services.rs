//! SaaS: notificaciones in-app y suscripción por planes (límites suaves).

use crate::errors::{ApiError, ApiResult};
use crate::models::{NewNotificacion, Notificacion, Vendedor};
use crate::planes::{self, Plan};
use crate::schema::{api_keys, ml_cuentas, notificaciones, productos, vendedores};
use diesel::prelude::*;
use serde::Serialize;
use uuid::Uuid;

// ----------------------------- Notificaciones -----------------------------

/// Crea una notificación (ignora si ya hay una pendiente idéntica). Best-effort.
pub fn notificar(conn: &mut PgConnection, vendedor_id: Uuid, tipo: &str, titulo: &str, cuerpo: Option<String>) {
    let _ = diesel::insert_into(notificaciones::table)
        .values(NewNotificacion {
            vendedor_id,
            tipo: tipo.to_string(),
            titulo: titulo.chars().take(255).collect(),
            cuerpo,
        })
        .on_conflict_do_nothing()
        .execute(conn);
}

pub fn listar(conn: &mut PgConnection, vendedor_id: Uuid, solo_no_leidas: bool, limit: i64) -> ApiResult<Vec<Notificacion>> {
    let mut q = notificaciones::table.filter(notificaciones::vendedor_id.eq(vendedor_id)).into_boxed();
    if solo_no_leidas {
        q = q.filter(notificaciones::leida.eq(false));
    }
    q.order(notificaciones::created_at.desc())
        .limit(limit.clamp(1, 200))
        .select(Notificacion::as_select())
        .load(conn)
        .map_err(ApiError::from)
}

pub fn no_leidas(conn: &mut PgConnection, vendedor_id: Uuid) -> ApiResult<i64> {
    notificaciones::table
        .filter(notificaciones::vendedor_id.eq(vendedor_id))
        .filter(notificaciones::leida.eq(false))
        .count()
        .get_result(conn)
        .map_err(ApiError::from)
}

/// Marca como leídas las indicadas, o todas si `ids` es None.
pub fn marcar_leidas(conn: &mut PgConnection, vendedor_id: Uuid, ids: Option<Vec<Uuid>>) -> ApiResult<usize> {
    let n = match ids {
        Some(ids) => diesel::update(
            notificaciones::table
                .filter(notificaciones::vendedor_id.eq(vendedor_id))
                .filter(notificaciones::id.eq_any(ids)),
        )
        .set(notificaciones::leida.eq(true))
        .execute(conn)?,
        None => diesel::update(
            notificaciones::table
                .filter(notificaciones::vendedor_id.eq(vendedor_id))
                .filter(notificaciones::leida.eq(false)),
        )
        .set(notificaciones::leida.eq(true))
        .execute(conn)?,
    };
    Ok(n)
}

// ----------------------------- Plan / suscripción -----------------------------

#[derive(Serialize)]
pub struct Uso {
    pub cuentas: i64,
    pub productos: i64,
    pub plataformas: i64,
}

#[derive(Serialize)]
pub struct MiPlan {
    pub plan: &'static Plan,
    pub uso: Uso,
    pub planes: &'static [Plan],
}

pub fn mi_plan(conn: &mut PgConnection, vendedor_id: Uuid) -> ApiResult<MiPlan> {
    let v: Vendedor = vendedores::table.find(vendedor_id).select(Vendedor::as_select()).first(conn)?;
    let cuentas: i64 = ml_cuentas::table.filter(ml_cuentas::vendedor_id.eq(vendedor_id)).count().get_result(conn)?;
    let prods: i64 = productos::table
        .inner_join(ml_cuentas::table)
        .filter(ml_cuentas::vendedor_id.eq(vendedor_id))
        .count()
        .get_result(conn)?;
    let plataformas: i64 = api_keys::table.filter(api_keys::vendedor_id.eq(vendedor_id)).count().get_result(conn)?;
    Ok(MiPlan {
        plan: planes::plan(&v.plan),
        uso: Uso { cuentas, productos: prods, plataformas },
        planes: planes::PLANES,
    })
}

fn plan_de(conn: &mut PgConnection, vendedor_id: Uuid) -> ApiResult<&'static Plan> {
    let v: Vendedor = vendedores::table.find(vendedor_id).select(Vendedor::as_select()).first(conn)?;
    Ok(planes::plan(&v.plan))
}

/// Permite conectar una cuenta NUEVA según el plan (una reconexión siempre pasa).
pub fn permitir_cuenta_nueva(conn: &mut PgConnection, vendedor_id: Uuid, ml_user_id: i64) -> ApiResult<()> {
    let existe: i64 = ml_cuentas::table
        .filter(ml_cuentas::vendedor_id.eq(vendedor_id))
        .filter(ml_cuentas::ml_user_id.eq(ml_user_id))
        .count()
        .get_result(conn)?;
    if existe > 0 {
        return Ok(()); // reconexión de la misma cuenta por el mismo dueño
    }
    // Una cuenta de ML pertenece a UN solo usuario de MeliHub: si ya está en otro,
    // hay que desconectarla allí primero (evita que aparezca en dos lados).
    let en_otro: i64 = ml_cuentas::table
        .filter(ml_cuentas::ml_user_id.eq(ml_user_id))
        .filter(ml_cuentas::vendedor_id.ne(vendedor_id))
        .count()
        .get_result(conn)?;
    if en_otro > 0 {
        return Err(ApiError::conflict(
            "Esta cuenta de Mercado Libre ya está conectada en otro usuario de MeliHub. Desconéctala allí antes de conectarla aquí.",
        ));
    }
    let p = plan_de(conn, vendedor_id)?;
    let cuentas: i64 = ml_cuentas::table.filter(ml_cuentas::vendedor_id.eq(vendedor_id)).count().get_result(conn)?;
    if !planes::cabe(p.max_cuentas, cuentas) {
        return Err(ApiError::forbidden(format!("Tu plan {} permite {} cuenta(s) de Mercado Libre.", p.nombre, p.max_cuentas)));
    }
    Ok(())
}

/// Permite crear una API-key (plataforma) según el plan.
pub fn permitir_plataforma(conn: &mut PgConnection, vendedor_id: Uuid) -> ApiResult<()> {
    let p = plan_de(conn, vendedor_id)?;
    let n: i64 = api_keys::table.filter(api_keys::vendedor_id.eq(vendedor_id)).count().get_result(conn)?;
    if !planes::cabe(p.max_plataformas, n) {
        return Err(ApiError::forbidden(format!("Tu plan {} permite {} plataforma(s)/API keys.", p.nombre, p.max_plataformas)));
    }
    Ok(())
}
