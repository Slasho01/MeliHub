<?php
/**
 * Plugin Name: MeliHub Sync
 * Description: Sincroniza productos de MeliHub (Mercado Libre) a WooCommerce y reporta las ventas para mantener el stock en dos vías.
 * Version: 0.1.1
 * Author: Jarstore
 * Requires Plugins: woocommerce
 */

if (!defined('ABSPATH')) exit;

define('MELIHUB_META_ID', '_melihub_id');

/* ----------------------------- Config ----------------------------- */

function melihub_opts() {
    return [
        'url' => rtrim(get_option('melihub_url', ''), '/'),
        'key' => get_option('melihub_key', ''),
    ];
}

/** Llama a la API pública de MeliHub con la API key. */
function melihub_api($method, $path, $body = null) {
    $o = melihub_opts();
    if (!$o['url'] || !$o['key']) {
        return new WP_Error('melihub', 'Falta configurar la URL y la API key de MeliHub.');
    }
    $args = [
        'method'  => $method,
        'timeout' => 20,
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
    return $data;
}

/* ----------------------------- Importar productos ----------------------------- */

/** Trae los productos de MeliHub y los crea/actualiza en WooCommerce. */
function melihub_sync_productos() {
    $items = melihub_api('GET', '/productos');
    if (is_wp_error($items)) return $items;

    $creados = 0; $actualizados = 0;
    foreach ($items as $it) {
        $sku = $it['ml_item_id'];
        $product_id = wc_get_product_id_by_sku($sku);
        $product = $product_id ? wc_get_product($product_id) : new WC_Product_Simple();

        $nuevo = !$product_id;
        $product->set_name($it['titulo']);
        $product->set_sku($sku);
        $product->set_regular_price((string) $it['precio']);
        $product->set_manage_stock(true);
        $product->set_stock_quantity((int) $it['stock']);
        $product->set_stock_status($it['stock'] > 0 ? 'instock' : 'outofstock');
        $product->set_status($it['estado'] === 'active' ? 'publish' : 'draft');
        if (!empty($it['descripcion'])) {
            $product->set_description($it['descripcion']);
        }
        // Ficha técnica -> atributos de producto (pestaña "Información adicional")
        if (!empty($it['especificaciones']) && is_array($it['especificaciones'])) {
            $attrs = [];
            foreach ($it['especificaciones'] as $i => $spec) {
                if (empty($spec['nombre']) || empty($spec['valor'])) continue;
                $a = new WC_Product_Attribute();
                $a->set_id(0); // atributo personalizado (no taxonomía)
                $a->set_name($spec['nombre']);
                $a->set_options([$spec['valor']]);
                $a->set_position($i);
                $a->set_visible(true);
                $a->set_variation(false);
                $attrs[] = $a;
            }
            if ($attrs) $product->set_attributes($attrs);
        }
        // Categoría -> término product_cat (se crea si no existe)
        if (!empty($it['categoria'])) {
            $term = get_term_by('name', $it['categoria'], 'product_cat');
            if (!$term) {
                $res = wp_insert_term($it['categoria'], 'product_cat');
                $term_id = is_wp_error($res) ? 0 : (int) $res['term_id'];
            } else {
                $term_id = (int) $term->term_id;
            }
            if ($term_id) $product->set_category_ids([$term_id]);
        }
        $product->update_meta_data(MELIHUB_META_ID, $it['id']);
        $id = $product->save();

        // Imagen: (re)descarga si es nueva o si cambió la URL de origen (p. ej. mayor calidad)
        $img_url = $it['thumbnail'] ?? '';
        if ($img_url && get_post_meta($id, '_melihub_img_src', true) !== $img_url) {
            melihub_set_imagen($id, $img_url);
            update_post_meta($id, '_melihub_img_src', $img_url);
        }
        $nuevo ? $creados++ : $actualizados++;
    }
    update_option('melihub_last_sync', current_time('mysql'));
    return ['creados' => $creados, 'actualizados' => $actualizados, 'total' => count($items)];
}

function melihub_set_imagen($product_id, $url) {
    require_once(ABSPATH . 'wp-admin/includes/media.php');
    require_once(ABSPATH . 'wp-admin/includes/file.php');
    require_once(ABSPATH . 'wp-admin/includes/image.php');
    $att_id = media_sideload_image($url, $product_id, null, 'id');
    if (!is_wp_error($att_id)) set_post_thumbnail($product_id, $att_id);
}

/* ----------------------------- Reportar ventas ----------------------------- */

/**
 * Al reducir stock por un pedido, reporta cada ítem a MeliHub para que baje el
 * stock allí y lo empuje a Mercado Libre.
 */
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
});

function melihub_admin_page() {
    $msg = '';
    if (isset($_POST['melihub_sync_now']) && check_admin_referer('melihub_sync')) {
        $r = melihub_sync_productos();
        $msg = is_wp_error($r)
            ? '<div class="notice notice-error"><p>' . esc_html($r->get_error_message()) . '</p></div>'
            : '<div class="notice notice-success"><p>Sincronizado: ' . intval($r['creados']) . ' creados, ' . intval($r['actualizados']) . ' actualizados (' . intval($r['total']) . ' en total).</p></div>';
    }
    $o = melihub_opts();
    $last = get_option('melihub_last_sync', 'nunca');
    ?>
    <div class="wrap">
        <h1>MeliHub Sync</h1>
        <?php echo $msg; ?>
        <form method="post" action="options.php">
            <?php settings_fields('melihub'); ?>
            <table class="form-table">
                <tr><th>URL de MeliHub</th><td><input type="url" name="melihub_url" value="<?php echo esc_attr($o['url']); ?>" class="regular-text" placeholder="https://melihub.onrender.com" /></td></tr>
                <tr><th>API Key</th><td><input type="password" name="melihub_key" value="<?php echo esc_attr($o['key']); ?>" class="regular-text" placeholder="mhk_..." /><p class="description">Créala en MeliHub → API keys (canal WordPress/JarStore).</p></td></tr>
            </table>
            <?php submit_button('Guardar'); ?>
        </form>
        <hr>
        <form method="post">
            <?php wp_nonce_field('melihub_sync'); ?>
            <p>Última sincronización: <b><?php echo esc_html($last); ?></b> · se sincroniza solo cada 15 minutos.</p>
            <button class="button button-primary" name="melihub_sync_now" value="1">Sincronizar ahora</button>
        </form>
    </div>
    <?php
}
