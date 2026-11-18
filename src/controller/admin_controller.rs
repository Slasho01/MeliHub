use crate::auth::AuthVendedor;
use crate::db::{self, DbPool};
use crate::errors::{ApiError, ApiResult};
use crate::services::admin_services as adm;
use actix_web::{web, HttpResponse};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

fn exigir_admin(user: &AuthVendedor) -> ApiResult<()> {
    if !user.es_admin() {
        return Err(ApiError::forbidden("Solo el administrador de la plataforma puede acceder aquí"));
    }
    Ok(())
}

/// Escapa HTML para incrustar texto del admin sin romper la plantilla.
fn escapar(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

#[derive(Deserialize)]
pub struct PromoBody {
    pub asunto: String,
    pub titulo: String,
    pub cuerpo: String,
    #[serde(default)]
    pub cta_texto: Option<String>,
    #[serde(default)]
    pub cta_url: Option<String>,
    /// Si viene, envía SOLO a esa dirección (prueba); si no, según `destino`.
    #[serde(default)]
    pub prueba_a: Option<String>,
    /// "usuarios" (cuentas activas, por defecto) | "lista" (correos de `emails`).
    #[serde(default)]
    pub destino: Option<String>,
    /// Lista de correos para `destino = "lista"` (posibles clientes).
    #[serde(default)]
    pub emails: Option<Vec<String>>,
}

/// Envía un correo de promoción con la plantilla de la marca. Modo prueba (a una
/// sola dirección) o difusión a todas las cuentas activas.
pub async fn enviar_promo(user: AuthVendedor, body: web::Json<PromoBody>, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    exigir_admin(&user)?;
    let PromoBody { asunto, titulo, cuerpo, cta_texto, cta_url, prueba_a, destino, emails } = body.into_inner();
    if asunto.trim().is_empty() || titulo.trim().is_empty() || cuerpo.trim().is_empty() {
        return Err(ApiError::bad_request("Completa asunto, título y cuerpo."));
    }
    // Cuerpo en párrafos (doble salto) con saltos simples como <br>, todo escapado.
    let cuerpo_html: String = cuerpo
        .split("\n\n")
        .filter(|p| !p.trim().is_empty())
        .map(|p| format!("<p style=\"margin:0 0 14px\">{}</p>", escapar(p.trim()).replace('\n', "<br>")))
        .collect();
    // Botón opcional (solo si hay texto y una URL http válida).
    let boton_html = match (
        cta_texto.as_deref().map(str::trim).filter(|s| !s.is_empty()),
        cta_url.as_deref().map(str::trim).filter(|s| s.starts_with("http")),
    ) {
        (Some(t), Some(u)) => crate::email::boton(&escapar(t), &u.replace('"', "%22")),
        _ => String::new(),
    };
    let contenido = format!(
        "<h2 style=\"margin:0 0 14px;color:#1f2733;font-size:20px\">{}</h2>{}{}",
        escapar(titulo.trim()),
        cuerpo_html,
        boton_html
    );
    let html = crate::email::plantilla(&contenido);

    // Modo prueba: una sola dirección.
    if let Some(dir) = prueba_a.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        crate::email::enviar_sistema(&pool, dir, asunto.trim(), &html).await?;
        return Ok(HttpResponse::Ok().json(json!({ "prueba": true, "total": 1, "enviados": 1, "fallidos": 0 })));
    }

    // Difusión: usuarios existentes (cuentas activas) o una lista de posibles clientes.
    let destinatarios: Vec<String> = if destino.as_deref() == Some("lista") {
        let mut v: Vec<String> = emails
            .unwrap_or_default()
            .into_iter()
            .map(|e| e.trim().to_lowercase())
            .filter(|e| e.contains('@') && e.len() >= 5)
            .collect();
        v.sort();
        v.dedup();
        if v.is_empty() {
            return Err(ApiError::bad_request("Agrega al menos un correo válido en la lista."));
        }
        if v.len() > 2000 {
            return Err(ApiError::bad_request("Máximo 2000 correos por envío."));
        }
        v
    } else {
        db::run(&pool, adm::emails_promo).await?.into_iter().map(|(_n, e)| e).collect()
    };
    let total = destinatarios.len();
    let mut enviados = 0usize;
    let mut fallidos = 0usize;
    for email in destinatarios {
        match crate::email::enviar_sistema(&pool, &email, asunto.trim(), &html).await {
            Ok(_) => enviados += 1,
            Err(_) => fallidos += 1,
        }
    }
    Ok(HttpResponse::Ok().json(json!({ "prueba": false, "total": total, "enviados": enviados, "fallidos": fallidos })))
}

/// Lista de vendedores (clientes) con su uso.
pub async fn vendedores(user: AuthVendedor, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    exigir_admin(&user)?;
    Ok(HttpResponse::Ok().json(db::run(&pool, adm::listar_vendedores).await?))
}

/// Métricas globales de la plataforma.
pub async fn metricas(user: AuthVendedor, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    exigir_admin(&user)?;
    Ok(HttpResponse::Ok().json(db::run(&pool, adm::metricas).await?))
}

#[derive(Deserialize)]
pub struct PlanBody {
    pub plan: String,
}

/// Cambia el plan de un vendedor.
pub async fn set_plan(user: AuthVendedor, path: web::Path<Uuid>, body: web::Json<PlanBody>, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    exigir_admin(&user)?;
    let (id, plan) = (path.into_inner(), body.into_inner().plan);
    Ok(HttpResponse::Ok().json(db::run(&pool, move |conn| adm::set_plan(conn, id, &plan)).await?))
}

#[derive(Deserialize)]
pub struct EstadoBody {
    pub activo: bool,
}

/// Suspende o reactiva a un vendedor.
pub async fn set_estado(user: AuthVendedor, path: web::Path<Uuid>, body: web::Json<EstadoBody>, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    exigir_admin(&user)?;
    let (id, activo) = (path.into_inner(), body.into_inner().activo);
    if id == user.id && !activo {
        return Err(ApiError::bad_request("No puedes suspender tu propia cuenta"));
    }
    Ok(HttpResponse::Ok().json(db::run(&pool, move |conn| adm::set_activo(conn, id, activo)).await?))
}

// ----------------------------- Suscripciones + boletas -----------------------------

/// Lista todas las suscripciones con sus boletas (solo admin).
pub async fn suscripciones(user: AuthVendedor, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    exigir_admin(&user)?;
    Ok(HttpResponse::Ok().json(db::run(&pool, adm::listar_suscripciones).await?))
}

#[derive(Deserialize)]
pub struct BoletaBody {
    pub periodo: String,
    pub nombre_archivo: String,
    pub archivo: String, // data:...;base64,...
}

/// Sube una boleta/factura para un vendedor (solo admin).
pub async fn subir_boleta(user: AuthVendedor, path: web::Path<Uuid>, body: web::Json<BoletaBody>, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    exigir_admin(&user)?;
    let vid = path.into_inner();
    let BoletaBody { periodo, nombre_archivo, archivo } = body.into_inner();
    if archivo.len() > 7_500_000 {
        return Err(ApiError::bad_request("El archivo es muy grande (máx. ~5 MB)."));
    }
    let id = db::run(&pool, move |conn| adm::guardar_boleta(conn, vid, &periodo, &nombre_archivo, &archivo)).await?;
    Ok(HttpResponse::Ok().json(json!({ "id": id })))
}

/// Descarga una boleta: devuelve nombre + data-URL para que el front la baje.
pub async fn descargar_boleta(user: AuthVendedor, path: web::Path<Uuid>, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    exigir_admin(&user)?;
    let id = path.into_inner();
    let (nombre_archivo, archivo) = db::run(&pool, move |conn| adm::boleta_archivo(conn, id)).await?;
    Ok(HttpResponse::Ok().json(json!({ "nombre_archivo": nombre_archivo, "archivo": archivo })))
}

/// Borra una boleta (solo admin).
pub async fn eliminar_boleta(user: AuthVendedor, path: web::Path<Uuid>, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    exigir_admin(&user)?;
    let id = path.into_inner();
    db::run(&pool, move |conn| adm::eliminar_boleta(conn, id)).await?;
    Ok(HttpResponse::Ok().json(json!({ "ok": true })))
}
