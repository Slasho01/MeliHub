use crate::errors::{ApiError, ApiResult, OrNotFound};
use crate::models::{MlCuenta, NewMlCuenta};
use crate::schema::{ml_cuentas, ml_oauth_estados};
use chrono::NaiveDateTime;
use diesel::prelude::*;
use rand::Rng;
use uuid::Uuid;

/// Crea y persiste un `state` anti-CSRF para iniciar el flujo OAuth.
pub fn crear_estado(conn: &mut PgConnection, vendedor_id: Uuid) -> ApiResult<String> {
    let estado: String = {
        let mut rng = rand::thread_rng();
        (0..32).map(|_| format!("{:x}", rng.gen_range(0..16))).collect()
    };
    diesel::insert_into(ml_oauth_estados::table)
        .values((ml_oauth_estados::estado.eq(&estado), ml_oauth_estados::vendedor_id.eq(vendedor_id)))
        .execute(conn)?;
    Ok(estado)
}

/// Valida y consume un `state` (un solo uso). Devuelve el vendedor dueño.
pub fn consumir_estado(conn: &mut PgConnection, estado: &str) -> ApiResult<Uuid> {
    let vendedor_id: Option<Uuid> = ml_oauth_estados::table
        .find(estado)
        .select(ml_oauth_estados::vendedor_id)
        .first(conn)
        .optional()?;
    let vendedor_id = vendedor_id.ok_or_else(|| ApiError::bad_request("Estado OAuth inválido o expirado"))?;
    diesel::delete(ml_oauth_estados::table.find(estado)).execute(conn)?;
    Ok(vendedor_id)
}

/// Guarda (o actualiza) una cuenta ML conectada por un vendedor.
#[allow(clippy::too_many_arguments)]
pub fn upsert_cuenta(
    conn: &mut PgConnection,
    vendedor_id: Uuid,
    ml_user_id: i64,
    nickname: Option<String>,
    access_token: String,
    refresh_token: String,
    token_expira_at: NaiveDateTime,
    sitio: String,
) -> ApiResult<MlCuenta> {
    diesel::insert_into(ml_cuentas::table)
        .values(NewMlCuenta {
            vendedor_id,
            ml_user_id,
            nickname: nickname.clone(),
            access_token: access_token.clone(),
            refresh_token: refresh_token.clone(),
            token_expira_at,
            sitio: sitio.clone(),
        })
        .on_conflict((ml_cuentas::vendedor_id, ml_cuentas::ml_user_id))
        .do_update()
        .set((
            ml_cuentas::nickname.eq(nickname),
            ml_cuentas::access_token.eq(access_token),
            ml_cuentas::refresh_token.eq(refresh_token),
            ml_cuentas::token_expira_at.eq(token_expira_at),
            ml_cuentas::sitio.eq(sitio),
            ml_cuentas::activa.eq(true),
            ml_cuentas::updated_at.eq(crate::services::common::now()),
        ))
        .returning(MlCuenta::as_returning())
        .get_result(conn)
        .map_err(ApiError::from)
}

pub fn listar_cuentas(conn: &mut PgConnection, vendedor_id: Uuid, es_admin: bool) -> ApiResult<Vec<MlCuenta>> {
    let q = ml_cuentas::table.select(MlCuenta::as_select()).order(ml_cuentas::created_at.desc()).into_boxed();
    let q = if es_admin { q } else { q.filter(ml_cuentas::vendedor_id.eq(vendedor_id)) };
    Ok(q.load(conn)?)
}

/// Obtiene una cuenta verificando que pertenezca al vendedor (o admin).
pub fn get_cuenta(conn: &mut PgConnection, cuenta_id: Uuid, vendedor_id: Uuid, es_admin: bool) -> ApiResult<MlCuenta> {
    let c: MlCuenta = ml_cuentas::table
        .find(cuenta_id)
        .select(MlCuenta::as_select())
        .first(conn)
        .optional()?
        .or_not_found("Cuenta")?;
    if !es_admin && c.vendedor_id != vendedor_id {
        return Err(ApiError::forbidden("Esa cuenta no te pertenece"));
    }
    Ok(c)
}

/// Busca una cuenta por su user_id de Mercado Libre (para el webhook).
pub fn cuenta_por_ml_user(conn: &mut PgConnection, ml_user_id: i64) -> ApiResult<Option<MlCuenta>> {
    ml_cuentas::table
        .filter(ml_cuentas::ml_user_id.eq(ml_user_id).and(ml_cuentas::activa.eq(true)))
        .select(MlCuenta::as_select())
        .first(conn)
        .optional()
        .map_err(ApiError::from)
}

/// Cuentas que no se sincronizan desde hace más de `horas` (o nunca).
pub fn cuentas_desactualizadas(conn: &mut PgConnection, horas: i64) -> ApiResult<Vec<MlCuenta>> {
    let corte = crate::services::common::now() - chrono::Duration::hours(horas);
    ml_cuentas::table
        .filter(ml_cuentas::activa.eq(true))
        .filter(ml_cuentas::ultima_sync_at.is_null().or(ml_cuentas::ultima_sync_at.lt(corte)))
        .select(MlCuenta::as_select())
        .load(conn)
        .map_err(ApiError::from)
}

pub fn actualizar_tokens(
    conn: &mut PgConnection,
    cuenta_id: Uuid,
    access_token: &str,
    refresh_token: &str,
    token_expira_at: NaiveDateTime,
) -> ApiResult<()> {
    diesel::update(ml_cuentas::table.find(cuenta_id))
        .set((
            ml_cuentas::access_token.eq(access_token),
            ml_cuentas::refresh_token.eq(refresh_token),
            ml_cuentas::token_expira_at.eq(token_expira_at),
            ml_cuentas::updated_at.eq(crate::services::common::now()),
        ))
        .execute(conn)?;
    Ok(())
}

pub fn marcar_sync(conn: &mut PgConnection, cuenta_id: Uuid) -> ApiResult<()> {
    diesel::update(ml_cuentas::table.find(cuenta_id))
        .set(ml_cuentas::ultima_sync_at.eq(crate::services::common::now()))
        .execute(conn)?;
    Ok(())
}

pub fn desconectar(conn: &mut PgConnection, cuenta_id: Uuid, vendedor_id: Uuid, es_admin: bool) -> ApiResult<()> {
    let c = get_cuenta(conn, cuenta_id, vendedor_id, es_admin)?;
    diesel::delete(ml_cuentas::table.find(c.id)).execute(conn)?;
    Ok(())
}
