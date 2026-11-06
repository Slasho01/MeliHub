use crate::schema::{auditoria, cupones, facturacion, historial_producto, miembros, ml_cuentas, notificaciones, ordenes_ml, productos, suscripciones, vendedores};
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
    pub plan: String,
    #[serde(skip_serializing)]
    pub twofa_secret: Option<String>,
    pub twofa_enabled: bool,
    #[serde(skip_serializing)]
    pub session_version: i32,
}

#[derive(Insertable)]
#[diesel(table_name = vendedores)]
pub struct NewVendedor {
    pub email: String,
    pub password_hash: String,
    pub nombre: String,
    pub rol: String,
    pub plan: String,
}

// ----------------------------- Auditoría -----------------------------

#[derive(Queryable, Selectable, Serialize, Debug, Clone)]
#[diesel(table_name = auditoria, check_for_backend(diesel::pg::Pg))]
pub struct Auditoria {
    pub id: Uuid,
    pub vendedor_id: Uuid,
    pub actor_id: Option<Uuid>,
    pub actor: Option<String>,
    pub accion: String,
    pub detalle: Option<String>,
    pub created_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = auditoria)]
pub struct NewAuditoria {
    pub vendedor_id: Uuid,
    pub actor_id: Option<Uuid>,
    pub actor: Option<String>,
    pub accion: String,
    pub detalle: Option<String>,
}

// ----------------------------- Miembro (equipo) -----------------------------

#[derive(Queryable, Selectable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = miembros, check_for_backend(diesel::pg::Pg))]
pub struct Miembro {
    pub id: Uuid,
    pub vendedor_id: Uuid,
    pub nombre: String,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: Option<String>,
    pub rol: String,
    pub estado: String,
    #[serde(skip_serializing)]
    pub invite_token: Option<String>,
    #[serde(skip_serializing)]
    pub invite_expira: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    #[serde(skip_serializing)]
    pub twofa_secret: Option<String>,
    pub twofa_enabled: bool,
    #[serde(skip_serializing)]
    pub session_version: i32,
}

#[derive(Insertable)]
#[diesel(table_name = miembros)]
pub struct NewMiembro {
    pub vendedor_id: Uuid,
    pub nombre: String,
    pub email: String,
    pub rol: String,
    pub estado: String,
    pub invite_token: Option<String>,
    pub invite_expira: Option<NaiveDateTime>,
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
    /// Margen objetivo (%) para sugerir precio.
    pub margen_objetivo: f64,
    /// Redondeo psicológico: terminación del precio (ej. 990). 0 = sin redondeo.
    pub redondeo: i64,
}

// ----------------------------- API Key -----------------------------

use crate::schema::api_keys;

#[derive(Queryable, Selectable, Serialize, Debug, Clone)]
#[diesel(table_name = api_keys, check_for_backend(diesel::pg::Pg))]
pub struct ApiKey {
    pub id: Uuid,
    pub vendedor_id: Uuid,
    pub nombre: String,
    #[serde(skip_serializing)]
    #[allow(dead_code)]
    pub token_hash: String,
    pub canal: String,
    pub ml_cuenta_id: Option<Uuid>,
    pub activa: bool,
    pub ultima_uso_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = api_keys)]
pub struct NewApiKey {
    pub vendedor_id: Uuid,
    pub nombre: String,
    pub token_hash: String,
    pub canal: String,
    pub ml_cuenta_id: Option<Uuid>,
}

// ----------------------------- Orden de Mercado Libre -----------------------------

#[derive(Queryable, Selectable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = ordenes_ml, check_for_backend(diesel::pg::Pg))]
pub struct OrdenMl {
    pub id: Uuid,
    pub ml_cuenta_id: Uuid,
    pub ml_order_id: i64,
    pub estado: String,
    pub fecha: NaiveDateTime,
    pub moneda: String,
    pub comprador: Option<String>,
    pub venta_total: i64,
    pub comision_total: i64,
    pub costo_total: i64,
    pub ganancia: i64,
    pub items: serde_json::Value,
    #[serde(skip_serializing)]
    pub datos: serde_json::Value,
    pub created_at: NaiveDateTime,
}

#[derive(Insertable, AsChangeset, Debug)]
#[diesel(table_name = ordenes_ml)]
pub struct NewOrdenMl {
    pub ml_cuenta_id: Uuid,
    pub ml_order_id: i64,
    pub estado: String,
    pub fecha: NaiveDateTime,
    pub moneda: String,
    pub comprador: Option<String>,
    pub venta_total: i64,
    pub comision_total: i64,
    pub costo_total: i64,
    pub ganancia: i64,
    pub items: serde_json::Value,
    pub datos: serde_json::Value,
}

// ----------------------------- Historial de producto -----------------------------

#[derive(Queryable, Selectable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = historial_producto, check_for_backend(diesel::pg::Pg))]
pub struct HistorialProducto {
    pub id: Uuid,
    pub producto_id: Uuid,
    pub campo: String,
    pub anterior: Option<String>,
    pub nuevo: Option<String>,
    pub fuente: String,
    pub created_at: NaiveDateTime,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = historial_producto)]
pub struct NewHistorialProducto {
    pub producto_id: Uuid,
    pub campo: String,
    pub anterior: Option<String>,
    pub nuevo: Option<String>,
    pub fuente: String,
}

// ----------------------------- Notificación -----------------------------

#[derive(Queryable, Selectable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = notificaciones, check_for_backend(diesel::pg::Pg))]
pub struct Notificacion {
    pub id: Uuid,
    pub vendedor_id: Uuid,
    pub tipo: String,
    pub titulo: String,
    pub cuerpo: Option<String>,
    pub leida: bool,
    pub created_at: NaiveDateTime,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = notificaciones)]
pub struct NewNotificacion {
    pub vendedor_id: Uuid,
    pub tipo: String,
    pub titulo: String,
    pub cuerpo: Option<String>,
}

// ----------------------------- Suscripción -----------------------------

#[derive(Queryable, Selectable, Identifiable, AsChangeset, Serialize, Debug, Clone)]
#[diesel(table_name = suscripciones, check_for_backend(diesel::pg::Pg))]
pub struct Suscripcion {
    pub id: Uuid,
    pub vendedor_id: Uuid,
    pub plan: String,
    pub estado: String,
    pub proveedor: String,
    pub preapproval_id: Option<String>,
    pub monto: i64,
    pub vigente_hasta: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub ciclo: String,
    pub cupon: Option<String>,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = suscripciones)]
pub struct NewSuscripcion {
    pub vendedor_id: Uuid,
    pub plan: String,
    pub estado: String,
    pub proveedor: String,
    pub preapproval_id: Option<String>,
    pub monto: i64,
    pub ciclo: String,
    pub cupon: Option<String>,
}

// ----------------------------- Cupón de descuento -----------------------------

#[derive(Queryable, Selectable, Identifiable, Serialize, Debug, Clone)]
#[diesel(table_name = cupones, check_for_backend(diesel::pg::Pg))]
pub struct Cupon {
    pub id: Uuid,
    pub codigo: String,
    pub tipo: String,
    pub valor: i64,
    pub activo: bool,
    pub vence: Option<NaiveDateTime>,
    pub usos_max: Option<i32>,
    pub usos: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

// ----------------------------- Facturación -----------------------------

#[derive(Queryable, Selectable, Serialize, Debug, Clone)]
#[diesel(table_name = facturacion, primary_key(vendedor_id), check_for_backend(diesel::pg::Pg))]
pub struct Facturacion {
    pub vendedor_id: Uuid,
    pub tipo_documento: String,
    pub rut: String,
    pub razon_social: String,
    pub giro: Option<String>,
    pub direccion: String,
    pub comuna: String,
    pub ciudad: Option<String>,
    pub telefono: Option<String>,
    pub pais: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = facturacion)]
pub struct NewFacturacion {
    pub vendedor_id: Uuid,
    pub tipo_documento: String,
    pub rut: String,
    pub razon_social: String,
    pub giro: Option<String>,
    pub direccion: String,
    pub comuna: String,
    pub ciudad: Option<String>,
    pub telefono: Option<String>,
    pub pais: String,
}

#[derive(Insertable)]
#[diesel(table_name = cupones)]
pub struct NewCupon {
    pub codigo: String,
    pub tipo: String,
    pub valor: i64,
    pub vence: Option<NaiveDateTime>,
    pub usos_max: Option<i32>,
}
