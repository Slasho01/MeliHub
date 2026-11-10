//! Conexión de cuentas de Falabella Seller Center. Al conectar se validan las credenciales
//! con una llamada de solo lectura (GetBrands) ANTES de guardarlas (cifradas).

use crate::auth::AuthVendedor;
use crate::db::{self, DbPool};
use crate::errors::{ApiError, ApiResult};
use crate::falabella::FalabellaClient;
use crate::services::falabella_services as svc;
use actix_web::{web, HttpResponse};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct ConectarReq {
    pub user_id: String,
    pub api_key: String,
    #[serde(default)]
    pub nickname: Option<String>,
}

pub async fn conectar(
    user: AuthVendedor,
    body: web::Json<ConectarReq>,
    pool: web::Data<DbPool>,
    client: web::Data<FalabellaClient>,
) -> ApiResult<HttpResponse> {
    user.exigir_owner()?;
    let ConectarReq { user_id, api_key, nickname } = body.into_inner();
    let user_id = user_id.trim().to_string();
    let api_key = api_key.trim().to_string();
    if user_id.is_empty() || api_key.is_empty() {
        return Err(ApiError::bad_request("Ingresa el UserID (email) y la API key de Falabella Seller Center."));
    }
    // Valida credenciales contra Falabella (lectura) antes de guardar nada.
    client.get_brands(&user_id, &api_key).await?;
    let vid = user.id;
    let nick = nickname.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    let cuenta = db::run(&pool, move |conn| svc::conectar(conn, vid, &user_id, &api_key, nick)).await?;
    Ok(HttpResponse::Created().json(cuenta))
}

pub async fn listar(user: AuthVendedor, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    let vid = user.id;
    Ok(HttpResponse::Ok().json(db::run(&pool, move |conn| svc::listar(conn, vid)).await?))
}

pub async fn desconectar(path: web::Path<Uuid>, user: AuthVendedor, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    user.exigir_owner()?;
    let (cid, vid) = (path.into_inner(), user.id);
    db::run(&pool, move |conn| svc::desconectar(conn, cid, vid)).await?;
    Ok(HttpResponse::NoContent().finish())
}
