//! Cuentas de Shopify (tiendas) por vendedor + mapeo de publicaciones para el sync.

use crate::errors::{ApiError, ApiResult};
use crate::models::{NewShopifyCuenta, NewShopifyPublicacion, ShopifyCuenta};
use crate::schema::{shopify_cuentas, shopify_publicaciones};
use diesel::prelude::*;
use uuid::Uuid;

pub fn conectar(conn: &mut PgConnection, vendedor_id: Uuid, shop_domain: &str, token: &str, nombre: Option<String>) -> ApiResult<ShopifyCuenta> {
    crate::services::saas_services::permitir_marketplace(conn, vendedor_id)?;
    let dom = crate::shopify::dominio(shop_domain);
    if dom.is_empty() || !dom.contains('.') {
        return Err(ApiError::bad_request("Dominio de Shopify inválido (ej. mitienda.myshopify.com)"));
    }
    if token.trim().is_empty() {
        return Err(ApiError::bad_request("El Admin API access token es obligatorio"));
    }
    let nuevo = NewShopifyCuenta {
        vendedor_id,
        nombre: nombre.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()),
        shop_domain: dom,
        access_token: crate::crypto::cifrar(token.trim()),
    };
    diesel::insert_into(shopify_cuentas::table)
        .values(&nuevo)
        .returning(ShopifyCuenta::as_returning())
        .get_result(conn)
        .map_err(ApiError::from)
}

pub fn listar(conn: &mut PgConnection, vendedor_id: Uuid) -> ApiResult<Vec<ShopifyCuenta>> {
    shopify_cuentas::table
        .filter(shopify_cuentas::vendedor_id.eq(vendedor_id))
        .select(ShopifyCuenta::as_select())
        .order(shopify_cuentas::created_at.desc())
        .load(conn)
        .map_err(ApiError::from)
}

pub fn cuenta_por_id(conn: &mut PgConnection, id: Uuid, vendedor_id: Uuid) -> ApiResult<ShopifyCuenta> {
    shopify_cuentas::table
        .find(id)
        .filter(shopify_cuentas::vendedor_id.eq(vendedor_id))
        .select(ShopifyCuenta::as_select())
        .first(conn)
        .map_err(|_| ApiError::not_found("Tienda Shopify no encontrada"))
}

pub fn desconectar(conn: &mut PgConnection, id: Uuid, vendedor_id: Uuid) -> ApiResult<()> {
    let n = diesel::delete(shopify_cuentas::table.find(id).filter(shopify_cuentas::vendedor_id.eq(vendedor_id))).execute(conn)?;
    if n == 0 {
        return Err(ApiError::not_found("Tienda Shopify no encontrada"));
    }
    Ok(())
}

pub fn todas_cuentas(conn: &mut PgConnection) -> ApiResult<Vec<ShopifyCuenta>> {
    shopify_cuentas::table.filter(shopify_cuentas::activa.eq(true)).select(ShopifyCuenta::as_select()).load(conn).map_err(ApiError::from)
}

/// Una publicación para el sync: variante/inventario en Shopify + stock/precio actuales del hub.
pub struct SyncItem {
    pub variant_id: Option<String>,
    pub inventory_item_id: Option<String>,
    pub precio: i64,
    pub stock: i32,
}

pub fn items_cuenta(conn: &mut PgConnection, cuenta_id: Uuid) -> ApiResult<Vec<SyncItem>> {
    use crate::schema::productos;
    let pubs: Vec<(Uuid, Option<String>, Option<String>)> = shopify_publicaciones::table
        .filter(shopify_publicaciones::cuenta_id.eq(cuenta_id))
        .select((shopify_publicaciones::producto_id, shopify_publicaciones::variant_id, shopify_publicaciones::inventory_item_id))
        .load(conn)?;
    let mut out = Vec::new();
    for (pid, vid, iid) in pubs {
        if let Ok((pj, precio, stock)) = productos::table.find(pid).select((productos::precio_jarstore, productos::precio, productos::stock)).first::<(Option<i64>, i64, i32)>(conn) {
            out.push(SyncItem { variant_id: vid, inventory_item_id: iid, precio: pj.filter(|v| *v > 0).unwrap_or(precio), stock });
        }
    }
    Ok(out)
}

/// Guarda (o actualiza) el mapeo producto del hub -> producto Shopify.
pub fn registrar_publicacion(conn: &mut PgConnection, producto_id: Uuid, cuenta_id: Uuid, shopify_product_id: &str, variant_id: Option<String>, inventory_item_id: Option<String>) -> ApiResult<()> {
    let nuevo = NewShopifyPublicacion {
        producto_id,
        cuenta_id,
        shopify_product_id: shopify_product_id.to_string(),
        variant_id: variant_id.clone(),
        inventory_item_id: inventory_item_id.clone(),
    };
    diesel::insert_into(shopify_publicaciones::table)
        .values(&nuevo)
        .on_conflict((shopify_publicaciones::producto_id, shopify_publicaciones::cuenta_id))
        .do_update()
        .set((
            shopify_publicaciones::shopify_product_id.eq(shopify_product_id.to_string()),
            shopify_publicaciones::variant_id.eq(variant_id),
            shopify_publicaciones::inventory_item_id.eq(inventory_item_id),
        ))
        .execute(conn)?;
    Ok(())
}
