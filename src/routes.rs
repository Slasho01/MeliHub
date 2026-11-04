use crate::controller::{api_key_controller as keys, auth_controller as auth, health, ml_controller as ml, producto_controller as prod, pub_controller as pubc};
use actix_web::web::{self, delete, get, post, put, resource as r};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .service(r("/health").route(get().to(health)))
            // Auth del hub
            .service(r("/auth/registro").route(post().to(auth::registrar)))
            .service(r("/auth/login").route(post().to(auth::login)))
            .service(r("/auth/me").route(get().to(auth::me)))
            // Mercado Libre: OAuth y cuentas
            .service(r("/ml/conectar").route(post().to(ml::conectar)))
            .service(r("/ml/callback").route(get().to(ml::callback)))
            .service(r("/ml/webhook").route(post().to(ml::webhook)))
            .service(r("/ml/cuentas").route(get().to(ml::listar_cuentas)))
            .service(r("/ml/cuentas/{id}").route(delete().to(ml::desconectar)))
            .service(r("/ml/cuentas/{id}/sincronizar").route(post().to(ml::sincronizar)))
            // Catálogo unificado + gestor de inventario
            .service(r("/productos").route(get().to(prod::listar)))
            .service(r("/productos/{id}/inventario").route(put().to(prod::set_inventario)))
            .service(r("/config/ganancias").route(get().to(prod::get_config)).route(put().to(prod::set_config)))
            // API keys (gestión por el vendedor autenticado)
            .service(r("/api-keys").route(get().to(keys::listar)).route(post().to(keys::crear)))
            .service(r("/api-keys/{id}").route(delete().to(keys::revocar)))
            // Público: lo consumen las plataformas externas con X-API-Key
            .service(r("/pub/productos").route(get().to(pubc::productos)))
            .service(r("/pub/ventas").route(post().to(pubc::reportar_venta))),
    );
}
