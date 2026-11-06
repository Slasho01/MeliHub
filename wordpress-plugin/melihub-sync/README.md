# MeliHub Sync (WooCommerce)

Plugin que conecta una tienda WooCommerce con **MeliHub**: trae los productos
(con el precio del canal) y reporta las ventas para mantener el stock
sincronizado en dos vías (WooCommerce ↔ MeliHub ↔ Mercado Libre).

## Instalación
1. Comprime la carpeta `melihub-sync` en un `.zip`.
2. En WordPress: **Plugins → Añadir nuevo → Subir plugin** → sube el `.zip` → **Activar**.
   (Requiere WooCommerce activo.)
3. Ve a **WooCommerce → MeliHub Sync** y configura:
   - **URL de MeliHub**: ej. `https://melihub.onrender.com`
   - **API Key**: créala en MeliHub → *API keys* (elige el canal que corresponda).
4. Pulsa **Sincronizar ahora**. Luego se sincroniza solo cada 15 minutos.

## Qué hace
- **Importa** los productos de MeliHub a WooCommerce (los empareja por SKU = ID de
  Mercado Libre; guarda el id de MeliHub como metadato para reportar ventas).
  Actualiza nombre, precio, stock, estado, descripción, ficha técnica (atributos)
  y categoría. La imagen se (re)descarga si cambia.
- La sincronización se hace **por páginas** (50 a la vez), así que no se cae con
  catálogos grandes; si un producto falla, sigue con el resto y lo reporta.
- Al **vender** en WooCommerce, descuenta stock en MeliHub (que a su vez lo baja
  en Mercado Libre).
- Cada 15 min **re-sincroniza** para reflejar ventas ocurridas en Mercado Libre.
- **Ocultar retirados** (opcional): los productos que ya no vienen de MeliHub se
  pasan a borrador automáticamente.
- **Probar conexión**: verifica URL + API key y muestra cuántos productos hay en
  el canal, antes de sincronizar.

## Notas
- MeliHub debe estar accesible por HTTPS desde el servidor de WordPress.
- La API key define qué productos y qué precio de canal recibe esta tienda.
