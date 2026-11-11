-- Órdenes de Falabella ya procesadas (para no descontar stock dos veces).
CREATE TABLE falabella_ventas (
    order_id VARCHAR(40) PRIMARY KEY,
    cuenta_id UUID NOT NULL REFERENCES falabella_cuentas(id) ON DELETE CASCADE,
    procesada_at TIMESTAMP NOT NULL DEFAULT now()
);
-- Precio propio para Falabella (como precio_jarstore). NULL = usa el precio de ML.
ALTER TABLE productos ADD COLUMN precio_falabella BIGINT;
