-- Alcance del cupón por ciclo de cobro (ambos | mensual | anual) y uso único por usuario.
ALTER TABLE cupones ADD COLUMN ciclo VARCHAR(12) NOT NULL DEFAULT 'ambos';

CREATE TABLE cupon_usos (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    cupon_id UUID NOT NULL REFERENCES cupones(id) ON DELETE CASCADE,
    vendedor_id UUID NOT NULL REFERENCES vendedores(id) ON DELETE CASCADE,
    usado_at TIMESTAMP NOT NULL DEFAULT now(),
    UNIQUE (cupon_id, vendedor_id)
);
CREATE INDEX idx_cupon_usos_cupon ON cupon_usos (cupon_id);
