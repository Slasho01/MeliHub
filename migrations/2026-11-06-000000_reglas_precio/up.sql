-- Reglas de precio: margen objetivo (%) y redondeo psicológico (ej. terminar en 990).
ALTER TABLE config_ganancias ADD COLUMN margen_objetivo DOUBLE PRECISION NOT NULL DEFAULT 30.0;
ALTER TABLE config_ganancias ADD COLUMN redondeo BIGINT NOT NULL DEFAULT 0;
