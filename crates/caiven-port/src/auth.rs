//! Accounts and authentication: argon2 password hashing, session cookies for
//! the web UI, per-user API tokens for CLI/Studio (same `X-Api-Key` header as
//! before), and a small in-memory per-IP rate limiter.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng, rand_core::RngCore},
};
use rocket::{
    http::{Method, Status},
    request::{FromRequest, Outcome, Request},
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};
use sha1::Sha1;
use sha2::{Digest, Sha256};
use totp_rs::{Algorithm, Secret, TOTP};
use uuid::Uuid;

use crate::PortState;
use crate::entities::{
    api_tokens, audit_log, email_tokens, mfa_backup_codes, mfa_challenges, sessions, users,
    webauthn_challenges,
};

pub const SESSION_COOKIE: &str = "caiven_session";
pub const SESSION_DAYS: i64 = 30;
pub const MAX_SESSIONS_PER_USER: usize = 20;
/// Only touch (write) `last_seen_at` when it's this stale, so an active
/// session isn't hit with a write on every single request.
const SESSION_TOUCH_INTERVAL_SECS: i64 = 300;

pub const EMAIL_VERIFY_TOKEN_HOURS: i64 = 24;
pub const PASSWORD_RESET_TOKEN_HOURS: i64 = 1;

/// Double-submit CSRF cookie: readable by JS (not `HttpOnly`), sent back as
/// a header on state-changing requests. Only meaningful for cookie-based
/// sessions — `X-Api-Key` requests carry no ambient cookie and can't be
/// forged cross-site, so they're exempt.
pub const CSRF_COOKIE: &str = "caiven_csrf";
pub const CSRF_HEADER: &str = "X-CSRF-Token";

pub const MFA_CHALLENGE_MINUTES: i64 = 5;
pub const MFA_BACKUP_CODE_COUNT: usize = 10;
pub const MFA_ISSUER: &str = "Caiven Port";

pub const WEBAUTHN_CHALLENGE_MINUTES: i64 = 5;

static DUMMY_PASSWORD_HASH: OnceLock<String> = OnceLock::new();

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

pub async fn hash_password_async(password: String) -> Result<String, String> {
    tokio::task::spawn_blocking(move || hash_password(&password))
        .await
        .map_err(|e| e.to_string())?
}

/// Always runs Argon2, including when no account exists, reducing login
/// username-enumeration timing differences.
pub async fn verify_login_password(password: String, hash: Option<String>) -> bool {
    tokio::task::spawn_blocking(move || {
        let has_user = hash.is_some();
        let hash = hash.unwrap_or_else(|| {
            DUMMY_PASSWORD_HASH
                .get_or_init(|| {
                    hash_password("caiven dummy password never used")
                        .expect("static dummy password must hash")
                })
                .clone()
        });
        verify_password(&password, &hash) && has_user
    })
    .await
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

/// Lowercase + trim, for the unique index and identifier-based login.
pub fn normalize_email(email: &str) -> String {
    email.trim().to_ascii_lowercase()
}

pub fn is_valid_email(email: &str) -> bool {
    let Some((local, domain)) = email.split_once('@') else {
        return false;
    };
    !local.is_empty()
        && !domain.is_empty()
        && domain.contains('.')
        && !email.chars().any(char::is_whitespace)
        && email.len() <= 254
}

/// Creates a single-use, hashed email token (`verify` or `reset`),
/// invalidating any earlier unused tokens of the same kind for this user so
/// only the newest link works. Returns the plaintext token to embed in a
/// link.
pub async fn create_email_token(
    db: &DatabaseConnection,
    user_id: &str,
    kind: &str,
    ttl_hours: i64,
) -> anyhow::Result<String> {
    let stale = email_tokens::Entity::find()
        .filter(email_tokens::Column::UserId.eq(user_id))
        .filter(email_tokens::Column::Kind.eq(kind))
        .filter(email_tokens::Column::UsedAt.is_null())
        .all(db)
        .await?;
    for row in stale {
        let mut update: email_tokens::ActiveModel = row.into();
        update.used_at = Set(Some(now_rfc3339()));
        update.update(db).await?;
    }

    let token = random_secret();
    let expires = chrono::Utc::now() + chrono::Duration::hours(ttl_hours);
    email_tokens::ActiveModel {
        id: Set(Uuid::new_v4().to_string()),
        user_id: Set(user_id.to_string()),
        kind: Set(kind.to_string()),
        token_hash: Set(sha256_hex(&token)),
        created_at: Set(now_rfc3339()),
        expires_at: Set(expires.to_rfc3339()),
        used_at: Set(None),
    }
    .insert(db)
    .await?;
    Ok(token)
}

/// Validates and consumes a single-use email token of the given kind,
/// returning the associated user id.
pub async fn consume_email_token(
    db: &DatabaseConnection,
    token: &str,
    kind: &str,
) -> anyhow::Result<Option<String>> {
    let hash = sha256_hex(token);
    let Some(row) = email_tokens::Entity::find()
        .filter(email_tokens::Column::TokenHash.eq(&hash))
        .filter(email_tokens::Column::Kind.eq(kind))
        .one(db)
        .await?
    else {
        return Ok(None);
    };
    if row.used_at.is_some() {
        return Ok(None);
    }
    let Ok(expires) = chrono::DateTime::parse_from_rfc3339(&row.expires_at) else {
        return Ok(None);
    };
    if expires < chrono::Utc::now() {
        return Ok(None);
    }
    let user_id = row.user_id.clone();
    let mut update: email_tokens::ActiveModel = row.into();
    update.used_at = Set(Some(now_rfc3339()));
    update.update(db).await?;
    Ok(Some(user_id))
}

/// Device/IP context attached to a session at creation, and shown back to
/// the user in the "active sessions" list.
#[derive(Clone, Default)]
pub struct SessionContext {
    pub ip: Option<String>,
    pub user_agent: Option<String>,
}

pub async fn create_session(
    db: &DatabaseConnection,
    user_id: &str,
    ctx: &SessionContext,
) -> anyhow::Result<String> {
    let token = random_secret();
    let id = sha256_hex(&token);
    let now = now_rfc3339();
    let expires = chrono::Utc::now() + chrono::Duration::days(SESSION_DAYS);
    sessions::ActiveModel {
        id: Set(id.clone()),
        user_id: Set(user_id.to_string()),
        created_at: Set(now.clone()),
        expires_at: Set(expires.to_rfc3339()),
        user_agent: Set(ctx.user_agent.clone()),
        ip: Set(ctx.ip.clone()),
        last_seen_at: Set(now),
    }
    .insert(db)
    .await?;

    let rows = sessions::Entity::find()
        .filter(sessions::Column::UserId.eq(user_id))
        .order_by_desc(sessions::Column::CreatedAt)
        .all(db)
        .await?;
    for stale in rows.iter().skip(MAX_SESSIONS_PER_USER) {
        sessions::Entity::delete_by_id(&stale.id).exec(db).await?;
    }

    Ok(token)
}

pub async fn delete_session(db: &DatabaseConnection, session_token: &str) -> anyhow::Result<()> {
    sessions::Entity::delete_by_id(sha256_hex(session_token))
        .exec(db)
        .await?;
    Ok(())
}

pub async fn delete_all_sessions(db: &DatabaseConnection, user_id: &str) -> anyhow::Result<()> {
    sessions::Entity::delete_many()
        .filter(sessions::Column::UserId.eq(user_id))
        .exec(db)
        .await?;
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

// --- TOTP / backup codes ---

/// A fresh base32-encoded TOTP secret, ready to store on the user row.
pub fn generate_totp_secret() -> String {
    Secret::generate_secret().to_encoded().to_string()
}

fn totp_for(secret: &str, account_name: &str) -> Option<TOTP> {
    let bytes = Secret::Encoded(secret.to_string()).to_bytes().ok()?;
    TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        bytes,
        Some(MFA_ISSUER.to_string()),
        account_name.to_string(),
    )
    .ok()
}

pub fn totp_otpauth_url(secret: &str, account_name: &str) -> Option<String> {
    Some(totp_for(secret, account_name)?.get_url())
}

pub fn totp_qr_png_base64(secret: &str, account_name: &str) -> Option<String> {
    totp_for(secret, account_name)?.get_qr_base64().ok()
}

pub fn verify_totp_code(secret: &str, code: &str) -> bool {
    let Some(totp) = totp_for(secret, "") else {
        return false;
    };
    totp.check_current(code).unwrap_or(false)
}

/// Plaintext backup codes to show the user once; only their SHA-256 hash is
/// persisted.
pub fn generate_backup_codes() -> Vec<String> {
    (0..MFA_BACKUP_CODE_COUNT)
        .map(|_| {
            let mut bytes = [0u8; 5];
            OsRng.fill_bytes(&mut bytes);
            to_hex(&bytes)
        })
        .collect()
}

pub async fn store_backup_codes(
    db: &DatabaseConnection,
    user_id: &str,
    codes: &[String],
) -> anyhow::Result<()> {
    mfa_backup_codes::Entity::delete_many()
        .filter(mfa_backup_codes::Column::UserId.eq(user_id))
        .exec(db)
        .await?;
    for code in codes {
        mfa_backup_codes::ActiveModel {
            id: Set(Uuid::new_v4().to_string()),
            user_id: Set(user_id.to_string()),
            code_hash: Set(sha256_hex(code)),
            used_at: Set(None),
            created_at: Set(now_rfc3339()),
        }
        .insert(db)
        .await?;
    }
    Ok(())
}

pub async fn clear_backup_codes(db: &DatabaseConnection, user_id: &str) -> anyhow::Result<()> {
    mfa_backup_codes::Entity::delete_many()
        .filter(mfa_backup_codes::Column::UserId.eq(user_id))
        .exec(db)
        .await?;
    Ok(())
}

/// Checks an unused backup code and consumes it if valid.
pub async fn consume_backup_code(
    db: &DatabaseConnection,
    user_id: &str,
    code: &str,
) -> anyhow::Result<bool> {
    let hash = sha256_hex(&code.trim().to_ascii_lowercase());
    let Some(row) = mfa_backup_codes::Entity::find()
        .filter(mfa_backup_codes::Column::UserId.eq(user_id))
        .filter(mfa_backup_codes::Column::CodeHash.eq(&hash))
        .filter(mfa_backup_codes::Column::UsedAt.is_null())
        .one(db)
        .await?
    else {
        return Ok(false);
    };
    let mut update: mfa_backup_codes::ActiveModel = row.into();
    update.used_at = Set(Some(now_rfc3339()));
    update.update(db).await?;
    Ok(true)
}

/// A short-lived (5 min) marker issued after the password check succeeds
/// for an MFA-enabled account, consumed by the second `/auth/login/mfa`
/// step. Mirrors `create_email_token`/`consume_email_token`.
pub async fn create_mfa_challenge(
    db: &DatabaseConnection,
    user_id: &str,
) -> anyhow::Result<String> {
    let token = random_secret();
    let expires = chrono::Utc::now() + chrono::Duration::minutes(MFA_CHALLENGE_MINUTES);
    mfa_challenges::ActiveModel {
        id: Set(sha256_hex(&token)),
        user_id: Set(user_id.to_string()),
        expires_at: Set(expires.to_rfc3339()),
        created_at: Set(now_rfc3339()),
    }
    .insert(db)
    .await?;
    Ok(token)
}

/// Validates a pending-2FA token *without* consuming it — a mistyped code
/// shouldn't burn the login attempt, only a successful one should. Callers
/// must explicitly [`delete_mfa_challenge`] once the code check passes.
pub async fn peek_mfa_challenge(
    db: &DatabaseConnection,
    token: &str,
) -> anyhow::Result<Option<String>> {
    let id = sha256_hex(token);
    let Some(row) = mfa_challenges::Entity::find_by_id(&id).one(db).await? else {
        return Ok(None);
    };
    let Ok(expires) = chrono::DateTime::parse_from_rfc3339(&row.expires_at) else {
        return Ok(None);
    };
    if expires < chrono::Utc::now() {
        let _ = mfa_challenges::Entity::delete_by_id(&id).exec(db).await;
        return Ok(None);
    }
    Ok(Some(row.user_id))
}

pub async fn delete_mfa_challenge(db: &DatabaseConnection, token: &str) -> anyhow::Result<()> {
    mfa_challenges::Entity::delete_by_id(sha256_hex(token))
        .exec(db)
        .await?;
    Ok(())
}

// --- WebAuthn / passkey challenge state ---

/// Persists WebAuthn ceremony state (a `PasskeyRegistration` or
/// `PasskeyAuthentication`, already serialized to JSON by the caller)
/// between the `start` and `finish` steps. Mirrors the MFA challenge
/// pattern; `user_id` is `None` for login challenges since identity isn't
/// confirmed until the credential itself verifies.
pub async fn create_webauthn_challenge(
    db: &DatabaseConnection,
    user_id: Option<&str>,
    kind: &str,
    state_json: String,
) -> anyhow::Result<String> {
    let token = random_secret();
    let expires = chrono::Utc::now() + chrono::Duration::minutes(WEBAUTHN_CHALLENGE_MINUTES);
    webauthn_challenges::ActiveModel {
        id: Set(sha256_hex(&token)),
        user_id: Set(user_id.map(str::to_string)),
        kind: Set(kind.to_string()),
        state_json: Set(state_json),
        expires_at: Set(expires.to_rfc3339()),
        created_at: Set(now_rfc3339()),
    }
    .insert(db)
    .await?;
    Ok(token)
}

/// Consumes (always deletes) a WebAuthn challenge — unlike MFA codes, a
/// WebAuthn ceremony can't be "retried" against the same challenge since
/// the browser/authenticator already consumed its nonce, so there's no
/// mistyped-code case to protect against.
pub async fn consume_webauthn_challenge(
    db: &DatabaseConnection,
    token: &str,
    kind: &str,
) -> anyhow::Result<Option<(Option<String>, String)>> {
    let id = sha256_hex(token);
    let Some(row) = webauthn_challenges::Entity::find_by_id(&id).one(db).await? else {
        return Ok(None);
    };
    webauthn_challenges::Entity::delete_by_id(&id)
        .exec(db)
        .await?;
    if row.kind != kind {
        return Ok(None);
    }
    let Ok(expires) = chrono::DateTime::parse_from_rfc3339(&row.expires_at) else {
        return Ok(None);
    };
    if expires < chrono::Utc::now() {
        return Ok(None);
    }
    Ok(Some((row.user_id, row.state_json)))
}

// --- Audit log ---

/// Records a security-relevant event. Fire-and-forget: a failure to write
/// the audit entry must never block or fail the action it's recording.
pub async fn audit(
    db: &DatabaseConnection,
    user_id: &str,
    event: &str,
    ip: Option<&str>,
    user_agent: Option<&str>,
    metadata: Option<&str>,
) {
    let result = audit_log::ActiveModel {
        id: Set(Uuid::new_v4().to_string()),
        user_id: Set(user_id.to_string()),
        event: Set(event.to_string()),
        ip: Set(ip.map(str::to_string)),
        user_agent: Set(user_agent.map(str::to_string)),
        metadata: Set(metadata.map(str::to_string)),
        created_at: Set(now_rfc3339()),
    }
    .insert(db)
    .await;
    if let Err(e) = result {
        log::error!("failed to write audit log entry ({event}) for {user_id}: {e}");
    }
}

// --- Breached-password check (HaveIBeenPwned Pwned Passwords, k-anonymity) ---

/// Returns `true` if the password appears in the Pwned Passwords corpus.
/// **Fails open**: any network/parse error is treated as "not found" and
/// logged, so a third-party outage can never block registration or a
/// password reset.
pub async fn is_breached_password(client: &reqwest::Client, password: &str) -> bool {
    let digest = Sha1::digest(password.as_bytes());
    let hex = to_hex_upper(&digest);
    let (prefix, suffix) = hex.split_at(5);

    let url = format!("https://api.pwnedpasswords.com/range/{prefix}");
    let resp = match client.get(&url).send().await {
        Ok(r) => r,
        Err(e) => {
            log::warn!("pwned-passwords check unavailable, failing open: {e}");
            return false;
        }
    };
    let body = match resp.text().await {
        Ok(b) => b,
        Err(e) => {
            log::warn!("pwned-passwords response unreadable, failing open: {e}");
            return false;
        }
    };
    body.lines().any(|line| {
        line.split_once(':')
            .is_some_and(|(candidate, _count)| candidate.eq_ignore_ascii_case(suffix))
    })
}

fn to_hex_upper(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02X}")).collect()
}

async fn user_for_session(db: &DatabaseConnection, session_token: &str) -> Option<users::Model> {
    let session_id = sha256_hex(session_token);
    let session = sessions::Entity::find_by_id(&session_id)
        .one(db)
        .await
        .ok()??;
    let expires = chrono::DateTime::parse_from_rfc3339(&session.expires_at).ok()?;
    if expires < chrono::Utc::now() {
        let _ = sessions::Entity::delete_by_id(&session_id).exec(db).await;
        return None;
    }
    touch_session(db, &session_id, &session.last_seen_at).await;
    users::Entity::find_by_id(&session.user_id)
        .one(db)
        .await
        .ok()?
}

/// Best-effort, rate-limited `last_seen_at` bump — skips the write unless
/// the session hasn't been touched in a while.
async fn touch_session(db: &DatabaseConnection, session_id: &str, last_seen_at: &str) {
    let stale = chrono::DateTime::parse_from_rfc3339(last_seen_at)
        .map(|t| {
            chrono::Utc::now().signed_duration_since(t)
                > chrono::Duration::seconds(SESSION_TOUCH_INTERVAL_SECS)
        })
        .unwrap_or(true);
    if !stale {
        return;
    }
    let update = sessions::ActiveModel {
        id: Set(session_id.to_string()),
        last_seen_at: Set(now_rfc3339()),
        ..Default::default()
    };
    let _ = update.update(db).await;
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
        let Some(state) = req.rocket().state::<PortState>() else {
            return Outcome::Error((Status::InternalServerError, ()));
        };
        // X-Api-Key is an explicit, deliberate credential (never sent
        // ambiently by a browser), so it's checked first and is exempt from
        // CSRF — a request that presents it authenticates via the token,
        // full stop, even if a session cookie also happens to be present.
        if let Some(token) = req.headers().get_one("X-Api-Key")
            && let Some(user) = user_for_token(&state.db, token).await
        {
            return Outcome::Success(user.into());
        }
        if let Some(cookie) = req.cookies().get(SESSION_COOKIE)
            && let Some(user) = user_for_session(&state.db, cookie.value()).await
        {
            // CSRF: cookie-based sessions are ambient (the browser attaches
            // them automatically), so state-changing requests must also
            // prove they can read the non-HttpOnly CSRF cookie — a
            // cross-site form/script can trigger the request but can't read
            // it.
            if is_unsafe_method(req.method()) && !csrf_ok(req) {
                return Outcome::Error((Status::Forbidden, ()));
            }
            return Outcome::Success(user.into());
        }
        Outcome::Error((Status::Unauthorized, ()))
    }
}

fn is_unsafe_method(method: Method) -> bool {
    matches!(method, Method::Post | Method::Put | Method::Patch | Method::Delete)
}

fn csrf_ok(req: &Request<'_>) -> bool {
    let Some(cookie) = req.cookies().get(CSRF_COOKIE) else {
        return false;
    };
    let Some(header) = req.headers().get_one(CSRF_HEADER) else {
        return false;
    };
    constant_time_eq_str(cookie.value(), header)
}

fn constant_time_eq_str(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.bytes()
        .zip(b.bytes())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}

/// Like [`AuthUser`], but additionally requires a verified email —
/// grandfathering in legacy accounts that predate email (no email on file
/// at all) so they aren't locked out until they add one. Use this to gate
/// actions that create public content (uploads, comments, etc.).
pub struct VerifiedUser(pub AuthUser);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for VerifiedUser {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, ()> {
        let Some(state) = req.rocket().state::<PortState>() else {
            return Outcome::Error((Status::InternalServerError, ()));
        };
        let user = match AuthUser::from_request(req).await {
            Outcome::Success(u) => u,
            Outcome::Error(e) => return Outcome::Error(e),
            Outcome::Forward(f) => return Outcome::Forward(f),
        };
        let Some(model) = users::Entity::find_by_id(&user.id).one(&state.db).await.ok().flatten()
        else {
            return Outcome::Error((Status::Unauthorized, ()));
        };
        if model.email.is_some() && !model.email_verified {
            return Outcome::Error((Status::Forbidden, ()));
        }
        Outcome::Success(VerifiedUser(user))
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

/// `User-Agent` header, recorded on sessions for the "active sessions" list.
pub struct UserAgent(pub Option<String>);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for UserAgent {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, ()> {
        Outcome::Success(UserAgent(
            req.headers().get_one("User-Agent").map(str::to_string),
        ))
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

    pub fn reset(&self, bucket: &str, key: &str) {
        let mut map = self.windows.lock().unwrap_or_else(|e| e.into_inner());
        map.remove(&(bucket.to_string(), key.to_string()));
    }
}
