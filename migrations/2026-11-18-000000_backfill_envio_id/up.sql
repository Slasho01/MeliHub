-- Rellena envio_id desde el JSON crudo ya guardado (sin re-sincronizar con ML).
UPDATE ordenes_ml
   SET envio_id = NULLIF(datos->'shipping'->>'id','')::bigint
 WHERE envio_id IS NULL
   AND datos->'shipping'->>'id' ~ '^[0-9]+$';
