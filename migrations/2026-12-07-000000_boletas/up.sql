-- Boletas/facturas de la suscripción, subidas manualmente por el admin.
-- El archivo se guarda como data-URL base64 en TEXT (igual que el logo del cotizador).
CREATE TABLE boletas (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    vendedor_id UUID NOT NULL REFERENCES vendedores(id) ON DELETE CASCADE,
    periodo VARCHAR(20) NOT NULL,           -- ej "2026-11" (mes) o "2026" (año)
    nombre_archivo VARCHAR(255) NOT NULL,
    archivo TEXT NOT NULL,                  -- data:application/pdf;base64,... (o imagen)
    subida_at TIMESTAMP NOT NULL DEFAULT now()
);
CREATE INDEX idx_boletas_vendedor ON boletas (vendedor_id);
