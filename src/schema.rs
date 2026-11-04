// Esquema de Diesel. Refleja /migrations.
diesel::table! {
    vendedores (id) {
        id -> Uuid,
        #[max_length = 255]
        email -> Varchar,
        #[max_length = 255]
        password_hash -> Varchar,
        #[max_length = 255]
        nombre -> Varchar,
        #[max_length = 20]
        rol -> Varchar,
        activo -> Bool,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    ml_cuentas (id) {
        id -> Uuid,
        vendedor_id -> Uuid,
        ml_user_id -> Int8,
        #[max_length = 255]
        nickname -> Nullable<Varchar>,
        access_token -> Text,
        refresh_token -> Text,
        token_expira_at -> Timestamp,
        #[max_length = 5]
        sitio -> Varchar,
        activa -> Bool,
        ultima_sync_at -> Nullable<Timestamp>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    ml_oauth_estados (estado) {
        #[max_length = 64]
        estado -> Varchar,
        vendedor_id -> Uuid,
        created_at -> Timestamp,
    }
}

diesel::table! {
    productos (id) {
        id -> Uuid,
        ml_cuenta_id -> Uuid,
        #[max_length = 30]
        ml_item_id -> Varchar,
        #[max_length = 500]
        titulo -> Varchar,
        precio -> Int8,
        #[max_length = 5]
        moneda -> Varchar,
        stock -> Int4,
        #[max_length = 20]
        estado -> Varchar,
        #[max_length = 500]
        permalink -> Nullable<Varchar>,
        #[max_length = 500]
        thumbnail -> Nullable<Varchar>,
        #[max_length = 30]
        categoria_ml -> Nullable<Varchar>,
        variantes -> Jsonb,
        datos -> Jsonb,
        sincronizado_at -> Timestamp,
        created_at -> Timestamp,
        costo -> Nullable<Int8>,
        precio_jarstore -> Nullable<Int8>,
    }
}

diesel::table! {
    config_ganancias (vendedor_id) {
        vendedor_id -> Uuid,
        precios_con_iva -> Bool,
        iva_pct -> Float8,
        comision_ml_pct -> Float8,
        comision_ml_fija -> Int8,
        comision_mp_pct -> Float8,
        comision_mp_fija -> Int8,
        updated_at -> Timestamp,
    }
}

diesel::joinable!(ml_cuentas -> vendedores (vendedor_id));
diesel::joinable!(ml_oauth_estados -> vendedores (vendedor_id));
diesel::joinable!(productos -> ml_cuentas (ml_cuenta_id));
diesel::joinable!(config_ganancias -> vendedores (vendedor_id));

diesel::allow_tables_to_appear_in_same_query!(
    config_ganancias,
    ml_cuentas,
    ml_oauth_estados,
    productos,
    vendedores,
);
