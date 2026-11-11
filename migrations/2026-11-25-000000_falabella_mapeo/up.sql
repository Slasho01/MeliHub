-- Mapeo guardado para republicar sin re-elegir categoría/marca/atributos.
ALTER TABLE falabella_publicaciones ADD COLUMN categoria VARCHAR(30);
ALTER TABLE falabella_publicaciones ADD COLUMN marca VARCHAR(255);
ALTER TABLE falabella_publicaciones ADD COLUMN atributos JSONB NOT NULL DEFAULT '[]'::jsonb;
