-- Configuración de correo (global, admin): proveedor Resend (HTTP, funciona en Render) o SMTP
-- propio (requiere host sin bloqueo SMTP, p. ej. un VPS). Secretos cifrados.
CREATE TABLE config_correo (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    proveedor VARCHAR(10) NOT NULL DEFAULT 'resend',   -- 'resend' | 'smtp'
    resend_api_key TEXT,                                -- cifrado
    smtp_host VARCHAR(255),
    smtp_port INT4 NOT NULL DEFAULT 587,
    smtp_usuario VARCHAR(255),
    smtp_password TEXT,                                 -- cifrado
    smtp_tls BOOLEAN NOT NULL DEFAULT TRUE,
    remitente VARCHAR(255),                             -- "Nombre <correo@dominio>"
    updated_at TIMESTAMP NOT NULL DEFAULT now()
);
