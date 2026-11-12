-- Cuentas de facturación electrónica (DTE / SII) por vendedor. Multi-tenant: cada seller
-- conecta su proveedor (LibreDTE) con su token y RUT emisor para emitir boletas/facturas.
CREATE TABLE dte_cuentas (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    vendedor_id UUID NOT NULL REFERENCES vendedores(id) ON DELETE CASCADE,
    proveedor VARCHAR(30) NOT NULL DEFAULT 'libredte',
    api_token TEXT NOT NULL,                       -- cifrado (crypto::cifrar)
    rut_emisor VARCHAR(20) NOT NULL,               -- RUT del emisor (ej. 76192083-9)
    razon_social VARCHAR(255),
    certificacion BOOLEAN NOT NULL DEFAULT TRUE,   -- ambiente SII certificación (pruebas) vs producción
    activa BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    updated_at TIMESTAMP NOT NULL DEFAULT now()
);
CREATE INDEX idx_dte_cuentas_vendedor ON dte_cuentas (vendedor_id);
