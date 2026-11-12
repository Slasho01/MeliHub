-- Shopify como canal: cada vendedor conecta su tienda (dominio + Admin API access token).
CREATE TABLE shopify_cuentas (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    vendedor_id UUID NOT NULL REFERENCES vendedores(id) ON DELETE CASCADE,
    nombre VARCHAR(255),
    shop_domain VARCHAR(255) NOT NULL,     -- ej. mitienda.myshopify.com
    access_token TEXT NOT NULL,            -- cifrado (crypto::cifrar)
    activa BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    updated_at TIMESTAMP NOT NULL DEFAULT now()
);
CREATE INDEX idx_shopify_cuentas_vendedor ON shopify_cuentas (vendedor_id);

-- Mapeo producto del hub -> producto en Shopify (para sync de stock/precio).
CREATE TABLE shopify_publicaciones (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    producto_id UUID NOT NULL REFERENCES productos(id) ON DELETE CASCADE,
    cuenta_id UUID NOT NULL REFERENCES shopify_cuentas(id) ON DELETE CASCADE,
    shopify_product_id VARCHAR(40) NOT NULL,
    variant_id VARCHAR(40),
    inventory_item_id VARCHAR(40),
    ultima_sync_at TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    UNIQUE (producto_id, cuenta_id)
);
