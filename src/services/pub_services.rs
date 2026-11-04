use crate::errors::{ApiError, ApiResult, OrNotFound};
use crate::models::{ApiKey, Producto};
use crate::schema::productos;
use crate::services::common::now;
use diesel::prelude::*;
use serde::Serialize;
use uuid::Uuid;

/// Producto tal como lo ve una plataforma externa (precio del canal de la key).
#[derive(Serialize)]
pub struct PublicProducto {
    pub id: Uuid,
    pub ml_item_id: String,
    pub titulo: String,
    pub precio: i64,
    pub moneda: String,
    pub stock: i32,
    pub estado: String,
    pub thumbnail: Option<String>,
    pub canal: String,
}

fn precio_canal(p: &Producto, canal: &str) -> i64 {
    match canal {
        "mercadolibre" => p.precio,
        _ => p.precio_jarstore.unwrap_or(p.precio), // jarstore: precio del canal, si no hay cae al de ML
    }
}

/// Productos del vendedor dueño de la key (acotados a su cuenta si la key lo indica).
pub fn productos(conn: &mut PgConnection, key: &ApiKey) -> ApiResult<Vec<PublicProducto>> {
    use crate::schema::ml_cuentas;
    let mut q = productos::table.inner_join(ml_cuentas::table).into_boxed();
    q = q.filter(ml_cuentas::vendedor_id.eq(key.vendedor_id));
    if let Some(cid) = key.ml_cuenta_id {
        q = q.filter(productos::ml_cuenta_id.eq(cid));
    }
    let items: Vec<Producto> = q.select(Producto::as_select()).order(productos::titulo.asc()).load(conn)?;
    Ok(items
        .into_iter()
        .map(|p| PublicProducto {
            precio: precio_canal(&p, &key.canal),
            id: p.id,
            ml_item_id: p.ml_item_id,
            titulo: p.titulo,
            moneda: p.moneda,
            stock: p.stock,
            estado: p.estado,
            thumbnail: p.thumbnail,
            canal: key.canal.clone(),
        })
        .collect())
}

/// Resultado de una venta: nuevo stock y, si está vinculado, datos para empujar a ML.
pub struct VentaAplicada {
    pub nuevo_stock: i32,
    pub ml_cuenta_id: Uuid,
    pub ml_item_id: String,
}

/// Descuenta stock por una venta en una plataforma externa (transacción con bloqueo).
pub fn reportar_venta(conn: &mut PgConnection, key: &ApiKey, producto_id: Uuid, cantidad: i32) -> ApiResult<VentaAplicada> {
    if cantidad <= 0 {
        return Err(ApiError::bad_request("La cantidad debe ser mayor que 0"));
    }
    conn.transaction(|conn| {
        let p: Producto = productos::table
            .find(producto_id)
            .select(Producto::as_select())
            .for_update()
            .first(conn)
            .optional()?
            .or_not_found("Producto")?;
        // Verificar que el producto sea de este vendedor (y de la cuenta de la key si aplica).
        use crate::schema::ml_cuentas;
        let vend: Uuid = ml_cuentas::table.find(p.ml_cuenta_id).select(ml_cuentas::vendedor_id).first(conn)?;
        if vend != key.vendedor_id || key.ml_cuenta_id.is_some_and(|c| c != p.ml_cuenta_id) {
            return Err(ApiError::forbidden("Ese producto no corresponde a esta API key"));
        }
        if p.stock < cantidad {
            return Err(ApiError::bad_request(format!("Stock insuficiente: disponible {}, solicitado {cantidad}", p.stock)));
        }
        let nuevo = p.stock - cantidad;
        diesel::update(productos::table.find(producto_id))
            .set((productos::stock.eq(nuevo), productos::sincronizado_at.eq(now())))
            .execute(conn)?;
        Ok(VentaAplicada { nuevo_stock: nuevo, ml_cuenta_id: p.ml_cuenta_id, ml_item_id: p.ml_item_id })
    })
}
