//! Accounts and authentication: argon2 password hashing, session cookies for
//! the web UI, per-user API tokens for CLI/Studio (same `X-Api-Key` header as
//! before), and a small in-memory per-IP rate limiter.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng, rand_core::RngCore},
};
use rocket::{
    http::Status,
    request::{FromRequest, Outcome, Request},
};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::HubState;
use crate::entities::{api_tokens, sessions, users};

pub const SESSION_COOKIE: &str = "fc_session";
pub const SESSION_DAYS: i64 = 30;

pub fn hash_password(password: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| e.to_string())
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    PasswordHash::new(hash)
        .map(|parsed| {
            Argon2::default()
                .verify_password(password.as_bytes(), &parsed)
                .is_ok()
        })
        .unwrap_or(false)
}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn sha256_hex(s: &str) -> String {
    to_hex(&Sha256::digest(s.as_bytes()))
}

/// Random 32-byte hex string, used for session ids and API tokens.
pub fn random_secret() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    to_hex(&bytes)
}

fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}

pub async fn create_session(db: &DatabaseConnection, user_id: &str) -> anyhow::Result<String> {
    let id = random_secret();
    let expires = chrono::Utc::now() + chrono::Duration::days(SESSION_DAYS);
    sessions::ActiveModel {
        id: Set(id.clone()),
        user_id: Set(user_id.to_string()),
        created_at: Set(now_rfc3339()),
        expires_at: Set(expires.to_rfc3339()),
    }
    .insert(db)
    .await?;
    Ok(id)
}

pub async fn delete_session(db: &DatabaseConnection, session_id: &str) -> anyhow::Result<()> {
    sessions::Entity::delete_by_id(session_id).exec(db).await?;
    Ok(())
}

/// Mint a new API token for a user; returns (token row id, plaintext token).
/// Only the SHA-256 of the token is stored.
pub async fn create_token(
    db: &DatabaseConnection,
    user_id: &str,
    name: &str,
) -> anyhow::Result<(String, String)> {
    let id = Uuid::new_v4().to_string();
    let token = random_secret();
    api_tokens::ActiveModel {
        id: Set(id.clone()),
        user_id: Set(user_id.to_string()),
        token_hash: Set(sha256_hex(&token)),
        name: Set(name.to_string()),
        created_at: Set(now_rfc3339()),
        last_used_at: Set(None),
    }
    .insert(db)
    .await?;
    Ok((id, token))
}

async fn user_for_session(db: &DatabaseConnection, session_id: &str) -> Option<users::Model> {
    let session = sessions::Entity::find_by_id(session_id)
        .one(db)
        .await
        .ok()??;
    let expires = chrono::DateTime::parse_from_rfc3339(&session.expires_at).ok()?;
    if expires < chrono::Utc::now() {
        let _ = sessions::Entity::delete_by_id(session_id).exec(db).await;
        return None;
    }
    users::Entity::find_by_id(&session.user_id)
        .one(db)
        .await
        .ok()?
}

async fn user_for_token(db: &DatabaseConnection, token: &str) -> Option<users::Model> {
    let hash = sha256_hex(token);
    let row = api_tokens::Entity::find()
        .filter(api_tokens::Column::TokenHash.eq(&hash))
        .one(db)
        .await
        .ok()??;
    let mut touch: api_tokens::ActiveModel = row.clone().into();
    touch.last_used_at = Set(Some(now_rfc3339()));
    let _ = touch.update(db).await;
    users::Entity::find_by_id(&row.user_id).one(db).await.ok()?
}

/// Authenticated user, accepted from either a session cookie (web) or an
/// `X-Api-Key` per-user token (CLI/Studio).
pub struct AuthUser {
    pub id: String,
    pub username: String,
    pub is_admin: bool,
}

impl From<users::Model> for AuthUser {
    fn from(u: users::Model) -> Self {
        AuthUser {
            id: u.id,
            username: u.username,
            is_admin: u.is_admin,
        }
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AuthUser {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, ()> {
        let Some(state) = req.rocket().state::<HubState>() else {
            return Outcome::Error((Status::InternalServerError, ()));
        };
        if let Some(cookie) = req.cookies().get(SESSION_COOKIE)
            && let Some(user) = user_for_session(&state.db, cookie.value()).await
        {
            return Outcome::Success(user.into());
        }
        if let Some(token) = req.headers().get_one("X-Api-Key")
            && let Some(user) = user_for_token(&state.db, token).await
        {
            return Outcome::Success(user.into());
        }
        Outcome::Error((Status::Unauthorized, ()))
    }
}

/// Client IP for rate limiting; falls back to loopback when unknown
/// (e.g. local test client).
pub struct ClientIp(pub String);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for ClientIp {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, ()> {
        let ip = req
            .client_ip()
            .map(|a| a.to_string())
            .unwrap_or_else(|| "127.0.0.1".into());
        Outcome::Success(ClientIp(ip))
    }
}

/// Fixed-window in-memory rate limiter keyed by (bucket, client key).
#[derive(Default)]
pub struct RateLimiter {
    windows: Mutex<HashMap<(String, String), (Instant, u32)>>,
}

impl RateLimiter {
    /// Record one hit; returns the hit count within the current window.
    pub fn hit(&self, bucket: &str, key: &str, window: Duration) -> u32 {
        let mut map = self.windows.lock().unwrap_or_else(|e| e.into_inner());
        let entry = map
            .entry((bucket.to_string(), key.to_string()))
            .or_insert((Instant::now(), 0));
        if entry.0.elapsed() > window {
            *entry = (Instant::now(), 0);
        }
        entry.1 += 1;
        entry.1
    }

    /// Current hit count without recording a new one.
    pub fn count(&self, bucket: &str, key: &str, window: Duration) -> u32 {
        let map = self.windows.lock().unwrap_or_else(|e| e.into_inner());
        match map.get(&(bucket.to_string(), key.to_string())) {
            Some((start, n)) if start.elapsed() <= window => *n,
            _ => 0,
        }
    }
}
