//! HTTP route handlers, split by area. Shared helpers live here.

pub mod auth;
pub mod carts;
pub mod discovery;
pub mod legacy;
pub mod versions;

use std::io::Cursor;
use std::path::Path;

use rocket::{
    http::{Header, Status},
    request::Request,
    response::{self, Responder, Response},
};

use crate::error::ApiError;

pub(crate) async fn move_file(src: &Path, dst: &Path) -> std::io::Result<()> {
    match tokio::fs::rename(src, dst).await {
        Ok(()) => Ok(()),
        Err(e) if e.raw_os_error() == Some(18) => {
            tokio::fs::copy(src, dst).await?;
            let _ = tokio::fs::remove_file(src).await;
            Ok(())
        }
        Err(e) => Err(e),
    }
}

pub(crate) fn valid_id(s: &str) -> bool {
    s.len() == 36 && s.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
}

pub(crate) fn safe_filename(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .take(64)
        .collect()
}

// ── custom binary responder ───────────────────────────────────────────────────

pub struct BinaryFile {
    pub(crate) bytes: Vec<u8>,
    pub(crate) content_type: &'static str,
    pub(crate) disposition: String,
    pub(crate) cache: Option<&'static str>,
}

impl<'r> Responder<'r, 'static> for BinaryFile {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'static> {
        let mut b = Response::build();
        b.status(Status::Ok)
            .header(Header::new("Content-Type", self.content_type))
            .header(Header::new("Content-Disposition", self.disposition))
            .header(Header::new("Content-Length", self.bytes.len().to_string()));
        if let Some(cc) = self.cache {
            b.header(Header::new("Cache-Control", cc));
        }
        b.sized_body(self.bytes.len(), Cursor::new(self.bytes)).ok()
    }
}

#[rocket::catch(401)]
pub fn unauthorized() -> ApiError {
    ApiError::Unauthorized
}
