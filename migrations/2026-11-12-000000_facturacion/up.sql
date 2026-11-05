-- Datos de facturación por organización (vendedor). 1:1, upsert.
CREATE TABLE facturacion (
    vendedor_id UUID PRIMARY KEY REFERENCES vendedores(id) ON DELETE CASCADE,
    tipo_documento VARCHAR(10) NOT NULL DEFAULT 'boleta', -- boleta | factura
    rut VARCHAR(20) NOT NULL,
    razon_social VARCHAR(160) NOT NULL,
    giro VARCHAR(120),
    direccion VARCHAR(200) NOT NULL,
    comuna VARCHAR(80) NOT NULL,
    ciudad VARCHAR(80),
    telefono VARCHAR(40),
    pais VARCHAR(40) NOT NULL DEFAULT 'Chile',
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
