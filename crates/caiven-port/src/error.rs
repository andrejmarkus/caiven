use std::io::Cursor;

use rocket::serde::json::serde_json;
use rocket::{
    Request,
    http::{ContentType, Status},
    response::{self, Responder, Response},
};

#[derive(Debug)]
pub enum ApiError {
    NotFound(String),
    BadRequest(String),
    PayloadTooLarge(String),
    Unauthorized,
    Forbidden(String),
    Conflict(String),
    TooManyRequests(String),
    Internal(String),
}

impl ApiError {
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::NotFound(msg.into())
    }
    pub fn bad_request(msg: impl Into<String>) -> Self {
        Self::BadRequest(msg.into())
    }
    pub fn forbidden(msg: impl Into<String>) -> Self {
        Self::Forbidden(msg.into())
    }
    pub fn conflict(msg: impl Into<String>) -> Self {
        Self::Conflict(msg.into())
    }
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }
}

impl<'r> Responder<'r, 'static> for ApiError {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'static> {
        let (status, msg) = match self {
            ApiError::NotFound(m) => (Status::NotFound, m),
            ApiError::BadRequest(m) => (Status::BadRequest, m),
            ApiError::PayloadTooLarge(m) => (Status::PayloadTooLarge, m),
            ApiError::Unauthorized => (Status::Unauthorized, "authentication required".into()),
            ApiError::Forbidden(m) => (Status::Forbidden, m),
            ApiError::Conflict(m) => (Status::Conflict, m),
            ApiError::TooManyRequests(m) => (Status::TooManyRequests, m),
            ApiError::Internal(m) => (Status::InternalServerError, m),
        };
        let body = serde_json::json!({"error": msg}).to_string();
        Response::build()
            .status(status)
            .header(ContentType::JSON)
            .sized_body(body.len(), Cursor::new(body))
            .ok()
    }
}

/// Internal-error message sent to clients — never the underlying error text,
/// which can carry table/column/SQL detail (see PORT-07). The real error is
/// logged server-side by the `From` impls below.
const INTERNAL_ERROR_MESSAGE: &str = "internal error";

impl From<anyhow::Error> for ApiError {
    fn from(e: anyhow::Error) -> Self {
        log::error!("internal error: {e:#}");
        ApiError::Internal(INTERNAL_ERROR_MESSAGE.into())
    }
}

impl From<sea_orm::DbErr> for ApiError {
    fn from(e: sea_orm::DbErr) -> Self {
        log::error!("database error: {e}");
        ApiError::Internal(INTERNAL_ERROR_MESSAGE.into())
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(e: serde_json::Error) -> Self {
        ApiError::BadRequest(e.to_string())
    }
}

impl From<std::io::Error> for ApiError {
    fn from(e: std::io::Error) -> Self {
        log::error!("io error: {e}");
        ApiError::Internal(INTERNAL_ERROR_MESSAGE.into())
    }
}

impl From<uuid::Error> for ApiError {
    fn from(e: uuid::Error) -> Self {
        log::error!("uuid parse error: {e}");
        ApiError::Internal(INTERNAL_ERROR_MESSAGE.into())
    }
}

impl From<webauthn_rs::prelude::WebauthnError> for ApiError {
    fn from(e: webauthn_rs::prelude::WebauthnError) -> Self {
        log::error!("webauthn error: {e}");
        ApiError::Internal(INTERNAL_ERROR_MESSAGE.into())
    }
}
