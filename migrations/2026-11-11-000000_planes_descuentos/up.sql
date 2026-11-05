-- Ciclo de cobro (mensual/anual) y cupón usado, por suscripción.
ALTER TABLE suscripciones ADD COLUMN ciclo VARCHAR(10) NOT NULL DEFAULT 'mensual';
ALTER TABLE suscripciones ADD COLUMN cupon VARCHAR(40);

-- Cupones de descuento (globales, administrados por el súper-admin).
CREATE TABLE cupones (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    codigo VARCHAR(40) NOT NULL,
    tipo VARCHAR(12) NOT NULL,            -- porcentaje | monto
    valor BIGINT NOT NULL,               -- % (1..100) o CLP fijo
    activo BOOLEAN NOT NULL DEFAULT TRUE,
    vence TIMESTAMP,                      -- null = sin vencimiento
    usos_max INTEGER,                    -- null = ilimitado
    usos INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE UNIQUE INDEX uq_cupon_codigo ON cupones (upper(codigo));
