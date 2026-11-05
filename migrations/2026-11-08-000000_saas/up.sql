-- Suscripción por niveles + notificaciones in-app.
ALTER TABLE vendedores ADD COLUMN plan VARCHAR(20) NOT NULL DEFAULT 'business';

CREATE TABLE notificaciones (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    vendedor_id UUID NOT NULL REFERENCES vendedores(id) ON DELETE CASCADE,
    tipo VARCHAR(30) NOT NULL,     -- 'quiebre' | 'sync_error' | 'info'
    titulo VARCHAR(255) NOT NULL,
    cuerpo TEXT,
    leida BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_notificaciones_vendedor ON notificaciones (vendedor_id, created_at DESC);
-- Evita duplicar la misma notificación no leída (p. ej. quiebre del mismo producto).
CREATE UNIQUE INDEX uq_notif_pendiente ON notificaciones (vendedor_id, tipo, titulo) WHERE NOT leida;
