-- Repricing automático de Buy Box: mantener ganando la publicación de catálogo respetando
-- un margen mínimo. Config por vendedor.
ALTER TABLE config_ganancias ADD COLUMN repricing_activo BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE config_ganancias ADD COLUMN repricing_margen_min FLOAT8 NOT NULL DEFAULT 15;
