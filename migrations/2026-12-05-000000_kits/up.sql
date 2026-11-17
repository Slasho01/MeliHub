-- Kits / combos: un producto (publicación) puede estar compuesto por varios productos
-- componentes. Al venderse el kit se descuenta el stock de cada componente, y la
-- disponibilidad del kit = min(floor(stock_componente / cantidad)).
CREATE TABLE kit_componentes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    vendedor_id UUID NOT NULL REFERENCES vendedores(id) ON DELETE CASCADE,
    kit_producto_id UUID NOT NULL REFERENCES productos(id) ON DELETE CASCADE,
    componente_producto_id UUID NOT NULL REFERENCES productos(id) ON DELETE CASCADE,
    cantidad INTEGER NOT NULL DEFAULT 1 CHECK (cantidad > 0),
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    UNIQUE (kit_producto_id, componente_producto_id),
    CHECK (kit_producto_id <> componente_producto_id)
);
CREATE INDEX idx_kit_comp_vendedor ON kit_componentes (vendedor_id);
CREATE INDEX idx_kit_comp_kit ON kit_componentes (kit_producto_id);
CREATE INDEX idx_kit_comp_componente ON kit_componentes (componente_producto_id);
