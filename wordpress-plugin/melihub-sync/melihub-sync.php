<?php
/**
 * Plugin Name: MeliHub Sync
 * Description: Sincroniza productos de MeliHub (Mercado Libre) a WooCommerce y reporta las ventas para mantener el stock en dos vías.
 * Version: 0.6.0
 * Author: Jarstore
 * Requires Plugins: woocommerce
 */

if (!defined('ABSPATH')) exit;

define('MELIHUB_META_ID', '_melihub_id');
define('MELIHUB_PAGE', 50); // productos por página al sincronizar

/* ----------------------------- Config ----------------------------- */

function melihub_opts() {
    return [
        'url'      => rtrim(get_option('melihub_url', ''), '/'),
        'key'      => get_option('melihub_key', ''),
        'huerfanos'=> get_option('melihub_huerfanos', '0') === '1', // ocultar los que ya no vienen
    ];
}

/** Llama a la API pública de MeliHub con la API key. $path puede incluir query. */
function melihub_api($method, $path, $body = null, &$headers_out = null) {
    $o = melihub_opts();
    if (!$o['url'] || !$o['key']) {
        return new WP_Error('melihub', 'Falta configurar la URL y la API key de MeliHub.');
    }
    $args = [
        'method'  => $method,
        'timeout' => 30,
        'headers' => ['X-API-Key' => $o['key'], 'Accept' => 'application/json', 'ngrok-skip-browser-warning' => '1'],
    ];
    if ($body !== null) {
        $args['headers']['Content-Type'] = 'application/json';
        $args['body'] = wp_json_encode($body);
    }
    $res = wp_remote_request($o['url'] . '/api/pub' . $path, $args);
    if (is_wp_error($res)) return $res;
    $code = wp_remote_retrieve_response_code($res);
    $data = json_decode(wp_remote_retrieve_body($res), true);
    if ($code < 200 || $code >= 300) {
        return new WP_Error('melihub', 'MeliHub respondió ' . $code . ': ' . (isset($data['error']) ? $data['error'] : ''));
    }
    if ($headers_out !== null) {
        $headers_out = ['total' => (int) wp_remote_retrieve_header($res, 'x-total-count')];
    }
    return $data;
}

/* ----------------------------- Importar productos ----------------------------- */

/** Crea/actualiza UN producto de MeliHub en WooCommerce. Devuelve [id, nuevo]. */
function melihub_upsert_producto($it) {
    // SKU ESTABLE del hub (catalog_product_id o item id): evita duplicar cuando el
    // representante salta entre la publicación de catálogo y la normal.
    $sku = !empty($it['sku']) ? $it['sku'] : $it['ml_item_id'];
    $product_id = wc_get_product_id_by_sku($sku);
    // Migración: si no existe con el SKU nuevo, búscalo por el id del hub (meta) o por el
    // item id antiguo, y re-apunta su SKU — así no se crea un duplicado.
    if (!$product_id && !empty($it['id'])) {
        $prev = wc_get_products(['limit' => 1, 'return' => 'ids', 'meta_key' => MELIHUB_META_ID, 'meta_value' => $it['id']]);
        if (!empty($prev)) $product_id = (int) $prev[0];
    }
    if (!$product_id && !empty($it['ml_item_id']) && $it['ml_item_id'] !== $sku) {
        $product_id = wc_get_product_id_by_sku($it['ml_item_id']);
    }
    $tiene_variantes = !empty($it['variantes']) && is_array($it['variantes']);

    // Tipo correcto (simple/variable). Si el existente es de otro tipo, se cambia.
    if ($product_id) {
        $tipo_deseado = $tiene_variantes ? 'variable' : 'simple';
        if (WC_Product_Factory::get_product_type($product_id) !== $tipo_deseado) {
            wp_set_object_terms($product_id, $tipo_deseado, 'product_type');
        }
        $product = $tiene_variantes ? new WC_Product_Variable($product_id) : new WC_Product_Simple($product_id);
    } else {
        $product = $tiene_variantes ? new WC_Product_Variable() : new WC_Product_Simple();
    }
    $nuevo = !$product_id;

    $product->set_name($it['titulo']);
    $product->set_sku($sku);
    $product->set_status($it['estado'] === 'active' ? 'publish' : 'draft');
    if (!empty($it['descripcion'])) $product->set_description($it['descripcion']);
    melihub_set_video($product, $it['video'] ?? '');

    if ($tiene_variantes) {
        // En variables, el precio y el stock los manejan las variaciones.
        $product->set_manage_stock(false);
    } else {
        $product->set_regular_price((string) $it['precio']);
        $product->set_manage_stock(true);
        $product->set_stock_quantity((int) $it['stock']);
        $product->set_stock_status($it['stock'] > 0 ? 'instock' : 'outofstock');
    }

    // Atributos: ficha técnica (no-variación) + los de variación (color/talla/etc.).
    $attrs = [];
    $pos = 0;
    if (!empty($it['especificaciones']) && is_array($it['especificaciones'])) {
        foreach ($it['especificaciones'] as $spec) {
            if (empty($spec['nombre']) || empty($spec['valor'])) continue;
            $a = new WC_Product_Attribute();
            $a->set_id(0); $a->set_name($spec['nombre']); $a->set_options([$spec['valor']]);
            $a->set_position($pos++); $a->set_visible(true); $a->set_variation(false);
            $attrs[] = $a;
        }
    }
    if ($tiene_variantes) {
        $varmap = []; // nombre_atributo => valores distintos
        foreach ($it['variantes'] as $v) {
            foreach (($v['atributos'] ?? []) as $a) {
                if (empty($a['nombre']) || empty($a['valor'])) continue;
                $varmap[$a['nombre']][$a['valor']] = true;
            }
        }
        foreach ($varmap as $nombre => $valset) {
            $a = new WC_Product_Attribute();
            $a->set_id(0); $a->set_name($nombre); $a->set_options(array_keys($valset));
            $a->set_position($pos++); $a->set_visible(true); $a->set_variation(true);
            $attrs[] = $a;
        }
    }
    if ($attrs) $product->set_attributes($attrs);

    // Categoría -> término product_cat (se crea si no existe y se marca para el menú).
    $cat_id = melihub_categoria_term_id($it['categoria'] ?? '');
    if ($cat_id) $product->set_category_ids([$cat_id]);
    $product->update_meta_data(MELIHUB_META_ID, $it['id']);
    $id = $product->save();

    // Galería del producto (padre). En variables, cada variación tiene además su imagen.
    $imgs = !empty($it['imagenes']) && is_array($it['imagenes']) ? $it['imagenes'] : array_filter([$it['thumbnail'] ?? '']);
    melihub_sync_imagenes($product, $imgs);
    if ($tiene_variantes) {
        melihub_sync_variantes($product, $it['variantes']);
    }
    return [$id, $nuevo];
}

/** Firma normalizada de una combinación de atributos [slug => valor] (para emparejar variaciones). */
function melihub_var_firma($attrs) {
    $norm = [];
    foreach ((array) $attrs as $k => $val) {
        $norm[sanitize_title($k)] = sanitize_title($val);
    }
    ksort($norm);
    return md5(wp_json_encode($norm));
}

/** Crea/actualiza las variaciones de WooCommerce desde las variantes de MeliHub. */
function melihub_sync_variantes($product, $variantes) {
    $parent_id = $product->get_id();
    // Variaciones existentes, indexadas por la firma de sus atributos (para no duplicar).
    $existentes = [];
    foreach ($product->get_children() as $cid) {
        $cv = wc_get_product($cid);
        if ($cv) $existentes[melihub_var_firma($cv->get_attributes())] = $cid;
    }
    $vistas = [];
    foreach ($variantes as $v) {
        $va = [];
        foreach (($v['atributos'] ?? []) as $a) {
            if (empty($a['nombre']) || empty($a['valor'])) continue;
            $va[sanitize_title($a['nombre'])] = $a['valor'];
        }
        if (!$va) continue;
        $firma = melihub_var_firma($va);
        $vistas[$firma] = true;
        $variation = isset($existentes[$firma]) ? wc_get_product($existentes[$firma]) : new WC_Product_Variation();
        $variation->set_parent_id($parent_id);
        $variation->set_attributes($va);
        $variation->set_regular_price((string) ($v['precio'] ?? 0));
        $variation->set_manage_stock(true);
        $variation->set_stock_quantity((int) ($v['stock'] ?? 0));
        $variation->set_stock_status(($v['stock'] ?? 0) > 0 ? 'instock' : 'outofstock');
        $vid = $variation->save();
        // Imagen propia de la variante (una firma para no re-subir).
        if (!empty($v['imagenes'][0])) {
            $firma_img = md5($v['imagenes'][0]);
            if (get_post_meta($vid, '_melihub_var_img', true) !== $firma_img) {
                require_once(ABSPATH . 'wp-admin/includes/media.php');
                require_once(ABSPATH . 'wp-admin/includes/file.php');
                require_once(ABSPATH . 'wp-admin/includes/image.php');
                $att = media_sideload_image($v['imagenes'][0], $parent_id, null, 'id');
                if (!is_wp_error($att)) {
                    $variation->set_image_id((int) $att);
                    $variation->save();
                    update_post_meta($vid, '_melihub_var_img', $firma_img);
                }
            }
        }
    }
    // Elimina variaciones que ya no vienen del hub.
    foreach ($existentes as $firma => $cid) {
        if (empty($vistas[$firma])) {
            $c = wc_get_product($cid);
            if ($c) $c->delete(true);
        }
    }
    if (class_exists('WC_Product_Variable')) {
        WC_Product_Variable::sync($parent_id);
    }
}

/** Descarga la galería: 1ª imagen = destacada, el resto = galería del producto. */
function melihub_sync_imagenes($product, $imagenes) {
    $imagenes = array_values(array_filter((array) $imagenes));
    if (!$imagenes) return;
    $id = $product->get_id();
    $firma = md5(implode('|', $imagenes));
    if (get_post_meta($id, '_melihub_img_firma', true) === $firma) return; // sin cambios
    require_once(ABSPATH . 'wp-admin/includes/media.php');
    require_once(ABSPATH . 'wp-admin/includes/file.php');
    require_once(ABSPATH . 'wp-admin/includes/image.php');
    $ids = [];
    foreach ($imagenes as $url) {
        $att = media_sideload_image($url, $id, null, 'id');
        if (!is_wp_error($att)) $ids[] = (int) $att;
    }
    if (!$ids) return;
    set_post_thumbnail($id, $ids[0]);
    $product->set_gallery_image_ids(count($ids) > 1 ? array_slice($ids, 1) : []);
    $product->save();
    update_post_meta($id, '_melihub_img_firma', $firma);
}

/** Inserta/actualiza el video (YouTube) en la descripción, para que WordPress lo embeba.
 *  La URL va sola en su línea (auto-embed) entre marcadores, para reemplazarla en cada sync. */
function melihub_set_video($product, $video) {
    $desc = (string) $product->get_description();
    $desc = preg_replace('#\s*<!--melihub-video-->.*?<!--/melihub-video-->#s', '', $desc);
    $video = trim((string) $video);
    if ($video !== '') {
        $desc = rtrim($desc) . "\n\n<!--melihub-video-->\n" . esc_url_raw($video) . "\n<!--/melihub-video-->";
    }
    $product->set_description($desc);
}

/** Devuelve el term_id de la categoría product_cat (la crea si no existe) y la marca
 *  como gestionada por MeliHub para poder añadirla al menú. Robusto ante "term_exists". */
function melihub_categoria_term_id($nombre) {
    $nombre = trim((string) $nombre);
    if ($nombre === '') return 0;
    $term = get_term_by('name', $nombre, 'product_cat');
    if ($term && !is_wp_error($term)) {
        $tid = (int) $term->term_id;
    } else {
        $r = wp_insert_term($nombre, 'product_cat');
        if (is_wp_error($r)) {
            // Si ya existía (carrera/acentos), WooCommerce devuelve el term_id en el error.
            $data = $r->get_error_data();
            if (is_array($data) && !empty($data['term_id'])) $tid = (int) $data['term_id'];
            elseif (is_int($data)) $tid = (int) $data;
            else { $t2 = get_term_by('name', $nombre, 'product_cat'); $tid = $t2 ? (int) $t2->term_id : 0; }
        } else {
            $tid = (int) $r['term_id'];
        }
    }
    if ($tid && !get_term_meta($tid, '_melihub_cat', true)) {
        add_term_meta($tid, '_melihub_cat', '1', true); // marca para el menú
    }
    return $tid;
}

/** Añade al menú elegido las categorías de MeliHub que tengan productos. Solo AGREGA
 *  (no borra lo que el usuario puso a mano) y evita duplicados. Devuelve cuántas añadió. */
function melihub_sync_menu_categorias() {
    $menu_id = (int) get_option('melihub_menu', 0);
    if (!$menu_id) return 0; // desactivado: el usuario no eligió menú
    $menu = wp_get_nav_menu_object($menu_id);
    if (!$menu) return 0;

    // Categorías creadas por MeliHub que tienen al menos un producto.
    $terms = get_terms([
        'taxonomy'   => 'product_cat',
        'hide_empty' => true,
        'meta_query' => [['key' => '_melihub_cat', 'value' => '1']],
    ]);
    if (is_wp_error($terms) || !$terms) return 0;

    // Categorías ya presentes en el menú (para no duplicar).
    $items = wp_get_nav_menu_items($menu_id);
    $ya = [];
    if (is_array($items)) {
        foreach ($items as $mi) {
            if ($mi->type === 'taxonomy' && $mi->object === 'product_cat') $ya[(int) $mi->object_id] = true;
        }
    }
    $add = 0;
    foreach ($terms as $t) {
        if (isset($ya[(int) $t->term_id])) continue;
        $r = wp_update_nav_menu_item($menu_id, 0, [
            'menu-item-title'     => $t->name,
            'menu-item-object'    => 'product_cat',
            'menu-item-object-id' => (int) $t->term_id,
            'menu-item-type'      => 'taxonomy',
            'menu-item-status'    => 'publish',
        ]);
        if (!is_wp_error($r)) $add++;
    }
    return $add;
}

/** Trae TODOS los productos (por páginas) y los sincroniza en WooCommerce. */
function melihub_sync_productos() {
    if (function_exists('set_time_limit')) @set_time_limit(0);

    $creados = 0; $actualizados = 0; $errores = 0; $total_api = 0;
    $vistos = [];      // melihub_id que siguen viniendo (para detectar huérfanos)
    $offset = 0;

    do {
        $h = [];
        $items = melihub_api('GET', '/productos?limit=' . MELIHUB_PAGE . '&offset=' . $offset, null, $h);
        if (is_wp_error($items)) return $items;
        $total_api = $h['total'] ?: $total_api;
        if (!is_array($items) || count($items) === 0) break;

        foreach ($items as $it) {
            try {
                list($id, $nuevo) = melihub_upsert_producto($it);
                $vistos[] = $it['id'];
                $nuevo ? $creados++ : $actualizados++;
            } catch (\Throwable $e) {
                $errores++;
                error_log('MeliHub: error en ' . ($it['ml_item_id'] ?? '?') . ': ' . $e->getMessage());
            }
        }
        $offset += count($items);
    } while (count($items) === MELIHUB_PAGE);

    // Huérfanos: productos con melihub_id que ya no vienen -> a borrador (ocultar).
    $ocultos = 0;
    $o = melihub_opts();
    if ($o['huerfanos'] && $vistos) {
        $ocultos = melihub_ocultar_huerfanos($vistos);
    }

    // Añade las categorías nuevas al menú elegido (si el usuario configuró uno).
    $menu_add = melihub_sync_menu_categorias();

    $resultado = [
        'creados' => $creados, 'actualizados' => $actualizados, 'errores' => $errores,
        'ocultos' => $ocultos, 'menu' => $menu_add, 'total' => $total_api ?: ($creados + $actualizados),
        'fecha' => current_time('mysql'),
    ];
    update_option('melihub_last_sync', $resultado['fecha']);
    update_option('melihub_last_result', $resultado);
    return $resultado;
}

/** Pone en borrador los productos MeliHub que ya no aparecen en el feed. */
function melihub_ocultar_huerfanos($vistos_ids) {
    $ids = get_posts([
        'post_type'   => 'product',
        'post_status' => 'publish',
        'numberposts' => -1,
        'fields'      => 'ids',
        'meta_key'    => MELIHUB_META_ID,
    ]);
    $set = array_flip($vistos_ids);
    $n = 0;
    foreach ($ids as $pid) {
        $mid = get_post_meta($pid, MELIHUB_META_ID, true);
        if ($mid && !isset($set[$mid])) {
            wp_update_post(['ID' => $pid, 'post_status' => 'draft']);
            $n++;
        }
    }
    return $n;
}

/* ----------------------------- Reportar ventas ----------------------------- */

add_action('woocommerce_reduce_order_stock', function ($order) {
    foreach ($order->get_items() as $item) {
        $product = $item->get_product();
        if (!$product) continue;
        $mid = $product->get_meta(MELIHUB_META_ID);
        if (!$mid) continue;
        $qty = (int) $item->get_quantity();
        if ($qty <= 0) continue;
        $r = melihub_api('POST', '/ventas', ['producto_id' => $mid, 'cantidad' => $qty]);
        if (is_wp_error($r)) {
            error_log('MeliHub: no se pudo reportar venta de ' . $product->get_sku() . ': ' . $r->get_error_message());
        }
    }
});

/* ----------------------------- Sincronización periódica ----------------------------- */

add_filter('cron_schedules', function ($s) {
    $s['melihub_15min'] = ['interval' => 900, 'display' => 'Cada 15 minutos (MeliHub)'];
    return $s;
});
register_activation_hook(__FILE__, function () {
    if (!wp_next_scheduled('melihub_cron_sync')) wp_schedule_event(time() + 60, 'melihub_15min', 'melihub_cron_sync');
});
register_deactivation_hook(__FILE__, function () {
    wp_clear_scheduled_hook('melihub_cron_sync');
});
add_action('melihub_cron_sync', 'melihub_sync_productos');

/* ----------------------------- Panel de administración ----------------------------- */

add_action('admin_menu', function () {
    add_submenu_page('woocommerce', 'MeliHub Sync', 'MeliHub Sync', 'manage_woocommerce', 'melihub-sync', 'melihub_admin_page');
});

add_action('admin_init', function () {
    register_setting('melihub', 'melihub_url');
    register_setting('melihub', 'melihub_key');
    register_setting('melihub', 'melihub_huerfanos');
    register_setting('melihub', 'melihub_menu');
});

function melihub_admin_page() {
    $msg = '';
    if (isset($_POST['melihub_sync_now']) && check_admin_referer('melihub_sync')) {
        $r = melihub_sync_productos();
        $msg = is_wp_error($r)
            ? '<div class="notice notice-error"><p>' . esc_html($r->get_error_message()) . '</p></div>'
            : '<div class="notice notice-success"><p>Sincronizado: ' . intval($r['creados']) . ' creados, ' . intval($r['actualizados']) . ' actualizados'
              . ($r['errores'] ? ', ' . intval($r['errores']) . ' con error' : '')
              . ($r['ocultos'] ? ', ' . intval($r['ocultos']) . ' ocultados' : '')
              . (!empty($r['menu']) ? ', ' . intval($r['menu']) . ' categorías al menú' : '')
              . ' (' . intval($r['total']) . ' en total).</p></div>';
    }
    if (isset($_POST['melihub_test']) && check_admin_referer('melihub_test_c')) {
        $h = [];
        $r = melihub_api('GET', '/productos?limit=1', null, $h);
        $msg = is_wp_error($r)
            ? '<div class="notice notice-error"><p>Conexión fallida: ' . esc_html($r->get_error_message()) . '</p></div>'
            : '<div class="notice notice-success"><p>Conexión OK. Productos disponibles en este canal: <b>' . intval($h['total']) . '</b>.</p></div>';
    }
    $o = melihub_opts();
    $last = get_option('melihub_last_sync', 'nunca');
    $lr = get_option('melihub_last_result', null);
    ?>
    <div class="wrap">
        <h1>MeliHub Sync</h1>
        <?php echo $msg; ?>
        <form method="post" action="options.php">
            <?php settings_fields('melihub'); ?>
            <table class="form-table">
                <tr><th>URL de MeliHub</th><td><input type="url" name="melihub_url" value="<?php echo esc_attr($o['url']); ?>" class="regular-text" placeholder="https://melihub.onrender.com" /></td></tr>
                <tr><th>API Key</th><td><input type="password" name="melihub_key" value="<?php echo esc_attr($o['key']); ?>" class="regular-text" placeholder="mhk_..." /><p class="description">Créala en MeliHub → API keys (nombre de tu canal).</p></td></tr>
                <tr><th>Ocultar retirados</th><td><label><input type="checkbox" name="melihub_huerfanos" value="1" <?php checked($o['huerfanos']); ?> /> Poner en borrador los productos que ya no vienen de MeliHub.</label></td></tr>
                <tr><th>Añadir categorías al menú</th><td>
                    <?php $menus = wp_get_nav_menus(); $sel = (int) get_option('melihub_menu', 0); ?>
                    <select name="melihub_menu">
                        <option value="0"><?php esc_html_e('— No añadir al menú —'); ?></option>
                        <?php foreach ($menus as $m): ?>
                            <option value="<?php echo (int) $m->term_id; ?>" <?php selected($sel, (int) $m->term_id); ?>><?php echo esc_html($m->name); ?></option>
                        <?php endforeach; ?>
                    </select>
                    <p class="description">Las categorías de los productos sincronizados se agregan a este menú (sin duplicar y sin tocar lo que pusiste a mano).
                    <?php if (!$menus): ?><br><b>No tienes menús.</b> Crea uno en Apariencia → Menús y vuelve aquí.<?php endif; ?></p>
                </td></tr>
            </table>
            <?php submit_button('Guardar'); ?>
        </form>
        <hr>
        <p>Última sincronización: <b><?php echo esc_html($last); ?></b> · automática cada 15 minutos.
        <?php if (is_array($lr)): ?>
            <br>Último resultado: <?php echo intval($lr['creados']); ?> creados, <?php echo intval($lr['actualizados']); ?> actualizados<?php
                if (!empty($lr['errores'])) echo ', <span style="color:#b32d2e">' . intval($lr['errores']) . ' con error</span>';
                if (!empty($lr['ocultos'])) echo ', ' . intval($lr['ocultos']) . ' ocultados';
            ?>.
        <?php endif; ?></p>
        <form method="post" style="display:inline">
            <?php wp_nonce_field('melihub_sync'); ?>
            <button class="button button-primary" name="melihub_sync_now" value="1">Sincronizar ahora</button>
        </form>
        <form method="post" style="display:inline; margin-left:8px">
            <?php wp_nonce_field('melihub_test_c'); ?>
            <button class="button" name="melihub_test" value="1">Probar conexión</button>
        </form>
    </div>
    <?php
}
