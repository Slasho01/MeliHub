-- 2FA (TOTP) para la cuenta dueño.
ALTER TABLE vendedores ADD COLUMN twofa_secret VARCHAR(160);
ALTER TABLE vendedores ADD COLUMN twofa_enabled BOOLEAN NOT NULL DEFAULT FALSE;

-- Registro de auditoría: acciones relevantes por organización.
CREATE TABLE auditoria (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    vendedor_id UUID NOT NULL REFERENCES vendedores(id) ON DELETE CASCADE,
    actor_id UUID,
    actor VARCHAR(255),
    accion VARCHAR(60) NOT NULL,
    detalle VARCHAR(400),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_auditoria_vendedor ON auditoria (vendedor_id, created_at DESC);
