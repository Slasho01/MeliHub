-- Cuentas del Marketplace de Cencosud (Paris). Integración "carga directa" vía su API
-- (REST+JSON, auth por Access Token que se obtiene con la API key).
CREATE TABLE paris_cuentas (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    vendedor_id UUID NOT NULL REFERENCES vendedores(id) ON DELETE CASCADE,
    nombre VARCHAR(255),                   -- etiqueta de la cuenta (opcional)
    api_key TEXT NOT NULL,                 -- cifrada (crypto::cifrar)
    sandbox BOOLEAN NOT NULL DEFAULT TRUE, -- staging vs producción
    activa BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    updated_at TIMESTAMP NOT NULL DEFAULT now()
);
CREATE INDEX idx_paris_cuentas_vendedor ON paris_cuentas (vendedor_id);
