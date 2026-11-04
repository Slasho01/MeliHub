use crate::controller::{auth_controller as auth, health, ml_controller as ml, producto_controller as prod};
use actix_web::web::{self, delete, get, post, resource as r};

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
            // Catálogo unificado
            .service(r("/productos").route(get().to(prod::listar))),
    );
}
