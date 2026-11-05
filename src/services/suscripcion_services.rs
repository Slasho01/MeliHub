//! Suscripciones de pago: registra el preapproval de Mercado Pago y ajusta el
//! `plan` del vendedor según el estado que llega por webhook.

use crate::errors::{ApiError, ApiResult};
use crate::models::{NewSuscripcion, Suscripcion};
use crate::schema::{suscripciones, vendedores};
use crate::services::common::now;
use chrono::{Duration, NaiveDateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

/// Email del vendedor (para el pagador de MP).
pub fn email_de(conn: &mut PgConnection, vendedor_id: Uuid) -> ApiResult<String> {
    vendedores::table
        .find(vendedor_id)
        .select(vendedores::email)
        .first(conn)
        .map_err(ApiError::from)
}

/// Registra una suscripción (pendiente) ligada a un preapproval de MP.
#[allow(clippy::too_many_arguments)]
pub fn registrar(conn: &mut PgConnection, vendedor_id: Uuid, plan: &str, monto: i64, preapproval_id: &str, ciclo: &str, cupon: Option<&str>) -> ApiResult<Suscripcion> {
    diesel::insert_into(suscripciones::table)
        .values(NewSuscripcion {
            vendedor_id,
            plan: plan.to_string(),
            estado: "pending".to_string(),
            proveedor: "mercadopago".to_string(),
            preapproval_id: Some(preapproval_id.to_string()),
            monto,
            ciclo: ciclo.to_string(),
            cupon: cupon.map(|c| c.to_string()),
        })
        .returning(Suscripcion::as_returning())
        .get_result(conn)
        .map_err(ApiError::from)
}

/// Suscripción más reciente del vendedor.
pub fn actual(conn: &mut PgConnection, vendedor_id: Uuid) -> ApiResult<Option<Suscripcion>> {
    suscripciones::table
        .filter(suscripciones::vendedor_id.eq(vendedor_id))
        .order(suscripciones::created_at.desc())
        .limit(1)
        .select(Suscripcion::as_select())
        .first(conn)
        .optional()
        .map_err(ApiError::from)
}

/// Aplica el estado de MP a la suscripción y ajusta el plan del vendedor.
///   authorized -> activa el plan contratado · cancelled -> vuelve a free
///   paused     -> queda pausada (conserva el plan hasta vencer)
pub fn aplicar_estado(conn: &mut PgConnection, preapproval_id: &str, estado_mp: &str) -> ApiResult<()> {
    let sub: Option<Suscripcion> = suscripciones::table
        .filter(suscripciones::preapproval_id.eq(preapproval_id))
        .select(Suscripcion::as_select())
        .first(conn)
        .optional()?;
    let Some(sub) = sub else { return Ok(()) }; // preapproval desconocido: ignorar

    let (estado, plan_vendedor, vigente): (String, String, Option<NaiveDateTime>) = match estado_mp {
        "authorized" => {
            let dias = if sub.ciclo == "anual" { 366 } else { 31 };
            ("authorized".into(), sub.plan.clone(), Some(Utc::now().naive_utc() + Duration::days(dias)))
        }
        "cancelled" => ("cancelled".into(), "free".into(), None),
        "paused" => ("paused".into(), sub.plan.clone(), sub.vigente_hasta),
        _ => (sub.estado.clone(), sub.plan.clone(), sub.vigente_hasta), // pending u otros: sin cambios de plan
    };

    diesel::update(suscripciones::table.find(sub.id))
        .set((
            suscripciones::estado.eq(&estado),
            suscripciones::vigente_hasta.eq(vigente),
            suscripciones::updated_at.eq(now()),
        ))
        .execute(conn)?;

    if estado == "authorized" || estado == "cancelled" {
        diesel::update(vendedores::table.find(sub.vendedor_id))
            .set(vendedores::plan.eq(&plan_vendedor))
            .execute(conn)?;
    }
    Ok(())
}

/// Marca la suscripción como cancelada y baja el plan a free. Devuelve el
/// preapproval_id (para cancelarlo también en MP).
pub fn cancelar_local(conn: &mut PgConnection, vendedor_id: Uuid) -> ApiResult<Option<String>> {
    let sub = actual(conn, vendedor_id)?;
    let Some(sub) = sub else { return Ok(None) };
    diesel::update(suscripciones::table.find(sub.id))
        .set((suscripciones::estado.eq("cancelled"), suscripciones::vigente_hasta.eq::<Option<NaiveDateTime>>(None), suscripciones::updated_at.eq(now())))
        .execute(conn)?;
    diesel::update(vendedores::table.find(vendedor_id)).set(vendedores::plan.eq("free")).execute(conn)?;
    Ok(sub.preapproval_id)
}

/// Email del vendedor y plan de una suscripción (para el correo de confirmación).
pub fn email_y_plan(conn: &mut PgConnection, preapproval_id: &str) -> ApiResult<Option<(String, String)>> {
    let sub: Option<Suscripcion> = suscripciones::table
        .filter(suscripciones::preapproval_id.eq(preapproval_id))
        .select(Suscripcion::as_select())
        .first(conn)
        .optional()?;
    let Some(sub) = sub else { return Ok(None) };
    let email: String = vendedores::table.find(sub.vendedor_id).select(vendedores::email).first(conn)?;
    Ok(Some((email, sub.plan)))
}
