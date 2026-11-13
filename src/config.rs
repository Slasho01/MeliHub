//! Configuración leída del entorno (.env en desarrollo).

#[derive(Clone)]
pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub host: String,
    pub port: u16,
    pub cors_origins: Vec<String>,
    pub ml: MlConfig,
    /// URL pública de MeliHub (para back_url de pagos, etc.).
    pub app_url: String,
    /// Access token de Mercado Pago (del dueño de MeliHub) para cobrar suscripciones.
    pub mp_access_token: String,
    /// API key de Resend (envío de correos por HTTP; Render bloquea SMTP).
    pub resend_api_key: String,
    /// Remitente de los correos (ej. "MeliHub <no-reply@tudominio.cl>").
    pub mail_from: String,
    /// Credenciales de la app de Shopify (OAuth): API key, API secret y scopes.
    pub shopify_api_key: String,
    pub shopify_api_secret: String,
    pub shopify_scopes: String,
}

impl Config {
    /// Shopify (OAuth) activo solo si hay credenciales de app.
    pub fn shopify_configurado(&self) -> bool {
        !self.shopify_api_key.trim().is_empty() && !self.shopify_api_secret.trim().is_empty()
    }
}

#[derive(Clone)]
pub struct MlConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub api_base: String,
    pub auth_base: String,
}

impl MlConfig {
    /// La integración con ML solo está activa si hay credenciales.
    pub fn configurado(&self) -> bool {
        !self.client_id.trim().is_empty() && !self.client_secret.trim().is_empty()
    }
}

fn var(key: &str, default: &str) -> String {
    std::env::var(key).ok().filter(|v| !v.trim().is_empty()).unwrap_or_else(|| default.to_string())
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        let database_url = std::env::var("DATABASE_URL").map_err(|_| "Falta DATABASE_URL")?;
        let jwt_secret = std::env::var("JWT_SECRET").map_err(|_| "Falta JWT_SECRET")?;
        if jwt_secret.len() < 16 {
            return Err("JWT_SECRET debe tener al menos 16 caracteres".into());
        }
        Ok(Config {
            database_url,
            jwt_secret,
            host: var("HOST", "127.0.0.1"),
            port: var("PORT", "8090").parse().map_err(|_| "PORT inválido")?,
            cors_origins: var("CORS_ORIGINS", "")
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            ml: MlConfig {
                client_id: var("ML_CLIENT_ID", ""),
                client_secret: var("ML_CLIENT_SECRET", ""),
                redirect_uri: var("ML_REDIRECT_URI", "http://localhost:8090/api/ml/callback"),
                api_base: var("ML_API_BASE", "https://api.mercadolibre.com").trim_end_matches('/').to_string(),
                auth_base: var("ML_AUTH_BASE", "https://auth.mercadolibre.cl").trim_end_matches('/').to_string(),
            },
            app_url: var("APP_URL", "http://localhost:8090").trim_end_matches('/').to_string(),
            mp_access_token: var("MP_ACCESS_TOKEN", ""),
            resend_api_key: var("RESEND_API_KEY", ""),
            mail_from: var("MAIL_FROM", "MeliHub <onboarding@resend.dev>"),
            shopify_api_key: var("SHOPIFY_API_KEY", ""),
            shopify_api_secret: var("SHOPIFY_API_SECRET", ""),
            shopify_scopes: var("SHOPIFY_SCOPES", "read_products,write_products,read_inventory,write_inventory,read_locations,read_orders"),
        })
    }
}
