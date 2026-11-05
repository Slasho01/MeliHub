-- Órdenes de Mercado Libre importadas (ventas centralizadas, ganancia real).
CREATE TABLE ordenes_ml (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    ml_cuenta_id UUID NOT NULL REFERENCES ml_cuentas(id) ON DELETE CASCADE,
    ml_order_id BIGINT NOT NULL,
    estado VARCHAR(40) NOT NULL,
    fecha TIMESTAMP NOT NULL,
    moneda VARCHAR(8) NOT NULL DEFAULT 'CLP',
    comprador VARCHAR(255),
    venta_total BIGINT NOT NULL DEFAULT 0,    -- suma unit_price * qty
    comision_total BIGINT NOT NULL DEFAULT 0, -- comisión real de ML (sale_fee)
    costo_total BIGINT NOT NULL DEFAULT 0,    -- suma costo * qty (lo conocido)
    ganancia BIGINT NOT NULL DEFAULT 0,       -- venta - comisión - costo
    items JSONB NOT NULL DEFAULT '[]'::jsonb,
    datos JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (ml_cuenta_id, ml_order_id)
);
CREATE INDEX idx_ordenes_ml_cuenta_fecha ON ordenes_ml (ml_cuenta_id, fecha DESC);
