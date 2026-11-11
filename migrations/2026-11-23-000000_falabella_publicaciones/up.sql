CREATE TABLE falabella_publicaciones (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    producto_id UUID NOT NULL REFERENCES productos(id) ON DELETE CASCADE,
    cuenta_id UUID NOT NULL REFERENCES falabella_cuentas(id) ON DELETE CASCADE,
    seller_sku VARCHAR(60) NOT NULL,
    operador VARCHAR(10) NOT NULL DEFAULT 'facl',
    ultima_sync_at TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    UNIQUE (producto_id, cuenta_id)
);
CREATE INDEX idx_fala_pub_cuenta ON falabella_publicaciones (cuenta_id);
