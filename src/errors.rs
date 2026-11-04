use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use serde_json::json;
use std::fmt;

/// Error de la API, con su representación HTTP + JSON.
#[derive(Debug)]
pub enum ApiError {
    BadRequest(String),
    Unauthorized(String),
    Forbidden(String),
    NotFound(String),
    Conflict(String),
    Internal(String),
}

pub type ApiResult<T> = Result<T, ApiError>;

impl ApiError {
    pub fn bad_request(m: impl Into<String>) -> Self {
        ApiError::BadRequest(m.into())
    }
    pub fn unauthorized(m: impl Into<String>) -> Self {
        ApiError::Unauthorized(m.into())
    }
    pub fn forbidden(m: impl Into<String>) -> Self {
        ApiError::Forbidden(m.into())
    }
    pub fn not_found(m: impl Into<String>) -> Self {
        ApiError::NotFound(m.into())
    }
    pub fn conflict(m: impl Into<String>) -> Self {
        ApiError::Conflict(m.into())
    }
    pub fn internal(m: impl Into<String>) -> Self {
        ApiError::Internal(m.into())
    }

    fn parts(&self) -> (StatusCode, &str, &str) {
        match self {
            ApiError::BadRequest(m) => (StatusCode::BAD_REQUEST, "BAD_REQUEST", m),
            ApiError::Unauthorized(m) => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED", m),
            ApiError::Forbidden(m) => (StatusCode::FORBIDDEN, "FORBIDDEN", m),
            ApiError::NotFound(m) => (StatusCode::NOT_FOUND, "NOT_FOUND", m),
            ApiError::Conflict(m) => (StatusCode::CONFLICT, "CONFLICT", m),
            ApiError::Internal(m) => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL", m),
        }
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (_, _, m) = self.parts();
        write!(f, "{m}")
    }
}

impl ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        self.parts().0
    }
    fn error_response(&self) -> HttpResponse {
        let (status, code, msg) = self.parts();
        if status == StatusCode::INTERNAL_SERVER_ERROR {
            log::error!("Error interno: {msg}");
        }
        HttpResponse::build(status).json(json!({ "code": code, "error": msg }))
    }
}

impl From<diesel::result::Error> for ApiError {
    fn from(e: diesel::result::Error) -> Self {
        use diesel::result::{DatabaseErrorKind, Error};
        match e {
            Error::NotFound => ApiError::not_found("Recurso no encontrado"),
            Error::DatabaseError(DatabaseErrorKind::UniqueViolation, info) => {
                ApiError::conflict(info.message().to_string())
            }
            other => ApiError::internal(other.to_string()),
        }
    }
}

impl From<diesel::r2d2::PoolError> for ApiError {
    fn from(e: diesel::r2d2::PoolError) -> Self {
        ApiError::internal(format!("Pool de conexiones: {e}"))
    }
}

impl From<actix_web::error::BlockingError> for ApiError {
    fn from(_: actix_web::error::BlockingError) -> Self {
        ApiError::internal("La tarea en segundo plano falló")
    }
}

/// Azúcar para convertir `Option` en 404.
pub trait OrNotFound<T> {
    fn or_not_found(self, what: &str) -> ApiResult<T>;
}
impl<T> OrNotFound<T> for Option<T> {
    fn or_not_found(self, what: &str) -> ApiResult<T> {
        self.ok_or_else(|| ApiError::not_found(format!("{what} no encontrado")))
    }
}
