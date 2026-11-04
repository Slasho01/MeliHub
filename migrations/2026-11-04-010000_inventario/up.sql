-- Enriquecer la publicación importada con datos del gestor de inventario
ALTER TABLE productos ADD COLUMN costo BIGINT;              -- costo del vendedor
ALTER TABLE productos ADD COLUMN precio_jarstore BIGINT;    -- precio para el canal JarStore (el de ML es `precio`)

-- Configuración de cálculo de ganancias por vendedor (tasas editables)
CREATE TABLE config_ganancias (
    vendedor_id UUID PRIMARY KEY REFERENCES vendedores(id) ON DELETE CASCADE,
    precios_con_iva BOOLEAN NOT NULL DEFAULT TRUE,
    iva_pct DOUBLE PRECISION NOT NULL DEFAULT 19.0,
    comision_ml_pct DOUBLE PRECISION NOT NULL DEFAULT 13.0,
    comision_ml_fija BIGINT NOT NULL DEFAULT 0,
    comision_mp_pct DOUBLE PRECISION NOT NULL DEFAULT 3.5,
    comision_mp_fija BIGINT NOT NULL DEFAULT 0,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
