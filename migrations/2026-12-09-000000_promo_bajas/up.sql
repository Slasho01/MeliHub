-- Lista de supresión: correos que pidieron no recibir correos de promoción.
CREATE TABLE promo_bajas (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) NOT NULL UNIQUE,
    creado_at TIMESTAMP NOT NULL DEFAULT now()
);
