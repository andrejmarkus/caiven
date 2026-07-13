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
    Internal(String),
}

impl ApiError {
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::NotFound(msg.into())
    }
    pub fn bad_request(msg: impl Into<String>) -> Self {
        Self::BadRequest(msg.into())
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
            ApiError::Unauthorized => (Status::Unauthorized, "invalid api key".into()),
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

impl From<anyhow::Error> for ApiError {
    fn from(e: anyhow::Error) -> Self {
        ApiError::Internal(e.to_string())
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(e: serde_json::Error) -> Self {
        ApiError::BadRequest(e.to_string())
    }
}
