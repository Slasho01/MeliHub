-- Vendedores: usuarios del hub que conectan sus cuentas de Mercado Libre
CREATE TABLE vendedores (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    nombre VARCHAR(255) NOT NULL,
    rol VARCHAR(20) NOT NULL DEFAULT 'vendedor',
    activo BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Cuentas de Mercado Libre conectadas (una o varias por vendedor)
CREATE TABLE ml_cuentas (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    vendedor_id UUID NOT NULL REFERENCES vendedores(id) ON DELETE CASCADE,
    ml_user_id BIGINT NOT NULL,
    nickname VARCHAR(255),
    access_token TEXT NOT NULL,
    refresh_token TEXT NOT NULL,
    token_expira_at TIMESTAMP NOT NULL,
    sitio VARCHAR(5) NOT NULL DEFAULT 'MLC',
    activa BOOLEAN NOT NULL DEFAULT TRUE,
    ultima_sync_at TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (vendedor_id, ml_user_id)
);

-- Estado anti-CSRF del flujo OAuth
CREATE TABLE ml_oauth_estados (
    estado VARCHAR(64) PRIMARY KEY,
    vendedor_id UUID NOT NULL REFERENCES vendedores(id) ON DELETE CASCADE,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Publicaciones sincronizadas desde Mercado Libre
CREATE TABLE productos (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    ml_cuenta_id UUID NOT NULL REFERENCES ml_cuentas(id) ON DELETE CASCADE,
    ml_item_id VARCHAR(30) NOT NULL,
    titulo VARCHAR(500) NOT NULL,
    precio BIGINT NOT NULL DEFAULT 0,
    moneda VARCHAR(5) NOT NULL DEFAULT 'CLP',
    stock INT NOT NULL DEFAULT 0,
    estado VARCHAR(20) NOT NULL DEFAULT 'active',
    permalink VARCHAR(500),
    thumbnail VARCHAR(500),
    categoria_ml VARCHAR(30),
    variantes JSONB NOT NULL DEFAULT '[]'::jsonb,
    datos JSONB NOT NULL DEFAULT '{}'::jsonb,
    sincronizado_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (ml_cuenta_id, ml_item_id)
);
CREATE INDEX idx_productos_cuenta ON productos (ml_cuenta_id);
