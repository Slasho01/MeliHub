-- Stock centralizado entre cuentas de ML por "grupo": productos del mismo grupo comparten stock.
-- Se asigna al clonar (origen y clon comparten grupo).
ALTER TABLE productos ADD COLUMN grupo_stock UUID;
CREATE INDEX idx_productos_grupo_stock ON productos (grupo_stock);

-- El clon aún no existe en el hub al clonar (se crea al sincronizar la cuenta destino),
-- así que el grupo queda "pendiente" por (cuenta, ml_item_id) y se aplica al importarse.
CREATE TABLE clon_grupo_pendiente (
    ml_cuenta_id UUID NOT NULL REFERENCES ml_cuentas(id) ON DELETE CASCADE,
    ml_item_id VARCHAR(40) NOT NULL,
    grupo_stock UUID NOT NULL,
    creado_at TIMESTAMP NOT NULL DEFAULT now(),
    PRIMARY KEY (ml_cuenta_id, ml_item_id)
);
