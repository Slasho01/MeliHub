use crate::schema::{ml_cuentas, productos, vendedores};
use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::Serialize;
use uuid::Uuid;

// ----------------------------- Vendedor -----------------------------

#[derive(Queryable, Selectable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = vendedores, check_for_backend(diesel::pg::Pg))]
pub struct Vendedor {
    pub id: Uuid,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub nombre: String,
    pub rol: String,
    pub activo: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = vendedores)]
pub struct NewVendedor {
    pub email: String,
    pub password_hash: String,
    pub nombre: String,
    pub rol: String,
}

// ----------------------------- Cuenta ML -----------------------------

#[derive(Queryable, Selectable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = ml_cuentas, check_for_backend(diesel::pg::Pg))]
pub struct MlCuenta {
    pub id: Uuid,
    pub vendedor_id: Uuid,
    pub ml_user_id: i64,
    pub nickname: Option<String>,
    #[serde(skip_serializing)]
    pub access_token: String,
    #[serde(skip_serializing)]
    pub refresh_token: String,
    #[serde(skip_serializing)]
    pub token_expira_at: NaiveDateTime,
    pub sitio: String,
    pub activa: bool,
    pub ultima_sync_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = ml_cuentas)]
pub struct NewMlCuenta {
    pub vendedor_id: Uuid,
    pub ml_user_id: i64,
    pub nickname: Option<String>,
    pub access_token: String,
    pub refresh_token: String,
    pub token_expira_at: NaiveDateTime,
    pub sitio: String,
}

// ----------------------------- Producto -----------------------------

#[derive(Queryable, Selectable, Serialize, Debug, Clone)]
#[diesel(table_name = productos, check_for_backend(diesel::pg::Pg))]
pub struct Producto {
    pub id: Uuid,
    pub ml_cuenta_id: Uuid,
    pub ml_item_id: String,
    pub titulo: String,
    pub precio: i64,
    pub moneda: String,
    pub stock: i32,
    pub estado: String,
    pub permalink: Option<String>,
    pub thumbnail: Option<String>,
    pub categoria_ml: Option<String>,
    pub variantes: serde_json::Value,
    pub datos: serde_json::Value,
    pub sincronizado_at: NaiveDateTime,
    pub created_at: NaiveDateTime,
    /// Datos del gestor de inventario (no vienen de ML)
    pub costo: Option<i64>,
    pub precio_jarstore: Option<i64>,
}

#[derive(Insertable, AsChangeset)]
#[diesel(table_name = productos)]
pub struct UpsertProducto {
    pub ml_cuenta_id: Uuid,
    pub ml_item_id: String,
    pub titulo: String,
    pub precio: i64,
    pub moneda: String,
    pub stock: i32,
    pub estado: String,
    pub permalink: Option<String>,
    pub thumbnail: Option<String>,
    pub categoria_ml: Option<String>,
    pub variantes: serde_json::Value,
    pub datos: serde_json::Value,
    pub sincronizado_at: NaiveDateTime,
}

// ----------------------------- Config de ganancias -----------------------------

use crate::schema::config_ganancias;

#[derive(Queryable, Selectable, Insertable, AsChangeset, Serialize, Debug, Clone)]
#[diesel(table_name = config_ganancias, check_for_backend(diesel::pg::Pg))]
pub struct ConfigGanancias {
    pub vendedor_id: Uuid,
    pub precios_con_iva: bool,
    pub iva_pct: f64,
    pub comision_ml_pct: f64,
    pub comision_ml_fija: i64,
    pub comision_mp_pct: f64,
    pub comision_mp_fija: i64,
    #[serde(skip)]
    pub updated_at: NaiveDateTime,
}
