mod auth;
mod config;
mod crypto;
mod controller;
mod db;
mod errors;
mod falabella;
mod jobs;
mod mailer;
mod dte;
mod email;
mod ml;
mod paris;
mod repricing;
mod shopify;
mod twofa;
mod models;
mod mp;
mod planes;
mod routes;
mod schema;
mod services;
mod sync;

use actix_cors::Cors;
use actix_web::middleware::{DefaultHeaders, Logger, NormalizePath, TrailingSlash};
use actix_web::{http, web, App, HttpServer};
use config::Config;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    // Monitoreo de errores (opcional): se activa si hay SENTRY_DSN.
    let _sentry = std::env::var("SENTRY_DSN").ok().filter(|s| !s.trim().is_empty()).map(|dsn| {
        sentry::init((dsn, sentry::ClientOptions { release: sentry::release_name!(), send_default_pii: false, ..Default::default() }))
    });
    if _sentry.is_some() {
        log::info!("Sentry activado (monitoreo de errores)");
    }

    let config = Config::from_env().unwrap_or_else(|e| {
        eprintln!("Error de configuración: {e}");
        std::process::exit(1);
    });
    let pool = db::establish_pool(&config.database_url).unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(1);
    });
    if let Err(e) = db::run_migrations(&pool) {
        eprintln!("{e}");
        std::process::exit(1);
    }

    let addr = (config.host.clone(), config.port);
    log::info!("MeliHub escuchando en http://{}:{}", addr.0, addr.1);
    if config.ml.configurado() {
        log::info!("Mercado Libre configurado (client_id presente)");
    } else {
        log::warn!("Mercado Libre NO configurado: define ML_CLIENT_ID y ML_CLIENT_SECRET para conectar cuentas");
    }

    let config_data = web::Data::new(config.clone());
    let pool_data = web::Data::new(pool);
    let ml_client = web::Data::new(ml::MlClient::new(config.ml.clone()));
    let falabella_client = web::Data::new(falabella::FalabellaClient::new());
    let paris_client = web::Data::new(paris::ParisClient::new());
    let login_limiter = web::Data::new(auth::LoginLimiter::new());
    let mp_client = web::Data::new(mp::MercadoPago::new(config.mp_access_token.clone()));
    let mailer = web::Data::new(mailer::Mailer::new(config.resend_api_key.clone(), config.mail_from.clone()));

    jobs::start(pool_data.clone(), ml_client.clone(), falabella_client.clone(), paris_client.clone(), config.ml.configurado());

    HttpServer::new(move || {
        // Sin CORS_ORIGINS definido, acotamos al propio origen (APP_URL) en vez de abrir a
        // cualquiera. El panel se sirve desde el mismo origen, así que no se rompe.
        let cors = if config_data.cors_origins.is_empty() {
            Cors::default().allowed_origin(&config_data.app_url)
        } else {
            config_data.cors_origins.iter().fold(Cors::default(), |c, o| c.allowed_origin(o))
        }
        .allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
        .allowed_headers(vec![http::header::AUTHORIZATION, http::header::CONTENT_TYPE])
        .max_age(3600);

        App::new()
            .app_data(config_data.clone())
            .app_data(pool_data.clone())
            .app_data(ml_client.clone())
            .app_data(falabella_client.clone())
            .app_data(paris_client.clone())
            .app_data(login_limiter.clone())
            .app_data(mp_client.clone())
            .app_data(mailer.clone())
            .app_data(web::JsonConfig::default().limit(1 << 20).error_handler(|err, _| {
                actix_web::error::Error::from(errors::ApiError::bad_request(format!("JSON inválido: {err}")))
            }))
            .wrap(sentry_actix::Sentry::new())
            .wrap(security_headers())
            .wrap(cors)
            .wrap(NormalizePath::new(TrailingSlash::Trim))
            .wrap(Logger::new("%a \"%r\" %s %Dms"))
            .configure(routes::configure)
            // Sirve el panel (web/dist) en todo lo que no sea /api. Las rutas del
            // SPA (p. ej. /inventario) caen a index.html.
            .service(
                actix_files::Files::new("/", "./web/dist")
                    .index_file("index.html")
                    .default_handler(actix_web::dev::fn_service(|req: actix_web::dev::ServiceRequest| async {
                        let (http_req, _payload) = req.into_parts();
                        let file = actix_files::NamedFile::open_async("./web/dist/index.html").await?;
                        let res = file.into_response(&http_req);
                        Ok(actix_web::dev::ServiceResponse::new(http_req, res))
                    })),
            )
    })
    .bind(addr)?
    .run()
    .await
}

fn security_headers() -> DefaultHeaders {
    // CSP: scripts/estilos propios (estilos inline del panel permitidos), imágenes
    // propias + data: (QR 2FA) + https (imágenes de Mercado Libre), nada de iframes.
    const CSP: &str = "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; \
        img-src 'self' data: https:; font-src 'self' data:; connect-src 'self'; \
        object-src 'none'; base-uri 'self'; frame-ancestors 'none'";
    DefaultHeaders::new()
        .add(("X-Content-Type-Options", "nosniff"))
        .add(("X-Frame-Options", "DENY"))
        .add(("Referrer-Policy", "strict-origin-when-cross-origin"))
        .add(("Content-Security-Policy", CSP))
        .add(("Strict-Transport-Security", "max-age=31536000; includeSubDomains"))
}
