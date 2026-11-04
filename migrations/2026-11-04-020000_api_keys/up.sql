-- API keys para que plataformas externas (JarStore, WordPress de terceros)
-- consuman los productos del vendedor y reporten ventas.
CREATE TABLE api_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    vendedor_id UUID NOT NULL REFERENCES vendedores(id) ON DELETE CASCADE,
    nombre VARCHAR(100) NOT NULL,
    token_hash VARCHAR(64) UNIQUE NOT NULL,     -- sha256 hex de la key (la key en claro se muestra una vez)
    canal VARCHAR(20) NOT NULL DEFAULT 'jarstore',  -- qué precio sirve: jarstore | mercadolibre
    ml_cuenta_id UUID REFERENCES ml_cuentas(id) ON DELETE CASCADE,  -- NULL = todas las cuentas del vendedor
    activa BOOLEAN NOT NULL DEFAULT TRUE,
    ultima_uso_at TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_api_keys_vendedor ON api_keys (vendedor_id);
