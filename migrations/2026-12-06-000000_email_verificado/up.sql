-- Verificación de correo al registrarse. Las cuentas existentes quedan verificadas
-- (grandfather) para no bloquear a nadie; las nuevas arrancan sin verificar.
ALTER TABLE vendedores ADD COLUMN email_verificado BOOLEAN NOT NULL DEFAULT false;
UPDATE vendedores SET email_verificado = true;
