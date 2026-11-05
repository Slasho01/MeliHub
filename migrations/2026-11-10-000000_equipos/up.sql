-- Miembros de un equipo: personas adicionales que acceden a la cuenta de un
-- vendedor (la "organización") con un rol. El dueño es el propio vendedor.
CREATE TABLE miembros (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    vendedor_id UUID NOT NULL REFERENCES vendedores(id) ON DELETE CASCADE, -- la organización
    nombre VARCHAR(255) NOT NULL,
    email VARCHAR(255) NOT NULL,
    password_hash VARCHAR(255),              -- null hasta que acepta la invitación
    rol VARCHAR(20) NOT NULL DEFAULT 'operador', -- operador | lectura
    estado VARCHAR(20) NOT NULL DEFAULT 'invitado', -- invitado | activo
    invite_token VARCHAR(255),
    invite_expira TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE UNIQUE INDEX miembros_email_uniq ON miembros (lower(email));
CREATE INDEX miembros_vendedor_idx ON miembros (vendedor_id);
CREATE UNIQUE INDEX miembros_invite_token_uniq ON miembros (invite_token) WHERE invite_token IS NOT NULL;
