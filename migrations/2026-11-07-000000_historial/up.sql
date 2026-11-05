-- Historial de cambios de precio/stock/estado por producto.
CREATE TABLE historial_producto (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    producto_id UUID NOT NULL REFERENCES productos(id) ON DELETE CASCADE,
    campo VARCHAR(20) NOT NULL,   -- 'precio' | 'stock' | 'estado'
    anterior TEXT,
    nuevo TEXT,
    fuente VARCHAR(20) NOT NULL,  -- 'sync' | 'venta' | 'manual'
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_historial_producto ON historial_producto (producto_id, created_at DESC);
