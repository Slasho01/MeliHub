-- Suscripciones de pago (Mercado Pago - preapproval).
CREATE TABLE suscripciones (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    vendedor_id UUID NOT NULL REFERENCES vendedores(id) ON DELETE CASCADE,
    plan VARCHAR(20) NOT NULL,
    estado VARCHAR(20) NOT NULL DEFAULT 'pending',   -- pending | authorized | paused | cancelled
    proveedor VARCHAR(20) NOT NULL DEFAULT 'mercadopago',
    preapproval_id VARCHAR(80),                       -- id de la suscripción en MP
    monto BIGINT NOT NULL DEFAULT 0,
    vigente_hasta TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_suscripciones_vendedor ON suscripciones (vendedor_id, created_at DESC);
CREATE UNIQUE INDEX uq_suscripcion_preapproval ON suscripciones (preapproval_id) WHERE preapproval_id IS NOT NULL;
