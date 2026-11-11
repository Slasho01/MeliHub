-- Mapeo de productos publicados en Paris (para empujar stock por sku_seller y precio por
-- el sku interno de Paris), y registro de órdenes ya procesadas (dedup de ventas).
CREATE TABLE paris_publicaciones (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    producto_id UUID NOT NULL REFERENCES productos(id) ON DELETE CASCADE,
    cuenta_id UUID NOT NULL REFERENCES paris_cuentas(id) ON DELETE CASCADE,
    sku_seller VARCHAR(80) NOT NULL,       -- nuestro SKU (refProduct)
    paris_sku VARCHAR(120),                -- SKU que asigna Paris (si lo devuelve)
    ultima_sync_at TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    UNIQUE (cuenta_id, sku_seller)
);
CREATE INDEX idx_paris_pub_cuenta ON paris_publicaciones (cuenta_id);

CREATE TABLE paris_ventas (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    cuenta_id UUID NOT NULL REFERENCES paris_cuentas(id) ON DELETE CASCADE,
    order_number VARCHAR(80) NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    UNIQUE (cuenta_id, order_number)
);
