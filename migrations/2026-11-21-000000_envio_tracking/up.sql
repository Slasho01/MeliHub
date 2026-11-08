ALTER TABLE ordenes_ml ADD COLUMN IF NOT EXISTS envio_tracking VARCHAR(80);
ALTER TABLE ordenes_ml ADD COLUMN IF NOT EXISTS envio_tracking_metodo VARCHAR(80);
