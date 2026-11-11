//! Conexión de cuentas de Falabella Seller Center. Al conectar se validan las credenciales
//! con una llamada de solo lectura (GetBrands) ANTES de guardarlas (cifradas).

use crate::auth::AuthVendedor;
use crate::config::Config;
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

#[derive(Deserialize)]
pub struct CuentaQuery {
    pub cuenta: Uuid,
}

/// Publicaciones locales (producto_id, sku, título) de una cuenta — la UI las cruza con GetProducts.
pub async fn publicaciones(user: AuthVendedor, q: web::Query<CuentaQuery>, pool: web::Data<DbPool>) -> ApiResult<HttpResponse> {
    let (vid, cid) = (user.id, q.cuenta);
    let rows = db::run(&pool, move |conn| {
        svc::get(conn, cid, vid)?; // valida pertenencia de la cuenta
        svc::listar_publicaciones(conn, cid)
    })
    .await?;
    let out: Vec<_> = rows.into_iter().map(|(pid, sku, titulo)| serde_json::json!({ "producto_id": pid, "seller_sku": sku, "titulo": titulo })).collect();
    Ok(HttpResponse::Ok().json(out))
}

/// Productos ya publicados en Falabella (para ver su estado/stock/imágenes).
pub async fn productos_fala(user: AuthVendedor, q: web::Query<CuentaQuery>, pool: web::Data<DbPool>, client: web::Data<FalabellaClient>) -> ApiResult<HttpResponse> {
    let (vid, cid) = (user.id, q.cuenta);
    let cuenta = db::run(&pool, move |conn| svc::get(conn, cid, vid)).await?;
    let api_key = crate::crypto::descifrar(&cuenta.api_key);
    Ok(HttpResponse::Ok().json(client.get_products(&cuenta.user_id, &api_key, None).await?))
}

#[derive(Deserialize)]
pub struct OrdenesQuery {
    pub cuenta: Uuid,
    #[serde(default)]
    pub dias: Option<i64>,
}

/// Ids de los ítems de una orden (para marcar listo / pedir documento).
async fn item_ids_de_orden(client: &FalabellaClient, user_id: &str, api_key: &str, order_id: &str) -> Vec<i64> {
    let v = match client.get_order_items(user_id, api_key, order_id).await {
        Ok(v) => v,
        Err(_) => return vec![],
    };
    let items = match v.pointer("/SuccessResponse/Body/OrderItems/OrderItem") {
        Some(serde_json::Value::Array(a)) => a.clone(),
        Some(o) if o.is_object() => vec![o.clone()],
        _ => vec![],
    };
    items
        .iter()
        .filter_map(|it| {
            it.get("OrderItemId")
                .and_then(|x| x.as_i64().or_else(|| x.as_str().and_then(|s| s.parse().ok())))
        })
        .collect()
}

#[derive(Deserialize)]
pub struct ListoReq {
    pub cuenta_id: Uuid,
    pub order_id: String,
    #[serde(default)]
    pub delivery_type: Option<String>,
    #[serde(default)]
    pub shipping_provider: Option<String>,
    #[serde(default)]
    pub tracking_number: Option<String>,
}

/// Marca una orden como "listo para enviar" en Falabella.
pub async fn orden_listo(user: AuthVendedor, body: web::Json<ListoReq>, pool: web::Data<DbPool>, client: web::Data<FalabellaClient>) -> ApiResult<HttpResponse> {
    user.exigir_edicion()?;
    let ListoReq { cuenta_id, order_id, delivery_type, shipping_provider, tracking_number } = body.into_inner();
    let vid = user.id;
    let cuenta = db::run(&pool, move |conn| svc::get(conn, cuenta_id, vid)).await?;
    let api_key = crate::crypto::descifrar(&cuenta.api_key);
    let ids = item_ids_de_orden(&client, &cuenta.user_id, &api_key, &order_id).await;
    if ids.is_empty() {
        return Err(ApiError::bad_request("No se encontraron ítems de la orden."));
    }
    let dt = delivery_type.as_deref().unwrap_or("dropship");
    let sp = shipping_provider.as_deref().unwrap_or("");
    let tn = tracking_number.as_deref().unwrap_or("");
    Ok(HttpResponse::Ok().json(client.set_ready_to_ship(&cuenta.user_id, &api_key, &ids, dt, sp, tn).await?))
}

#[derive(Deserialize)]
pub struct DocQuery {
    pub cuenta: Uuid,
    pub order_id: String,
    #[serde(default)]
    pub tipo: Option<String>,
}

/// Descarga un documento de la orden (etiqueta de envío o factura) como PDF.
pub async fn orden_documento(user: AuthVendedor, q: web::Query<DocQuery>, pool: web::Data<DbPool>, client: web::Data<FalabellaClient>) -> ApiResult<HttpResponse> {
    use base64::Engine as _;
    let (vid, cid) = (user.id, q.cuenta);
    let cuenta = db::run(&pool, move |conn| svc::get(conn, cid, vid)).await?;
    let api_key = crate::crypto::descifrar(&cuenta.api_key);
    let ids = item_ids_de_orden(&client, &cuenta.user_id, &api_key, &q.order_id).await;
    if ids.is_empty() {
        return Err(ApiError::bad_request("No se encontraron ítems de la orden."));
    }
    let tipo = q.tipo.as_deref().unwrap_or("shippingLabel");
    let v = client.get_document(&cuenta.user_id, &api_key, &ids, tipo).await?;
    let b64 = v
        .pointer("/SuccessResponse/Body/Documents/Document/File")
        .and_then(|x| x.as_str())
        .ok_or_else(|| ApiError::bad_request("Falabella no entregó el documento (marca la orden como 'listo para enviar' primero)."))?;
    let bytes = base64::prelude::BASE64_STANDARD
        .decode(b64)
        .map_err(|_| ApiError::internal("Documento inválido"))?;
    Ok(HttpResponse::Ok().content_type("application/pdf").body(bytes))
}

/// Órdenes (ventas) de Falabella de los últimos `dias` (por defecto 30).
pub async fn ordenes(user: AuthVendedor, q: web::Query<OrdenesQuery>, pool: web::Data<DbPool>, client: web::Data<FalabellaClient>) -> ApiResult<HttpResponse> {
    let (vid, cid, dias) = (user.id, q.cuenta, q.dias.unwrap_or(30).clamp(1, 365));
    let cuenta = db::run(&pool, move |conn| svc::get(conn, cid, vid)).await?;
    let api_key = crate::crypto::descifrar(&cuenta.api_key);
    let created_after = (chrono::Utc::now() - chrono::Duration::days(dias)).format("%Y-%m-%dT%H:%M:%S%:z").to_string();
    Ok(HttpResponse::Ok().json(client.get_orders(&cuenta.user_id, &api_key, &created_after).await?))
}

/// Marcas válidas de Falabella (para elegir una que exista; texto libre falla).
pub async fn marcas(user: AuthVendedor, q: web::Query<CuentaQuery>, pool: web::Data<DbPool>, client: web::Data<FalabellaClient>) -> ApiResult<HttpResponse> {
    let (vid, cid) = (user.id, q.cuenta);
    let cuenta = db::run(&pool, move |conn| svc::get(conn, cid, vid)).await?;
    let api_key = crate::crypto::descifrar(&cuenta.api_key);
    Ok(HttpResponse::Ok().json(client.get_brands(&cuenta.user_id, &api_key).await?))
}

/// Árbol de categorías de Falabella (para mapear la categoría del producto).
pub async fn categorias(user: AuthVendedor, q: web::Query<CuentaQuery>, pool: web::Data<DbPool>, client: web::Data<FalabellaClient>) -> ApiResult<HttpResponse> {
    let (vid, cid) = (user.id, q.cuenta);
    let cuenta = db::run(&pool, move |conn| svc::get(conn, cid, vid)).await?;
    let api_key = crate::crypto::descifrar(&cuenta.api_key);
    Ok(HttpResponse::Ok().json(client.get_category_tree(&cuenta.user_id, &api_key).await?))
}

#[derive(Deserialize)]
pub struct AtributosQuery {
    pub cuenta: Uuid,
    pub categoria: String,
}

/// Atributos (obligatorios/opcionales) de una categoría Falabella.
pub async fn atributos(user: AuthVendedor, q: web::Query<AtributosQuery>, pool: web::Data<DbPool>, client: web::Data<FalabellaClient>) -> ApiResult<HttpResponse> {
    let (vid, cid, cat) = (user.id, q.cuenta, q.categoria.clone());
    let cuenta = db::run(&pool, move |conn| svc::get(conn, cid, vid)).await?;
    let api_key = crate::crypto::descifrar(&cuenta.api_key);
    Ok(HttpResponse::Ok().json(client.get_category_attributes(&cuenta.user_id, &api_key, &cat).await?))
}

#[derive(Deserialize)]
pub struct AtributoKV {
    pub nombre: String,
    pub valor: String,
}

#[derive(Deserialize)]
pub struct PublicarReq {
    pub producto_id: Uuid,
    pub cuenta_id: Uuid,
    pub categoria: String,
    pub marca: String,
    #[serde(default)]
    pub variation: Option<String>,
    #[serde(default)]
    pub operador: Option<String>,
    #[serde(default)]
    pub atributos: Vec<AtributoKV>,
}

#[derive(Deserialize)]
pub struct ImagenesReq {
    pub producto_id: Uuid,
    pub cuenta_id: Uuid,
}

/// (Re)sube las imágenes de un producto YA publicado en Falabella (acción `Image`).
pub async fn subir_imagenes(user: AuthVendedor, body: web::Json<ImagenesReq>, pool: web::Data<DbPool>, client: web::Data<FalabellaClient>) -> ApiResult<HttpResponse> {
    user.exigir_edicion()?;
    let ImagenesReq { producto_id, cuenta_id } = body.into_inner();
    let (vid, admin) = (user.id, user.es_admin());
    let (producto, cuenta) = db::run(&pool, move |conn| {
        let (p, _) = crate::services::producto_services::get_con_cuenta(conn, producto_id, vid, admin)?;
        let c = svc::get(conn, cuenta_id, vid)?;
        Ok::<_, ApiError>((p, c))
    })
    .await?;
    let api_key = crate::crypto::descifrar(&cuenta.api_key);
    let xml = crate::services::falabella_push_services::build_image_xml(&producto)
        .ok_or_else(|| ApiError::bad_request("Este producto no tiene imágenes para subir."))?;
    Ok(HttpResponse::Ok().json(client.product_image(&cuenta.user_id, &api_key, xml).await?))
}

#[derive(Deserialize)]
pub struct EstadoReq {
    pub producto_id: Uuid,
    pub cuenta_id: Uuid,
    pub activo: bool,
}

/// Pausa o reactiva un producto en Falabella (ProductUpdate con Status active/inactive).
pub async fn cambiar_estado(user: AuthVendedor, body: web::Json<EstadoReq>, pool: web::Data<DbPool>, client: web::Data<FalabellaClient>) -> ApiResult<HttpResponse> {
    user.exigir_edicion()?;
    let EstadoReq { producto_id, cuenta_id, activo } = body.into_inner();
    let (vid, admin) = (user.id, user.es_admin());
    let (producto, cuenta, operador) = db::run(&pool, move |conn| {
        let (p, _) = crate::services::producto_services::get_con_cuenta(conn, producto_id, vid, admin)?;
        let c = svc::get(conn, cuenta_id, vid)?;
        let op = svc::operador_de(conn, producto_id, cuenta_id);
        Ok::<_, ApiError>((p, c, op))
    })
    .await?;
    let api_key = crate::crypto::descifrar(&cuenta.api_key);
    let xml = crate::services::falabella_push_services::build_estado_xml(&producto.ml_item_id, activo, &operador);
    Ok(HttpResponse::Ok().json(client.product_update(&cuenta.user_id, &api_key, xml).await?))
}

/// Webhook de Falabella (PÚBLICO, sin auth): al recibir un evento, procesa en 2º plano las
/// ventas recientes de esa cuenta (descuenta stock + empuja a ML). La cuenta va en la URL.
pub async fn webhook(path: web::Path<Uuid>, pool: web::Data<DbPool>, client: web::Data<FalabellaClient>, ml: web::Data<crate::ml::MlClient>, cfg: web::Data<Config>) -> ApiResult<HttpResponse> {
    let cuenta_id = path.into_inner();
    let (pool2, fala2, ml2) = (pool.clone(), client.clone(), ml.clone());
    let ml_ok = cfg.ml.configurado();
    actix_web::rt::spawn(async move {
        if let Ok(Some(cuenta)) = db::run(&pool2, move |conn| svc::cuenta_por_id(conn, cuenta_id)).await {
            crate::jobs::procesar_ventas_cuenta(&pool2, &ml2, &fala2, &cuenta, ml_ok).await;
        }
    });
    Ok(HttpResponse::Ok().finish())
}

#[derive(Deserialize)]
pub struct RegistrarWebhookReq {
    pub cuenta_id: Uuid,
}

/// Registra el webhook de órdenes de Falabella (callback único por cuenta) para recibir las
/// ventas en tiempo real. Requiere `APP_URL` pública en el servidor.
pub async fn registrar_webhook(user: AuthVendedor, body: web::Json<RegistrarWebhookReq>, pool: web::Data<DbPool>, client: web::Data<FalabellaClient>, cfg: web::Data<Config>) -> ApiResult<HttpResponse> {
    user.exigir_owner()?;
    let cuenta_id = body.into_inner().cuenta_id;
    let vid = user.id;
    let cuenta = db::run(&pool, move |conn| svc::get(conn, cuenta_id, vid)).await?;
    let api_key = crate::crypto::descifrar(&cuenta.api_key);
    let callback = format!("{}/api/falabella/webhook/{}", cfg.app_url, cuenta.id);
    Ok(HttpResponse::Ok().json(client.create_webhook(&cuenta.user_id, &api_key, &callback, &["onOrderCreated"]).await?))
}

/// Sube imágenes en 2º plano (solo productos simples) tras crear/recrear el producto.
fn spawn_imagenes(client: &web::Data<FalabellaClient>, cuenta: &crate::models::FalabellaCuenta, producto: &crate::models::Producto) {
    if producto.variantes.as_array().is_some_and(|a| !a.is_empty()) {
        return;
    }
    let Some(img_xml) = crate::services::falabella_push_services::build_image_xml(producto) else { return };
    let client2 = client.get_ref().clone();
    let (uid, key, sku) = (cuenta.user_id.clone(), crate::crypto::descifrar(&cuenta.api_key), producto.ml_item_id.clone());
    actix_web::rt::spawn(async move {
        for _ in 0..15 {
            actix_web::rt::time::sleep(std::time::Duration::from_secs(8)).await;
            let existe = client2
                .get_products(&uid, &key, Some(&sku))
                .await
                .ok()
                .and_then(|v| v.get("SuccessResponse")?.get("Body")?.get("Products").map(|p| !p.is_string()))
                .unwrap_or(false);
            if existe {
                let _ = client2.product_image(&uid, &key, img_xml.clone()).await;
                return;
            }
        }
    });
}

#[derive(Deserialize)]
pub struct RepublicarReq {
    pub producto_id: Uuid,
    pub cuenta_id: Uuid,
}

/// Republica un producto reutilizando su mapeo guardado (categoría/marca/atributos). Sirve
/// para re-enviarlo tras un cambio o un rechazo, sin re-elegir nada (base de la masiva).
pub async fn republicar(user: AuthVendedor, body: web::Json<RepublicarReq>, pool: web::Data<DbPool>, client: web::Data<FalabellaClient>) -> ApiResult<HttpResponse> {
    user.exigir_edicion()?;
    let RepublicarReq { producto_id, cuenta_id } = body.into_inner();
    let (vid, admin) = (user.id, user.es_admin());
    let (producto, cuenta, mapeo) = db::run(&pool, move |conn| {
        let (p, _) = crate::services::producto_services::get_con_cuenta(conn, producto_id, vid, admin)?;
        let c = svc::get(conn, cuenta_id, vid)?;
        let m = svc::mapeo_de(conn, producto_id, cuenta_id)?;
        Ok::<_, ApiError>((p, c, m))
    })
    .await?;
    let Some((cat, mar, op, attrs_json)) = mapeo else {
        return Err(ApiError::bad_request("Este producto no tiene un mapeo guardado; publícalo primero desde el formulario."));
    };
    let categoria = cat.unwrap_or_default();
    if categoria.is_empty() {
        return Err(ApiError::bad_request("Falta la categoría guardada; vuelve a publicarlo desde el formulario."));
    }
    let marca = mar.unwrap_or_default();
    let mut atributos: Vec<(String, String)> = attrs_json
        .as_array()
        .map(|a| a.iter().filter_map(|x| Some((x.get("nombre")?.as_str()?.to_string(), x.get("valor")?.as_str()?.to_string()))).collect())
        .unwrap_or_default();
    for (feed, val) in crate::services::falabella_push_services::dimensiones_ml(&producto) {
        if !atributos.iter().any(|(k, _)| k == &feed) {
            atributos.push((feed, val));
        }
    }
    let api_key = crate::crypto::descifrar(&cuenta.api_key);
    let xml = crate::services::falabella_push_services::build_variantes_xml(&producto, &categoria, &marca, &op, &atributos)
        .unwrap_or_else(|| crate::services::falabella_push_services::build_product_xml(&producto, &categoria, &marca, "", &op, &atributos));
    let creado = client.product_create(&cuenta.user_id, &api_key, xml).await?;
    spawn_imagenes(&client, &cuenta, &producto);
    Ok(HttpResponse::Ok().json(serde_json::json!({ "producto": creado, "imagenes": "en_proceso" })))
}

/// Feeds recientes (ProductCreate/Update son asíncronos): estado y nº de errores.
pub async fn feeds(user: AuthVendedor, q: web::Query<CuentaQuery>, pool: web::Data<DbPool>, client: web::Data<FalabellaClient>) -> ApiResult<HttpResponse> {
    let (vid, cid) = (user.id, q.cuenta);
    let cuenta = db::run(&pool, move |conn| svc::get(conn, cid, vid)).await?;
    let api_key = crate::crypto::descifrar(&cuenta.api_key);
    Ok(HttpResponse::Ok().json(client.feed_list(&cuenta.user_id, &api_key).await?))
}

#[derive(Deserialize)]
pub struct FeedQuery {
    pub cuenta: Uuid,
    pub id: String,
}

/// Detalle de un feed: errores por producto (por qué no se publicó).
pub async fn feed_estado(user: AuthVendedor, q: web::Query<FeedQuery>, pool: web::Data<DbPool>, client: web::Data<FalabellaClient>) -> ApiResult<HttpResponse> {
    let (vid, cid, fid) = (user.id, q.cuenta, q.id.clone());
    let cuenta = db::run(&pool, move |conn| svc::get(conn, cid, vid)).await?;
    let api_key = crate::crypto::descifrar(&cuenta.api_key);
    Ok(HttpResponse::Ok().json(client.feed_status(&cuenta.user_id, &api_key, &fid).await?))
}

/// Publica (o actualiza) un producto del hub en Falabella con su categoría/atributos.
pub async fn publicar(user: AuthVendedor, body: web::Json<PublicarReq>, pool: web::Data<DbPool>, client: web::Data<FalabellaClient>) -> ApiResult<HttpResponse> {
    user.exigir_edicion()?;
    let PublicarReq { producto_id, cuenta_id, categoria, marca, variation, operador, atributos } = body.into_inner();
    let (vid, admin) = (user.id, user.es_admin());
    let (producto, cuenta) = db::run(&pool, move |conn| {
        let (p, _) = crate::services::producto_services::get_con_cuenta(conn, producto_id, vid, admin)?;
        let c = svc::get(conn, cuenta_id, vid)?;
        Ok::<_, ApiError>((p, c))
    })
    .await?;
    let api_key = crate::crypto::descifrar(&cuenta.api_key);
    let mut atributos: Vec<(String, String)> = atributos
        .into_iter()
        .map(|a| (a.nombre, a.valor.trim().to_string()))
        .filter(|(_, v)| !v.is_empty())
        .collect();
    // Autollenar dimensiones/peso del paquete desde ML si el vendedor no las proporcionó.
    for (feed, val) in crate::services::falabella_push_services::dimensiones_ml(&producto) {
        if !atributos.iter().any(|(k, _)| k == &feed) {
            atributos.push((feed, val));
        }
    }
    let es_variante = producto.variantes.as_array().is_some_and(|a| !a.is_empty());
    let xml = crate::services::falabella_push_services::build_variantes_xml(&producto, &categoria, &marca, operador.as_deref().unwrap_or(""), &atributos)
        .unwrap_or_else(|| {
            crate::services::falabella_push_services::build_product_xml(
                &producto,
                &categoria,
                &marca,
                variation.as_deref().unwrap_or(""),
                operador.as_deref().unwrap_or(""),
                &atributos,
            )
        });
    let creado = client.product_create(&cuenta.user_id, &api_key, xml).await?;
    // Registrar la publicación para la sincronización continua de precio/stock.
    let op = operador.clone().unwrap_or_default();
    let op = if op.trim().is_empty() { "facl".to_string() } else { op };
    let attrs_json = serde_json::Value::Array(atributos.iter().map(|(n, v)| serde_json::json!({ "nombre": n, "valor": v })).collect());
    let (pid, caid, sku, cat, mar) = (producto.id, cuenta.id, producto.ml_item_id.clone(), categoria.clone(), marca.clone());
    let _ = db::run(&pool, move |conn| svc::registrar_publicacion(conn, pid, caid, &sku, &op, &cat, &mar, attrs_json)).await;
    // Imágenes: acción `Image` aparte, PERO el producto debe estar ya registrado (ProductCreate
    // es asíncrono y puede tardar >1 min). Lo subimos EN SEGUNDO PLANO con reintentos, para no
    // colgar la respuesta: cuando el SKU aparece en GetProducts, mandamos las imágenes.
    // Imágenes en 2º plano SOLO para productos simples (en variantes, cada variación tiene su
    // SellerSku y las imágenes van inline en el ProductCreate).
    if let Some(img_xml) = (!es_variante).then(|| crate::services::falabella_push_services::build_image_xml(&producto)).flatten() {
        let client2 = client.get_ref().clone();
        let (uid, key, sku) = (cuenta.user_id.clone(), api_key.clone(), producto.ml_item_id.clone());
        actix_web::rt::spawn(async move {
            for intento in 0..15 {
                actix_web::rt::time::sleep(std::time::Duration::from_secs(8)).await;
                let existe = client2
                    .get_products(&uid, &key, Some(&sku))
                    .await
                    .ok()
                    .and_then(|v| v.get("SuccessResponse")?.get("Body")?.get("Products").map(|p| !p.is_string()))
                    .unwrap_or(false);
                if existe {
                    match client2.product_image(&uid, &key, img_xml.clone()).await {
                        Ok(_) => log::info!("Falabella: imágenes de {sku} enviadas (intento {intento})"),
                        Err(e) => log::warn!("Falabella: imágenes de {sku} fallaron: {e}"),
                    }
                    return;
                }
            }
            log::warn!("Falabella: {sku} no se registró a tiempo; las imágenes quedan para 'Subir imágenes'.");
        });
    }
    Ok(HttpResponse::Ok().json(serde_json::json!({ "producto": creado, "imagenes": "en_proceso" })))
}
