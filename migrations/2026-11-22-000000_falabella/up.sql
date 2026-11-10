CREATE TABLE falabella_cuentas (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    vendedor_id UUID NOT NULL REFERENCES vendedores(id) ON DELETE CASCADE,
    user_id VARCHAR(255) NOT NULL,        -- email/login del Seller Center
    api_key TEXT NOT NULL,                -- cifrada (crypto::cifrar)
    nickname VARCHAR(255),
    activa BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    updated_at TIMESTAMP NOT NULL DEFAULT now(),
    UNIQUE (vendedor_id, user_id)
);
CREATE INDEX idx_falabella_cuentas_vendedor ON falabella_cuentas (vendedor_id);
