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
        #[max_length = 20]
        plan -> Varchar,
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
    api_keys (id) {
        id -> Uuid,
        vendedor_id -> Uuid,
        #[max_length = 100]
        nombre -> Varchar,
        #[max_length = 64]
        token_hash -> Varchar,
        #[max_length = 20]
        canal -> Varchar,
        ml_cuenta_id -> Nullable<Uuid>,
        activa -> Bool,
        ultima_uso_at -> Nullable<Timestamp>,
        created_at -> Timestamp,
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
        margen_objetivo -> Float8,
        redondeo -> Int8,
    }
}

diesel::table! {
    ordenes_ml (id) {
        id -> Uuid,
        ml_cuenta_id -> Uuid,
        ml_order_id -> Int8,
        #[max_length = 40]
        estado -> Varchar,
        fecha -> Timestamp,
        #[max_length = 8]
        moneda -> Varchar,
        #[max_length = 255]
        comprador -> Nullable<Varchar>,
        venta_total -> Int8,
        comision_total -> Int8,
        costo_total -> Int8,
        ganancia -> Int8,
        items -> Jsonb,
        datos -> Jsonb,
        created_at -> Timestamp,
    }
}

diesel::table! {
    suscripciones (id) {
        id -> Uuid,
        vendedor_id -> Uuid,
        #[max_length = 20]
        plan -> Varchar,
        #[max_length = 20]
        estado -> Varchar,
        #[max_length = 20]
        proveedor -> Varchar,
        #[max_length = 80]
        preapproval_id -> Nullable<Varchar>,
        monto -> Int8,
        vigente_hasta -> Nullable<Timestamp>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        #[max_length = 10]
        ciclo -> Varchar,
        #[max_length = 40]
        cupon -> Nullable<Varchar>,
    }
}

diesel::table! {
    facturacion (vendedor_id) {
        vendedor_id -> Uuid,
        #[max_length = 10]
        tipo_documento -> Varchar,
        #[max_length = 20]
        rut -> Varchar,
        #[max_length = 160]
        razon_social -> Varchar,
        #[max_length = 120]
        giro -> Nullable<Varchar>,
        #[max_length = 200]
        direccion -> Varchar,
        #[max_length = 80]
        comuna -> Varchar,
        #[max_length = 80]
        ciudad -> Nullable<Varchar>,
        #[max_length = 40]
        telefono -> Nullable<Varchar>,
        #[max_length = 40]
        pais -> Varchar,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    cupones (id) {
        id -> Uuid,
        #[max_length = 40]
        codigo -> Varchar,
        #[max_length = 12]
        tipo -> Varchar,
        valor -> Int8,
        activo -> Bool,
        vence -> Nullable<Timestamp>,
        usos_max -> Nullable<Int4>,
        usos -> Int4,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    notificaciones (id) {
        id -> Uuid,
        vendedor_id -> Uuid,
        #[max_length = 30]
        tipo -> Varchar,
        #[max_length = 255]
        titulo -> Varchar,
        cuerpo -> Nullable<Text>,
        leida -> Bool,
        created_at -> Timestamp,
    }
}

diesel::table! {
    historial_producto (id) {
        id -> Uuid,
        producto_id -> Uuid,
        #[max_length = 20]
        campo -> Varchar,
        anterior -> Nullable<Text>,
        nuevo -> Nullable<Text>,
        #[max_length = 20]
        fuente -> Varchar,
        created_at -> Timestamp,
    }
}

diesel::table! {
    miembros (id) {
        id -> Uuid,
        vendedor_id -> Uuid,
        #[max_length = 255]
        nombre -> Varchar,
        #[max_length = 255]
        email -> Varchar,
        #[max_length = 255]
        password_hash -> Nullable<Varchar>,
        #[max_length = 20]
        rol -> Varchar,
        #[max_length = 20]
        estado -> Varchar,
        #[max_length = 255]
        invite_token -> Nullable<Varchar>,
        invite_expira -> Nullable<Timestamp>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::joinable!(ml_cuentas -> vendedores (vendedor_id));
diesel::joinable!(ml_oauth_estados -> vendedores (vendedor_id));
diesel::joinable!(productos -> ml_cuentas (ml_cuenta_id));
diesel::joinable!(config_ganancias -> vendedores (vendedor_id));
diesel::joinable!(api_keys -> vendedores (vendedor_id));
diesel::joinable!(ordenes_ml -> ml_cuentas (ml_cuenta_id));
diesel::joinable!(historial_producto -> productos (producto_id));
diesel::joinable!(notificaciones -> vendedores (vendedor_id));
diesel::joinable!(suscripciones -> vendedores (vendedor_id));
diesel::joinable!(miembros -> vendedores (vendedor_id));

diesel::allow_tables_to_appear_in_same_query!(
    api_keys,
    config_ganancias,
    cupones,
    facturacion,
    historial_producto,
    miembros,
    ml_cuentas,
    ml_oauth_estados,
    notificaciones,
    ordenes_ml,
    productos,
    suscripciones,
    vendedores,
);
