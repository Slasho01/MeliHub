-- Comisión real de Mercado Libre por producto (traída de listing_prices durante la sync).
-- comision_pct = % de la comisión por venta; comision_fija = cargo fijo por unidad (CLP).
ALTER TABLE productos ADD COLUMN comision_pct DOUBLE PRECISION;
ALTER TABLE productos ADD COLUMN comision_fija BIGINT;
