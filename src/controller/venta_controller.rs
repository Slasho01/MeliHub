use crate::auth::AuthVendedor;
use crate::config::Config;
use crate::db::{self, DbPool};
use crate::errors::{ApiError, ApiResult};
use crate::ml::MlClient;
use crate::services::{ml_services, venta_services as venta};
use actix_web::{web, HttpResponse};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

/// Trae las órdenes recientes de todas las cuentas ML del vendedor.
pub async fn sincronizar(user: AuthVendedor, pool: web::Data<DbPool>, client: web::Data<MlClient>, cfg: web::Data<Config>) -> ApiResult<HttpResponse> {
    user.exigir_edicion()?;
    if !cfg.ml.configurado() {
        return Err(ApiError::bad_request("Mercado Libre no está configurado."));
    }
    let (vid, admin) = (user.id, user.es_admin());
    let cuentas = db::run(&pool, move |conn| ml_services::listar_cuentas(conn, vid, admin)).await?;
    let (mut total, mut errores) = (0usize, 0usize);
    for cuenta in &cuentas {
        match crate::sync::sincronizar_ordenes(&pool, &client, cuenta).await {
            Ok(n) => total += n,
            Err(e) => {
                errores += 1;
                log::warn!("No se pudieron sincronizar órdenes de la cuenta {}: {e}", cuenta.id);
            }
        }
    }
    Ok(HttpResponse::Ok().json(json!({ "sincronizadas": total, "cuentas": cuentas.len(), "errores": errores })))
}

#[derive(Deserialize)]
pub struct DashQuery {
    pub dias: Option<i64>,
}

/// Dashboard de ventas con ganancia real (últimos N días, 30 por defecto).
pub async fn dashboard(user: AuthVendedor, q: web::Query<DashQuery>, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    let (vid, dias) = (user.id, q.into_inner().dias.unwrap_or(30));
    Ok(HttpResponse::Ok().json(db::run(&pool, move |conn| venta::dashboard(conn, vid, dias)).await?))
}

fn csv_campo(s: &str) -> String {
    if s.contains([',', '"', '\n']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// Exporta las órdenes del período a CSV (descarga).
pub async fn exportar(user: AuthVendedor, q: web::Query<DashQuery>, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    let (vid, dias) = (user.id, q.into_inner().dias.unwrap_or(30));
    let ordenes = db::run(&pool, move |conn| venta::ordenes_periodo(conn, vid, dias)).await?;
    let mut csv = String::from("fecha,orden_ml,estado,comprador,venta_total,comision,costo,ganancia\n");
    for o in &ordenes {
        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{}\n",
            o.fecha.format("%Y-%m-%d %H:%M"),
            o.ml_order_id,
            csv_campo(&o.estado),
            csv_campo(o.comprador.as_deref().unwrap_or("")),
            o.venta_total,
            o.comision_total,
            o.costo_total,
            o.ganancia,
        ));
    }
    Ok(HttpResponse::Ok()
        .insert_header(("Content-Type", "text/csv; charset=utf-8"))
        .insert_header(("Content-Disposition", "attachment; filename=\"ventas-melihub.csv\""))
        .body(csv))
}

#[derive(Deserialize)]
pub struct ListQuery {
    pub cuenta_id: Option<Uuid>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Listado de órdenes del vendedor (más recientes primero).
pub async fn listar(user: AuthVendedor, q: web::Query<ListQuery>, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    let vid = user.id;
    let ListQuery { cuenta_id, limit, offset } = q.into_inner();
    let items = db::run(&pool, move |conn| venta::listar(conn, vid, cuenta_id, limit.unwrap_or(50), offset.unwrap_or(0))).await?;
    Ok(HttpResponse::Ok().json(items))
}

// ----------------------------- Envíos / etiquetas -----------------------------

#[derive(Deserialize)]
pub struct DiaQuery {
    pub dia: Option<String>,
    pub cuenta: Option<Uuid>,
}

/// Rango de fechas: si `dia` es una fecha válida → ese día; si no → últimos `default_dias`.
fn rango(dia: Option<&str>, default_dias: i64) -> (chrono::NaiveDateTime, chrono::NaiveDateTime) {
    match dia.and_then(|s| chrono::NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d").ok()) {
        Some(d) => {
            let ini = d.and_hms_opt(0, 0, 0).unwrap();
            (ini, ini + chrono::Duration::days(1))
        }
        None => {
            let ahora = chrono::Utc::now().naive_utc();
            (ahora - chrono::Duration::days(default_dias), ahora + chrono::Duration::days(1))
        }
    }
}

fn slug_archivo(s: &str) -> String {
    let out: String = s.chars().map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' }).collect();
    out.trim_matches('-').chars().take(40).collect()
}

fn pdf_resp(filename: &str, bytes: Vec<u8>) -> HttpResponse {
    HttpResponse::Ok()
        .insert_header(("Content-Type", "application/pdf"))
        .insert_header(("Content-Disposition", format!("attachment; filename=\"{filename}\"")))
        .body(bytes)
}

fn zip_pdfs(files: &[(String, Vec<u8>)]) -> ApiResult<Vec<u8>> {
    use std::io::Write;
    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        let mut zw = zip::ZipWriter::new(&mut cursor);
        let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for (name, bytes) in files {
            zw.start_file(name.clone(), opts).map_err(|e| ApiError::internal(format!("zip: {e}")))?;
            zw.write_all(bytes).map_err(|e| ApiError::internal(format!("zip: {e}")))?;
        }
        zw.finish().map_err(|e| ApiError::internal(format!("zip: {e}")))?;
    }
    Ok(cursor.into_inner())
}

#[derive(Deserialize)]
pub struct EnviosQuery {
    pub dia: Option<String>,
    pub incluir_entregados: Option<u8>,
    pub cuenta: Option<Uuid>,
}

/// Consulta a ML el tipo logístico y estado de los envíos que lo necesiten (clasifica
/// FULL y refresca los pendientes) y lo guarda. Best-effort, con tope y caché de token.
async fn enriquecer(pool: &web::Data<DbPool>, client: &web::Data<MlClient>, cfg: &web::Data<Config>, vid: Uuid, ini: chrono::NaiveDateTime, fin: chrono::NaiveDateTime, cuenta: Option<Uuid>) -> ApiResult<()> {
    if !cfg.ml.configurado() {
        return Ok(());
    }
    let pendientes = db::run(pool, move |conn| venta::para_enriquecer(conn, vid, ini, fin, cuenta)).await?;
    let mut tokens: std::collections::HashMap<Uuid, Option<String>> = std::collections::HashMap::new();
    let mut updates: Vec<(Uuid, String, String, String, String, String)> = Vec::new();
    for (orden_id, cuenta, envio) in pendientes.into_iter().take(80) {
        let token = match tokens.get(&cuenta.id) {
            Some(t) => t.clone(),
            None => {
                let t = crate::sync::access_token_valido(pool, client, &cuenta).await.ok();
                tokens.insert(cuenta.id, t.clone());
                t
            }
        };
        let Some(token) = token else { continue };
        if let Some(sh) = client.get_shipment(&token, envio).await {
            let lt = sh.get("logistic_type").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let st = sh.get("status").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let ss = sh.get("substatus").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let tn = sh.get("tracking_number").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let tm = sh.get("tracking_method").and_then(|v| v.as_str()).unwrap_or("").to_string();
            if !lt.is_empty() {
                updates.push((orden_id, lt, st, ss, tn, tm));
            }
        }
    }
    if !updates.is_empty() {
        db::run(pool, move |conn| {
            for (id, lt, st, ss, tn, tm) in &updates {
                let _ = venta::set_envio_info(conn, *id, lt, st, ss, tn, tm);
            }
            Ok::<(), ApiError>(())
        })
        .await?;
    }
    Ok(())
}

/// Lista las ventas con envío pendiente (sin FULL ni ya enviados) y su estado.
pub async fn envios(user: AuthVendedor, q: web::Query<EnviosQuery>, pool: web::Data<DbPool>, client: web::Data<MlClient>, cfg: web::Data<Config>) -> ApiResult<HttpResponse> {
    let vid = user.id;
    let EnviosQuery { dia, incluir_entregados, cuenta } = q.into_inner();
    let (ini, fin) = rango(dia.as_deref(), 15);
    enriquecer(&pool, &client, &cfg, vid, ini, fin, cuenta).await?;
    let incluir_cerrados = incluir_entregados == Some(1);
    let rows = db::run(&pool, move |conn| venta::envios(conn, vid, ini, fin, incluir_cerrados, cuenta)).await?;
    Ok(HttpResponse::Ok().json(rows))
}

/// Etiqueta (PDF) de una venta puntual.
pub async fn etiqueta(user: AuthVendedor, path: web::Path<Uuid>, pool: web::Data<DbPool>, client: web::Data<MlClient>, cfg: web::Data<Config>) -> ApiResult<HttpResponse> {
    if !cfg.ml.configurado() {
        return Err(ApiError::bad_request("Mercado Libre no está configurado."));
    }
    let (vid, oid) = (user.id, path.into_inner());
    let (cuenta, envio) = db::run(&pool, move |conn| venta::orden_para_etiqueta(conn, vid, oid)).await?;
    let token = crate::sync::access_token_valido(&pool, &client, &cuenta).await?;
    let pdf = client.shipment_labels(&token, &[envio]).await?;
    Ok(pdf_resp(&format!("etiqueta-{envio}.pdf"), pdf))
}

/// Descarga todas las etiquetas del día: 1 cuenta → PDF; varias → ZIP (un PDF por cuenta).
pub async fn etiquetas(user: AuthVendedor, q: web::Query<DiaQuery>, pool: web::Data<DbPool>, client: web::Data<MlClient>, cfg: web::Data<Config>) -> ApiResult<HttpResponse> {
    if !cfg.ml.configurado() {
        return Err(ApiError::bad_request("Mercado Libre no está configurado."));
    }
    let vid = user.id;
    let DiaQuery { dia, cuenta } = q.into_inner();
    let (ini, fin) = rango(dia.as_deref(), 15);
    enriquecer(&pool, &client, &cfg, vid, ini, fin, cuenta).await?;
    let grupos = db::run(&pool, move |conn| venta::envios_por_cuenta(conn, vid, ini, fin, cuenta)).await?;
    if grupos.is_empty() {
        return Err(ApiError::bad_request("No hay ventas con envío imprimible en ese día."));
    }
    let mut pdfs: Vec<(String, Vec<u8>)> = Vec::new();
    for (cuenta, ids) in grupos {
        match crate::sync::access_token_valido(&pool, &client, &cuenta).await {
            Ok(token) => match client.shipment_labels(&token, &ids).await {
                Ok(pdf) => {
                    let nombre = cuenta.nickname.clone().unwrap_or_else(|| format!("cuenta-{}", cuenta.ml_user_id));
                    pdfs.push((format!("etiquetas-{}.pdf", slug_archivo(&nombre)), pdf));
                }
                Err(e) => log::warn!("etiquetas cuenta {}: {e}", cuenta.id),
            },
            Err(e) => log::warn!("token cuenta {}: {e}", cuenta.id),
        }
    }
    if pdfs.is_empty() {
        return Err(ApiError::bad_request("No se pudo generar ninguna etiqueta (¿envíos FULL, o falta el permiso de envíos en tu app de ML?)."));
    }
    let fecha = dia.unwrap_or_else(|| "recientes".to_string());
    if pdfs.len() == 1 {
        let (_, pdf) = pdfs.into_iter().next().unwrap();
        return Ok(pdf_resp(&format!("etiquetas-{fecha}.pdf"), pdf));
    }
    let zip = zip_pdfs(&pdfs)?;
    Ok(HttpResponse::Ok()
        .insert_header(("Content-Type", "application/zip"))
        .insert_header(("Content-Disposition", format!("attachment; filename=\"etiquetas-{fecha}.zip\"")))
        .body(zip))
}
