-- Dedupe de órdenes de Shopify ya procesadas (para no descontar stock dos veces).
CREATE TABLE shopify_ventas (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    cuenta_id UUID NOT NULL REFERENCES shopify_cuentas(id) ON DELETE CASCADE,
    order_id VARCHAR(40) NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    UNIQUE (cuenta_id, order_id)
);
