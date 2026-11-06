DROP TABLE IF EXISTS auditoria;
ALTER TABLE vendedores DROP COLUMN IF EXISTS twofa_enabled;
ALTER TABLE vendedores DROP COLUMN IF EXISTS twofa_secret;
