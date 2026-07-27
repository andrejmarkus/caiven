use std::time::Duration;

use rocket::{
    State, delete, get,
    http::{Cookie, CookieJar, SameSite, Status},
    post,
    response::Redirect,
    serde::json::Json,
    time::Duration as CookieDuration,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
    TransactionTrait,
};
use uuid::Uuid;
use webauthn_rs::prelude::Passkey;

use crate::{
    PortState,
    auth::{self, AuthUser, CSRF_COOKIE, ClientIp, SESSION_COOKIE, UserAgent},
    db,
    entities::{
        api_tokens, audit_log as audit_log_entity, carts, oauth_identities, sessions, users,
        webauthn_credentials,
    },
    error::ApiError,
    mailer,
    models::{
        AuditEntry, AuthConfigInfo, DeleteAccountInput, ForgotPasswordInput, LoginInput,
        LoginMfaInput, LoginOutcome, MfaConfirmInput, MfaConfirmed, MfaDisableInput, MfaSetupInfo,
        MfaStatus, PasskeyInfo, PasswordChange, RegisterInput, ResetPasswordInput, SessionInfo,
        SetPasswordInput, TokenCreate, TokenCreated, TokenInfo, UserInfo, VerifyEmailInput,
        WebauthnLoginFinishInput, WebauthnLoginStartInput, WebauthnRegisterFinishInput,
        WebauthnStartResponse,
    },
    oauth, turnstile,
};

const REGISTER_LIMIT: u32 = 5;
const REGISTER_WINDOW: Duration = Duration::from_secs(3600);
const LOGIN_FAIL_LIMIT: u32 = 10;
const LOGIN_FAIL_WINDOW: Duration = Duration::from_secs(15 * 60);
/// After this many failed attempts from an IP within the login-fail window,
/// a valid Turnstile token is required on top of the rate limit — keeps
/// login frictionless for humans while slowing down credential-stuffing.
const LOGIN_TURNSTILE_THRESHOLD: u32 = 3;
const RESEND_LIMIT: u32 = 3;
const RESEND_WINDOW: Duration = Duration::from_secs(3600);
const FORGOT_LIMIT: u32 = 5;
const FORGOT_WINDOW: Duration = Duration::from_secs(3600);

const OAUTH_STATE_COOKIE: &str = "caiven_oauth";
const OAUTH_PATH: &str = "/api/v2/auth/oauth";

fn normalize_username(username: &str) -> String {
    username.trim().to_ascii_lowercase()
}

fn validate_username(name: &str) -> Result<(), ApiError> {
    if name.len() < 3 || name.len() > 32 {
        return Err(ApiError::bad_request("username must be 3-32 chars"));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
    {
        return Err(ApiError::bad_request(
            "username may only contain a-z, 0-9, _ and -",
        ));
    }
    Ok(())
}

fn validate_password(password: &str) -> Result<(), ApiError> {
    let length = password.chars().count();
    if !(8..=128).contains(&length) {
        return Err(ApiError::bad_request("password must be 8-128 characters"));
    }
    if !password.chars().any(|c| c.is_uppercase()) {
        return Err(ApiError::bad_request(
            "password must contain at least one uppercase letter",
        ));
    }
    if !password
        .chars()
        .any(|c| !c.is_alphanumeric() && !c.is_whitespace())
    {
        return Err(ApiError::bad_request(
            "password must contain at least one special character",
        ));
    }
    Ok(())
}

/// Rejects a password found in the Pwned Passwords corpus. Called after
/// `validate_password` on every path that sets a password (register, reset,
/// set-password, change-password).
async fn reject_breached_password(state: &PortState, password: &str) -> Result<(), ApiError> {
    if auth::is_breached_password(&state.http, password).await {
        return Err(ApiError::bad_request(
            "this password has appeared in a known data breach — please choose a different one",
        ));
    }
    Ok(())
}

fn to_user_info(u: users::Model) -> UserInfo {
    UserInfo {
        id: u.id,
        username: u.username,
        is_admin: u.is_admin,
        email: u.email,
        email_verified: u.email_verified,
        password_set: u.password_set,
    }
}

fn link_for(state: &PortState, path: &str) -> String {
    format!("{}{path}", state.base_url.as_deref().unwrap_or(""))
}

async fn start_session(
    state: &PortState,
    jar: &CookieJar<'_>,
    user_id: &str,
    ctx: &auth::SessionContext,
) -> Result<(), ApiError> {
    if let Some(cookie) = jar.get(SESSION_COOKIE) {
        auth::delete_session(&state.db, cookie.value()).await?;
    }
    let sid = auth::create_session(&state.db, user_id, ctx).await?;
    jar.add(
        Cookie::build((SESSION_COOKIE, sid))
            .http_only(true)
            .same_site(SameSite::Lax)
            .secure(state.secure_cookies)
            .path("/")
            .max_age(CookieDuration::days(auth::SESSION_DAYS)),
    );
    // Double-submit CSRF cookie: JS-readable so it can be echoed back as a
    // header on mutating requests. See `auth::AuthUser::from_request`.
    jar.add(
        Cookie::build((CSRF_COOKIE, auth::random_secret()))
            .http_only(false)
            .same_site(SameSite::Lax)
            .secure(state.secure_cookies)
            .path("/")
            .max_age(CookieDuration::days(auth::SESSION_DAYS)),
    );
    Ok(())
}

fn session_ctx(ip: &ClientIp, ua: &UserAgent) -> auth::SessionContext {
    auth::SessionContext {
        ip: Some(ip.0.clone()),
        user_agent: ua.0.clone(),
    }
}

async fn alert_email(state: &PortState, user: &users::Model, subject: &str, body: &str) {
    if let Some(email) = &user.email {
        mailer::send_or_log_alert(state.mailer.as_ref(), email, subject, body).await;
    }
}

/// Sends the existing security-alert email *and* records a durable audit
/// log entry for the same event — the email is best-effort and ephemeral,
/// the audit entry is what backs `/auth/audit-log`.
#[allow(clippy::too_many_arguments)]
async fn notify(
    state: &PortState,
    user: &users::Model,
    event: &str,
    ip: Option<&str>,
    ua: Option<&str>,
    subject: &str,
    body: &str,
) {
    alert_email(state, user, subject, body).await;
    auth::audit(&state.db, &user.id, event, ip, ua, None).await;
}

#[get("/api/v2/auth/config")]
pub fn auth_config(state: &State<PortState>) -> Json<AuthConfigInfo> {
    Json(AuthConfigInfo {
        turnstile_site_key: state.turnstile_site_key.clone(),
        providers: state
            .oauth
            .enabled()
            .into_iter()
            .map(|p| p.as_str().to_string())
            .collect(),
    })
}

#[post("/api/v2/auth/register", data = "<input>")]
pub async fn register(
    state: &State<PortState>,
    ip: ClientIp,
    ua: UserAgent,
    jar: &CookieJar<'_>,
    input: Json<RegisterInput>,
) -> Result<Json<UserInfo>, ApiError> {
    if state.rate.hit("register", &ip.0, REGISTER_WINDOW) > REGISTER_LIMIT {
        return Err(ApiError::TooManyRequests("try again later".into()));
    }
    if !turnstile::verify(
        &state.http,
        state.turnstile_secret.as_deref(),
        input.turnstile_token.as_deref().unwrap_or(""),
        &ip.0,
    )
    .await
    {
        return Err(ApiError::bad_request("antibot check failed"));
    }

    let username = normalize_username(&input.username);
    validate_username(&username)?;
    validate_password(&input.password)?;
    reject_breached_password(state, &input.password).await?;
    let email = input.email.trim().to_string();
    if !auth::is_valid_email(&email) {
        return Err(ApiError::bad_request("invalid email address"));
    }
    let email_normalized = auth::normalize_email(&email);

    let existing = users::Entity::find()
        .filter(
            users::Column::Username
                .eq(&username)
                .or(users::Column::EmailNormalized.eq(&email_normalized)),
        )
        .one(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    if existing.is_some() {
        return Err(ApiError::conflict("username or email already in use"));
    }

    // First account on a fresh port becomes the admin.
    let user_count = users::Entity::find()
        .count(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;

    // Without SMTP configured there's no way for the user to click a link,
    // so local/self-hosted deployments auto-verify instead of locking every
    // account out of write actions forever.
    let auto_verified = state.mailer.is_none();

    let user = users::ActiveModel {
        id: Set(Uuid::new_v4().to_string()),
        username: Set(username),
        password_hash: Set(auth::hash_password_async(input.password.clone())
            .await
            .map_err(ApiError::internal)?),
        is_admin: Set(user_count == 0),
        created_at: Set(chrono::Utc::now().to_rfc3339()),
        email: Set(Some(email.clone())),
        email_verified: Set(auto_verified),
        email_normalized: Set(Some(email_normalized)),
        mfa_totp_secret: Set(None),
        mfa_enabled: Set(false),
        password_set: Set(true),
    }
    .insert(&state.db)
    .await
    .map_err(|_| ApiError::conflict("username or email already in use"))?;

    send_verification_email(state, &user).await;

    start_session(state, jar, &user.id, &session_ctx(&ip, &ua)).await?;
    Ok(Json(to_user_info(user)))
}

async fn send_verification_email(state: &PortState, user: &users::Model) {
    let Some(email) = &user.email else { return };
    let token = match auth::create_email_token(
        &state.db,
        &user.id,
        "verify",
        auth::EMAIL_VERIFY_TOKEN_HOURS,
    )
    .await
    {
        Ok(t) => t,
        Err(e) => {
            log::error!("failed to create verification token for {}: {e}", user.id);
            return;
        }
    };
    let link = link_for(state, &format!("/verify-email?token={token}"));
    mailer::send_or_log_verification(state.mailer.as_ref(), email, &link).await;
}

#[post("/api/v2/auth/login", data = "<input>")]
pub async fn login(
    state: &State<PortState>,
    ip: ClientIp,
    ua: UserAgent,
    jar: &CookieJar<'_>,
    input: Json<LoginInput>,
) -> Result<Json<LoginOutcome>, ApiError> {
    if state.rate.count("login_fail", &ip.0, LOGIN_FAIL_WINDOW) >= LOGIN_FAIL_LIMIT {
        return Err(ApiError::TooManyRequests(
            "too many failed logins, try again later".into(),
        ));
    }

    let identifier = input.identifier.trim().to_string();
    let login_key = format!("{}:{}", ip.0, identifier.to_ascii_lowercase());
    if state
        .rate
        .count("login_identity_fail", &login_key, LOGIN_FAIL_WINDOW)
        >= LOGIN_FAIL_LIMIT
    {
        return Err(ApiError::TooManyRequests(
            "too many failed logins, try again later".into(),
        ));
    }

    // Adaptive antibot: only demand Turnstile once this IP has racked up a
    // few failures, so normal logins stay frictionless.
    if state.rate.count("login_fail", &ip.0, LOGIN_FAIL_WINDOW) >= LOGIN_TURNSTILE_THRESHOLD
        && !turnstile::verify(
            &state.http,
            state.turnstile_secret.as_deref(),
            input.turnstile_token.as_deref().unwrap_or(""),
            &ip.0,
        )
        .await
    {
        return Err(ApiError::bad_request("antibot check failed"));
    }

    let username = normalize_username(&identifier);
    let email_normalized = auth::normalize_email(&identifier);
    let user = users::Entity::find()
        .filter(
            users::Column::Username
                .eq(&username)
                .or(users::Column::EmailNormalized.eq(&email_normalized)),
        )
        .one(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;

    let valid = auth::verify_login_password(
        input.password.clone(),
        user.as_ref().map(|u| u.password_hash.clone()),
    )
    .await;
    if !valid {
        state.rate.hit("login_fail", &ip.0, LOGIN_FAIL_WINDOW);
        state
            .rate
            .hit("login_identity_fail", &login_key, LOGIN_FAIL_WINDOW);
        return Err(ApiError::Unauthorized);
    }
    let user = user.expect("checked above");
    state.rate.reset("login_fail", &ip.0);
    state.rate.reset("login_identity_fail", &login_key);

    if user.mfa_enabled {
        let pending_token = auth::create_mfa_challenge(&state.db, &user.id).await?;
        return Ok(Json(LoginOutcome {
            mfa_required: true,
            pending_token: Some(pending_token),
            user: None,
        }));
    }

    start_session(state, jar, &user.id, &session_ctx(&ip, &ua)).await?;
    notify(
        state,
        &user,
        "login",
        Some(&ip.0),
        ua.0.as_deref(),
        "New sign-in to your Caiven account",
        &format!("Your account was just signed in from IP {}.\n\nIf this wasn't you, change your password immediately from Settings.", ip.0),
    )
    .await;
    Ok(Json(LoginOutcome {
        mfa_required: false,
        pending_token: None,
        user: Some(to_user_info(user)),
    }))
}

#[post("/api/v2/auth/login/mfa", data = "<input>")]
pub async fn login_mfa(
    state: &State<PortState>,
    ip: ClientIp,
    ua: UserAgent,
    jar: &CookieJar<'_>,
    input: Json<LoginMfaInput>,
) -> Result<Json<UserInfo>, ApiError> {
    if state.rate.hit("mfa_login", &ip.0, LOGIN_FAIL_WINDOW) > LOGIN_FAIL_LIMIT {
        return Err(ApiError::TooManyRequests("try again later".into()));
    }
    let user_id = auth::peek_mfa_challenge(&state.db, &input.pending_token)
        .await?
        .ok_or_else(|| ApiError::bad_request("invalid or expired login"))?;
    let user = users::Entity::find_by_id(&user_id)
        .one(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?
        .ok_or(ApiError::Unauthorized)?;

    let code = input.code.trim();
    let totp_ok = user
        .mfa_totp_secret
        .as_deref()
        .is_some_and(|secret| auth::verify_totp_code(secret, code));
    let ok = totp_ok || auth::consume_backup_code(&state.db, &user.id, code).await?;
    if !ok {
        // A mistyped code doesn't burn the pending login — only a
        // successful check consumes the challenge, below.
        return Err(ApiError::Unauthorized);
    }
    auth::delete_mfa_challenge(&state.db, &input.pending_token).await?;

    start_session(state, jar, &user.id, &session_ctx(&ip, &ua)).await?;
    notify(
        state,
        &user,
        "login_mfa",
        Some(&ip.0),
        ua.0.as_deref(),
        "New sign-in to your Caiven account",
        &format!("Your account was just signed in from IP {}.\n\nIf this wasn't you, change your password immediately from Settings.", ip.0),
    )
    .await;
    Ok(Json(to_user_info(user)))
}

#[post("/api/v2/auth/logout")]
pub async fn logout(state: &State<PortState>, jar: &CookieJar<'_>) -> Result<(), ApiError> {
    if let Some(cookie) = jar.get(SESSION_COOKIE) {
        auth::delete_session(&state.db, cookie.value()).await?;
        jar.remove(Cookie::build(SESSION_COOKIE).path("/"));
    }
    jar.remove(Cookie::build(CSRF_COOKIE).path("/"));
    Ok(())
}

#[get("/api/v2/auth/me")]
pub async fn me(user: AuthUser, state: &State<PortState>) -> Result<Json<UserInfo>, ApiError> {
    let model = users::Entity::find_by_id(&user.id)
        .one(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?
        .ok_or(ApiError::Unauthorized)?;
    Ok(Json(to_user_info(model)))
}

#[post("/api/v2/auth/verify-email", data = "<input>")]
pub async fn verify_email(
    state: &State<PortState>,
    input: Json<VerifyEmailInput>,
) -> Result<Status, ApiError> {
    let user_id = auth::consume_email_token(&state.db, &input.token, "verify")
        .await?
        .ok_or_else(|| ApiError::bad_request("invalid or expired token"))?;
    let model = users::Entity::find_by_id(&user_id)
        .one(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?
        .ok_or_else(|| ApiError::not_found("account not found"))?;
    let mut update: users::ActiveModel = model.into();
    update.email_verified = Set(true);
    update
        .update(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(Status::NoContent)
}

#[post("/api/v2/auth/resend-verification")]
pub async fn resend_verification(
    state: &State<PortState>,
    ip: ClientIp,
    user: AuthUser,
) -> Result<Status, ApiError> {
    if state.rate.hit("resend_verify", &ip.0, RESEND_WINDOW) > RESEND_LIMIT {
        return Err(ApiError::TooManyRequests("try again later".into()));
    }
    let model = users::Entity::find_by_id(&user.id)
        .one(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?
        .ok_or(ApiError::Unauthorized)?;
    if model.email.is_none() {
        return Err(ApiError::bad_request("no email on file"));
    }
    if model.email_verified {
        return Ok(Status::NoContent);
    }
    send_verification_email(state, &model).await;
    Ok(Status::NoContent)
}

#[post("/api/v2/auth/forgot-password", data = "<input>")]
pub async fn forgot_password(
    state: &State<PortState>,
    ip: ClientIp,
    input: Json<ForgotPasswordInput>,
) -> Result<Status, ApiError> {
    if state.rate.hit("forgot_password", &ip.0, FORGOT_WINDOW) > FORGOT_LIMIT {
        return Err(ApiError::TooManyRequests("try again later".into()));
    }
    if !turnstile::verify(
        &state.http,
        state.turnstile_secret.as_deref(),
        input.turnstile_token.as_deref().unwrap_or(""),
        &ip.0,
    )
    .await
    {
        return Err(ApiError::bad_request("antibot check failed"));
    }

    let email_normalized = auth::normalize_email(&input.email);
    // Always return 204 regardless of whether the account exists, so the
    // endpoint can't be used to enumerate registered emails.
    if let Some(user) = users::Entity::find()
        .filter(users::Column::EmailNormalized.eq(&email_normalized))
        .one(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?
        && let Some(email) = &user.email
    {
        let token = auth::create_email_token(
            &state.db,
            &user.id,
            "reset",
            auth::PASSWORD_RESET_TOKEN_HOURS,
        )
        .await?;
        let link = link_for(state, &format!("/reset-password?token={token}"));
        mailer::send_or_log_reset(state.mailer.as_ref(), email, &link).await;
    }
    Ok(Status::NoContent)
}

#[post("/api/v2/auth/reset-password", data = "<input>")]
pub async fn reset_password(
    state: &State<PortState>,
    input: Json<ResetPasswordInput>,
) -> Result<Status, ApiError> {
    validate_password(&input.new_password)?;
    reject_breached_password(state, &input.new_password).await?;
    let user_id = auth::consume_email_token(&state.db, &input.token, "reset")
        .await?
        .ok_or_else(|| ApiError::bad_request("invalid or expired token"))?;
    let model = users::Entity::find_by_id(&user_id)
        .one(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?
        .ok_or_else(|| ApiError::not_found("account not found"))?;

    let mut update: users::ActiveModel = model.into();
    update.password_hash = Set(auth::hash_password_async(input.new_password.clone())
        .await
        .map_err(ApiError::internal)?);
    update
        .update(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;

    auth::delete_all_sessions(&state.db, &user_id).await?;
    Ok(Status::NoContent)
}

#[post("/api/v2/auth/password", data = "<req>")]
pub async fn change_password(
    state: &State<PortState>,
    ip: ClientIp,
    ua: UserAgent,
    jar: &CookieJar<'_>,
    user: AuthUser,
    req: Json<PasswordChange>,
) -> Result<Status, ApiError> {
    validate_password(&req.new_password)?;
    reject_breached_password(state, &req.new_password).await?;
    let model = users::Entity::find_by_id(&user.id)
        .one(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?
        .ok_or(ApiError::Unauthorized)?;
    if !model.password_set {
        return Err(ApiError::bad_request(
            "this account has no password yet — use /auth/set-password",
        ));
    }

    if !auth::verify_login_password(
        req.current_password.clone(),
        Some(model.password_hash.clone()),
    )
    .await
    {
        return Err(ApiError::Unauthorized);
    }
    if auth::verify_login_password(req.new_password.clone(), Some(model.password_hash.clone()))
        .await
    {
        return Err(ApiError::bad_request(
            "new password must differ from current password",
        ));
    }

    let mut update: users::ActiveModel = model.clone().into();
    update.password_hash = Set(auth::hash_password_async(req.new_password.clone())
        .await
        .map_err(ApiError::internal)?);
    update
        .update(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;

    auth::delete_all_sessions(&state.db, &user.id).await?;
    start_session(state, jar, &user.id, &session_ctx(&ip, &ua)).await?;
    notify(
        state,
        &model,
        "password_changed",
        Some(&ip.0),
        ua.0.as_deref(),
        "Your Caiven password was changed",
        "Your password was just changed and all other sessions were signed out. If this wasn't you, contact support immediately.",
    )
    .await;
    Ok(Status::NoContent)
}

#[post("/api/v2/auth/set-password", data = "<input>")]
pub async fn set_password(
    state: &State<PortState>,
    ip: ClientIp,
    ua: UserAgent,
    user: AuthUser,
    input: Json<SetPasswordInput>,
) -> Result<Status, ApiError> {
    validate_password(&input.new_password)?;
    reject_breached_password(state, &input.new_password).await?;
    let model = users::Entity::find_by_id(&user.id)
        .one(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?
        .ok_or(ApiError::Unauthorized)?;
    if model.password_set {
        return Err(ApiError::bad_request(
            "password already set — use /auth/password to change it",
        ));
    }

    let mut update: users::ActiveModel = model.clone().into();
    update.password_hash = Set(auth::hash_password_async(input.new_password.clone())
        .await
        .map_err(ApiError::internal)?);
    update.password_set = Set(true);
    update
        .update(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;

    notify(
        state,
        &model,
        "password_set",
        Some(&ip.0),
        ua.0.as_deref(),
        "A password was set on your Caiven account",
        "A password was just added to your account, so you can now log in with it in addition to any social login you use.",
    )
    .await;
    Ok(Status::NoContent)
}

#[get("/api/v2/auth/sessions")]
pub async fn list_sessions(
    state: &State<PortState>,
    jar: &CookieJar<'_>,
    user: AuthUser,
) -> Result<Json<Vec<SessionInfo>>, ApiError> {
    let current_id = jar
        .get(SESSION_COOKIE)
        .map(|cookie| auth::sha256_hex(cookie.value()));
    let now = chrono::Utc::now();
    let rows = sessions::Entity::find()
        .filter(sessions::Column::UserId.eq(&user.id))
        .order_by_desc(sessions::Column::CreatedAt)
        .all(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(Json(
        rows.into_iter()
            .filter(|session| {
                chrono::DateTime::parse_from_rfc3339(&session.expires_at)
                    .is_ok_and(|expires| expires > now)
            })
            .map(|session| SessionInfo {
                current: current_id.as_deref() == Some(session.id.as_str()),
                id: session.id,
                created_at: session.created_at,
                expires_at: session.expires_at,
                last_seen_at: session.last_seen_at,
                ip: session.ip,
                user_agent: session.user_agent,
            })
            .collect(),
    ))
}

#[delete("/api/v2/auth/sessions/<session_id>")]
pub async fn revoke_session(
    state: &State<PortState>,
    jar: &CookieJar<'_>,
    user: AuthUser,
    session_id: &str,
) -> Result<Status, ApiError> {
    let result = sessions::Entity::delete_many()
        .filter(sessions::Column::Id.eq(session_id))
        .filter(sessions::Column::UserId.eq(&user.id))
        .exec(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    if result.rows_affected == 0 {
        return Err(ApiError::not_found("session not found"));
    }
    if jar
        .get(SESSION_COOKIE)
        .is_some_and(|cookie| auth::sha256_hex(cookie.value()) == session_id)
    {
        jar.remove(Cookie::build(SESSION_COOKIE).path("/"));
    }
    Ok(Status::NoContent)
}

#[delete("/api/v2/auth/sessions")]
pub async fn revoke_all_sessions(
    state: &State<PortState>,
    ip: ClientIp,
    ua: UserAgent,
    jar: &CookieJar<'_>,
    user: AuthUser,
) -> Result<Status, ApiError> {
    auth::delete_all_sessions(&state.db, &user.id).await?;
    jar.remove(Cookie::build(SESSION_COOKIE).path("/"));
    jar.remove(Cookie::build(CSRF_COOKIE).path("/"));
    if let Some(model) = users::Entity::find_by_id(&user.id)
        .one(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?
    {
        notify(
            state,
            &model,
            "sessions_revoked_all",
            Some(&ip.0),
            ua.0.as_deref(),
            "All Caiven sessions were signed out",
            "Every browser session on your account was just signed out. If you didn't do this, change your password immediately.",
        )
        .await;
    }
    Ok(Status::NoContent)
}

#[get("/api/v2/auth/tokens")]
pub async fn list_tokens(
    state: &State<PortState>,
    user: AuthUser,
) -> Result<Json<Vec<TokenInfo>>, ApiError> {
    let rows = api_tokens::Entity::find()
        .filter(api_tokens::Column::UserId.eq(&user.id))
        .order_by_desc(api_tokens::Column::CreatedAt)
        .all(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(Json(
        rows.into_iter()
            .map(|t| TokenInfo {
                id: t.id,
                name: t.name,
                created_at: t.created_at,
                last_used_at: t.last_used_at,
            })
            .collect(),
    ))
}

#[post("/api/v2/auth/tokens", data = "<req>")]
pub async fn create_token(
    state: &State<PortState>,
    user: AuthUser,
    req: Json<TokenCreate>,
) -> Result<Json<TokenCreated>, ApiError> {
    if req.name.len() > 64 {
        return Err(ApiError::bad_request("name max 64 chars"));
    }
    let (id, token) = auth::create_token(&state.db, &user.id, &req.name).await?;
    Ok(Json(TokenCreated {
        id,
        name: req.name.clone(),
        token,
    }))
}

#[delete("/api/v2/auth/tokens/<token_id>")]
pub async fn revoke_token(
    state: &State<PortState>,
    user: AuthUser,
    token_id: &str,
) -> Result<(), ApiError> {
    let res = api_tokens::Entity::delete_many()
        .filter(api_tokens::Column::Id.eq(token_id))
        .filter(api_tokens::Column::UserId.eq(&user.id))
        .exec(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    if res.rows_affected == 0 {
        return Err(ApiError::not_found("token not found"));
    }
    Ok(())
}

// --- Two-factor authentication (TOTP + backup codes) ---

#[get("/api/v2/auth/mfa/status")]
pub async fn mfa_status(
    state: &State<PortState>,
    user: AuthUser,
) -> Result<Json<MfaStatus>, ApiError> {
    let model = users::Entity::find_by_id(&user.id)
        .one(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?
        .ok_or(ApiError::Unauthorized)?;
    Ok(Json(MfaStatus {
        enabled: model.mfa_enabled,
    }))
}

#[post("/api/v2/auth/mfa/setup")]
pub async fn mfa_setup(
    state: &State<PortState>,
    user: AuthUser,
) -> Result<Json<MfaSetupInfo>, ApiError> {
    let model = users::Entity::find_by_id(&user.id)
        .one(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?
        .ok_or(ApiError::Unauthorized)?;
    if model.mfa_enabled {
        return Err(ApiError::conflict(
            "two-factor authentication is already enabled",
        ));
    }

    // Generating a fresh secret here (rather than reusing any prior pending
    // one) means an abandoned setup attempt can't linger indefinitely.
    let secret = auth::generate_totp_secret();
    let mut update: users::ActiveModel = model.clone().into();
    update.mfa_totp_secret = Set(Some(secret.clone()));
    update
        .update(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;

    let account_name = model.email.clone().unwrap_or(model.username);
    let otpauth_url = auth::totp_otpauth_url(&secret, &account_name)
        .ok_or_else(|| ApiError::internal("failed to build otpauth url"))?;
    let qr_png_base64 = auth::totp_qr_png_base64(&secret, &account_name)
        .ok_or_else(|| ApiError::internal("failed to render qr code"))?;

    Ok(Json(MfaSetupInfo {
        secret,
        otpauth_url,
        qr_png_base64,
    }))
}

#[post("/api/v2/auth/mfa/confirm", data = "<input>")]
pub async fn mfa_confirm(
    state: &State<PortState>,
    ip: ClientIp,
    ua: UserAgent,
    user: AuthUser,
    input: Json<MfaConfirmInput>,
) -> Result<Json<MfaConfirmed>, ApiError> {
    let model = users::Entity::find_by_id(&user.id)
        .one(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?
        .ok_or(ApiError::Unauthorized)?;
    let secret = model
        .mfa_totp_secret
        .clone()
        .ok_or_else(|| ApiError::bad_request("call /auth/mfa/setup first"))?;
    if !auth::verify_totp_code(&secret, input.code.trim()) {
        return Err(ApiError::bad_request("invalid code"));
    }

    let mut update: users::ActiveModel = model.clone().into();
    update.mfa_enabled = Set(true);
    update
        .update(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;

    let backup_codes = auth::generate_backup_codes();
    auth::store_backup_codes(&state.db, &user.id, &backup_codes).await?;

    notify(
        state,
        &model,
        "mfa_enabled",
        Some(&ip.0),
        ua.0.as_deref(),
        "Two-factor authentication enabled",
        "Two-factor authentication was just turned on for your Caiven account.",
    )
    .await;

    Ok(Json(MfaConfirmed { backup_codes }))
}

#[post("/api/v2/auth/mfa/disable", data = "<input>")]
pub async fn mfa_disable(
    state: &State<PortState>,
    ip: ClientIp,
    ua: UserAgent,
    user: AuthUser,
    input: Json<MfaDisableInput>,
) -> Result<Status, ApiError> {
    let model = users::Entity::find_by_id(&user.id)
        .one(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?
        .ok_or(ApiError::Unauthorized)?;
    if !auth::verify_login_password(
        input.current_password.clone(),
        Some(model.password_hash.clone()),
    )
    .await
    {
        return Err(ApiError::Unauthorized);
    }
    let secret = model
        .mfa_totp_secret
        .clone()
        .ok_or_else(|| ApiError::bad_request("two-factor authentication is not enabled"))?;
    let code = input.code.trim();
    let ok = auth::verify_totp_code(&secret, code)
        || auth::consume_backup_code(&state.db, &user.id, code).await?;
    if !ok {
        return Err(ApiError::Unauthorized);
    }

    let mut update: users::ActiveModel = model.clone().into();
    update.mfa_enabled = Set(false);
    update.mfa_totp_secret = Set(None);
    update
        .update(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    auth::clear_backup_codes(&state.db, &user.id).await?;

    notify(
        state,
        &model,
        "mfa_disabled",
        Some(&ip.0),
        ua.0.as_deref(),
        "Two-factor authentication disabled",
        "Two-factor authentication was just turned off for your Caiven account. If this wasn't you, secure your account immediately.",
    )
    .await;

    Ok(Status::NoContent)
}

// --- OAuth (Google / GitHub / Discord) ---

fn oauth_redirect_uri(state: &PortState, provider: oauth::Provider) -> String {
    link_for(
        state,
        &format!("{OAUTH_PATH}/{}/callback", provider.as_str()),
    )
}

#[get("/api/v2/auth/oauth/<provider>/start")]
pub async fn oauth_start(
    state: &State<PortState>,
    jar: &CookieJar<'_>,
    provider: &str,
) -> Result<Redirect, ApiError> {
    let Some(provider) = oauth::Provider::parse(provider) else {
        return Err(ApiError::not_found("unknown provider"));
    };
    let Some(cfg) = state.oauth.get(provider) else {
        return Err(ApiError::bad_request("provider not enabled"));
    };
    if state.base_url.is_none() {
        return Err(ApiError::internal(
            "CAIVEN_BASE_URL must be set to use OAuth login",
        ));
    }

    let csrf_state = auth::random_secret();
    let verifier = oauth::new_code_verifier();
    let challenge = oauth::code_challenge_s256(&verifier);

    jar.add(
        Cookie::build((
            OAUTH_STATE_COOKIE,
            format!("{}:{csrf_state}:{verifier}", provider.as_str()),
        ))
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(state.secure_cookies)
        .path(OAUTH_PATH)
        .max_age(CookieDuration::minutes(10)),
    );

    let redirect_uri = oauth_redirect_uri(state, provider);
    let url = oauth::build_authorize_url(provider, cfg, &redirect_uri, &csrf_state, &challenge);
    Ok(Redirect::to(url))
}

#[get("/api/v2/auth/oauth/<provider>/callback?<code>&<state_param>&<error>")]
#[allow(clippy::too_many_arguments)]
pub async fn oauth_callback(
    state: &State<PortState>,
    ip: ClientIp,
    ua: UserAgent,
    jar: &CookieJar<'_>,
    provider: &str,
    code: Option<String>,
    state_param: Option<String>,
    error: Option<String>,
) -> Redirect {
    match oauth_callback_inner(state, jar, provider, code, state_param, error).await {
        Ok(user_id) => {
            if start_session(state, jar, &user_id, &session_ctx(&ip, &ua))
                .await
                .is_err()
            {
                return Redirect::to("/login?error=oauth_failed");
            }
            Redirect::to("/")
        }
        Err(_) => Redirect::to("/login?error=oauth_failed"),
    }
}

async fn oauth_callback_inner(
    state: &PortState,
    jar: &CookieJar<'_>,
    provider: &str,
    code: Option<String>,
    state_param: Option<String>,
    error: Option<String>,
) -> Result<String, ApiError> {
    if error.is_some() {
        return Err(ApiError::bad_request("provider denied access"));
    }
    let Some(provider) = oauth::Provider::parse(provider) else {
        return Err(ApiError::not_found("unknown provider"));
    };
    let cfg = state
        .oauth
        .get(provider)
        .ok_or_else(|| ApiError::bad_request("provider not enabled"))?;
    let code = code.ok_or_else(|| ApiError::bad_request("missing code"))?;
    let state_param = state_param.ok_or_else(|| ApiError::bad_request("missing state"))?;

    let cookie = jar
        .get(OAUTH_STATE_COOKIE)
        .ok_or_else(|| ApiError::bad_request("missing oauth cookie"))?;
    let parts: Vec<&str> = cookie.value().splitn(3, ':').collect();
    jar.remove(Cookie::build(OAUTH_STATE_COOKIE).path(OAUTH_PATH));
    let [cookie_provider, cookie_state, verifier] = parts[..] else {
        return Err(ApiError::bad_request("malformed oauth cookie"));
    };
    if cookie_provider != provider.as_str() || cookie_state != state_param {
        return Err(ApiError::bad_request("state mismatch"));
    }

    let redirect_uri = oauth_redirect_uri(state, provider);
    let identity =
        oauth::exchange_and_fetch(&state.http, provider, cfg, &code, &redirect_uri, verifier)
            .await
            .map_err(|e| ApiError::internal(e.to_string()))?;

    // Already linked: log that user in.
    if let Some(link) = oauth_identities::Entity::find()
        .filter(oauth_identities::Column::Provider.eq(provider.as_str()))
        .filter(oauth_identities::Column::Subject.eq(&identity.subject))
        .one(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?
    {
        return Ok(link.user_id);
    }

    // Link to an existing account with a matching, already-verified email —
    // never auto-link to an unverified email, to avoid account takeover via
    // a spoofed email at the OAuth provider.
    let email_normalized = identity.email.as_deref().map(auth::normalize_email);
    if let Some(norm) = &email_normalized
        && let Some(existing) = users::Entity::find()
            .filter(users::Column::EmailNormalized.eq(norm))
            .filter(users::Column::EmailVerified.eq(true))
            .one(&state.db)
            .await
            .map_err(|e| ApiError::internal(e.to_string()))?
    {
        oauth_identities::ActiveModel {
            id: Set(Uuid::new_v4().to_string()),
            user_id: Set(existing.id.clone()),
            provider: Set(provider.as_str().to_string()),
            subject: Set(identity.subject.clone()),
            email: Set(identity.email.clone()),
            created_at: Set(chrono::Utc::now().to_rfc3339()),
        }
        .insert(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
        return Ok(existing.id);
    }

    // Otherwise, create a brand new account.
    let username = unique_oauth_username(state, &identity.suggested_username).await?;
    let user_count = users::Entity::find()
        .count(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    let placeholder_password = auth::hash_password_async(auth::random_secret())
        .await
        .map_err(ApiError::internal)?;
    let user = users::ActiveModel {
        id: Set(Uuid::new_v4().to_string()),
        username: Set(username),
        password_hash: Set(placeholder_password),
        is_admin: Set(user_count == 0),
        created_at: Set(chrono::Utc::now().to_rfc3339()),
        email: Set(identity.email.clone()),
        email_verified: Set(identity.email_verified && identity.email.is_some()),
        email_normalized: Set(email_normalized),
        mfa_totp_secret: Set(None),
        mfa_enabled: Set(false),
        // No password exists yet for a brand-new OAuth account; the
        // placeholder hash above is unguessable and unusable for login.
        // `/auth/set-password` lets the user add a real one later.
        password_set: Set(false),
    }
    .insert(&state.db)
    .await
    .map_err(|e| ApiError::internal(e.to_string()))?;

    oauth_identities::ActiveModel {
        id: Set(Uuid::new_v4().to_string()),
        user_id: Set(user.id.clone()),
        provider: Set(provider.as_str().to_string()),
        subject: Set(identity.subject.clone()),
        email: Set(identity.email.clone()),
        created_at: Set(chrono::Utc::now().to_rfc3339()),
    }
    .insert(&state.db)
    .await
    .map_err(|e| ApiError::internal(e.to_string()))?;

    Ok(user.id)
}

/// Turns a provider-suggested display name into a valid, unique username.
async fn unique_oauth_username(state: &PortState, suggested: &str) -> Result<String, ApiError> {
    let base: String = suggested
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let base = base.trim_matches('-');
    let base = if base.len() < 3 {
        "user".to_string()
    } else {
        base.chars().take(24).collect::<String>()
    };

    for attempt in 0..20 {
        let candidate = if attempt == 0 {
            base.clone()
        } else {
            format!("{base}-{}", &auth::random_secret()[..6])
        };
        let taken = users::Entity::find()
            .filter(users::Column::Username.eq(&candidate))
            .one(&state.db)
            .await
            .map_err(|e| ApiError::internal(e.to_string()))?
            .is_some();
        if !taken {
            return Ok(candidate);
        }
    }
    Err(ApiError::internal("could not allocate a unique username"))
}

// --- Audit log ---

#[get("/api/v2/auth/audit-log?<page>&<per_page>")]
pub async fn audit_log(
    state: &State<PortState>,
    user: AuthUser,
    page: Option<u32>,
    per_page: Option<u32>,
) -> Result<Json<Vec<AuditEntry>>, ApiError> {
    let page = page.unwrap_or(0);
    let per_page = per_page.unwrap_or(20).min(100);
    let pager = audit_log_entity::Entity::find()
        .filter(audit_log_entity::Column::UserId.eq(&user.id))
        .order_by_desc(audit_log_entity::Column::CreatedAt)
        .paginate(&state.db, per_page as u64);
    let items = pager
        .fetch_page(page as u64)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(Json(
        items
            .into_iter()
            .map(|row| AuditEntry {
                event: row.event,
                ip: row.ip,
                user_agent: row.user_agent,
                metadata: row.metadata,
                created_at: row.created_at,
            })
            .collect(),
    ))
}

// --- Passkeys (WebAuthn) ---

async fn user_passkeys(state: &PortState, user_id: &str) -> Result<Vec<Passkey>, ApiError> {
    let rows = webauthn_credentials::Entity::find()
        .filter(webauthn_credentials::Column::UserId.eq(user_id))
        .all(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    rows.iter()
        .map(|row| {
            serde_json::from_str::<Passkey>(&row.passkey_json)
                .map_err(|e| ApiError::internal(e.to_string()))
        })
        .collect()
}

#[post("/api/v2/auth/webauthn/register/start")]
pub async fn webauthn_register_start(
    state: &State<PortState>,
    user: AuthUser,
) -> Result<Json<WebauthnStartResponse>, ApiError> {
    let webauthn = state
        .webauthn
        .as_ref()
        .ok_or_else(|| ApiError::bad_request("passkeys are not configured on this server"))?;
    let user_id = Uuid::parse_str(&user.id).map_err(|e| ApiError::internal(e.to_string()))?;
    let existing = user_passkeys(state, &user.id).await?;
    let exclude =
        (!existing.is_empty()).then(|| existing.iter().map(|p| p.cred_id().clone()).collect());

    let (ccr, reg_state) = webauthn
        .start_passkey_registration(user_id, &user.username, &user.username, exclude)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    let state_json =
        serde_json::to_string(&reg_state).map_err(|e| ApiError::internal(e.to_string()))?;
    let token =
        auth::create_webauthn_challenge(&state.db, Some(&user.id), "register", state_json).await?;
    let options = serde_json::to_value(&ccr).map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(Json(WebauthnStartResponse { token, options }))
}

#[post("/api/v2/auth/webauthn/register/finish", data = "<input>")]
pub async fn webauthn_register_finish(
    state: &State<PortState>,
    user: AuthUser,
    input: Json<WebauthnRegisterFinishInput>,
) -> Result<Json<PasskeyInfo>, ApiError> {
    let webauthn = state
        .webauthn
        .as_ref()
        .ok_or_else(|| ApiError::bad_request("passkeys are not configured on this server"))?;
    if input.label.trim().is_empty() || input.label.len() > 64 {
        return Err(ApiError::bad_request("label must be 1-64 chars"));
    }
    let (challenge_user, state_json) =
        auth::consume_webauthn_challenge(&state.db, &input.token, "register")
            .await?
            .ok_or_else(|| ApiError::bad_request("invalid or expired challenge"))?;
    if challenge_user.as_deref() != Some(user.id.as_str()) {
        return Err(ApiError::bad_request(
            "challenge does not belong to this account",
        ));
    }
    let reg_state: webauthn_rs::prelude::PasskeyRegistration =
        serde_json::from_str(&state_json).map_err(|e| ApiError::internal(e.to_string()))?;
    let passkey = webauthn
        .finish_passkey_registration(&input.credential, &reg_state)
        .map_err(|e| ApiError::bad_request(e.to_string()))?;
    let passkey_json =
        serde_json::to_string(&passkey).map_err(|e| ApiError::internal(e.to_string()))?;

    let id = Uuid::new_v4().to_string();
    let created_at = chrono::Utc::now().to_rfc3339();
    webauthn_credentials::ActiveModel {
        id: Set(id.clone()),
        user_id: Set(user.id.clone()),
        label: Set(input.label.trim().to_string()),
        passkey_json: Set(passkey_json),
        created_at: Set(created_at.clone()),
        last_used_at: Set(None),
    }
    .insert(&state.db)
    .await
    .map_err(|e| ApiError::internal(e.to_string()))?;

    if let Some(model) = users::Entity::find_by_id(&user.id)
        .one(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?
    {
        auth::audit(
            &state.db,
            &user.id,
            "passkey_registered",
            None,
            None,
            Some(&input.label),
        )
        .await;
        alert_email(
            state,
            &model,
            "A passkey was added to your Caiven account",
            &format!(
                "A new passkey (\"{}\") was just added to your account.",
                input.label
            ),
        )
        .await;
    }

    Ok(Json(PasskeyInfo {
        id,
        label: input.label.trim().to_string(),
        created_at,
        last_used_at: None,
    }))
}

#[post("/api/v2/auth/webauthn/login/start", data = "<input>")]
pub async fn webauthn_login_start(
    state: &State<PortState>,
    input: Json<WebauthnLoginStartInput>,
) -> Result<Json<WebauthnStartResponse>, ApiError> {
    let webauthn = state
        .webauthn
        .as_ref()
        .ok_or_else(|| ApiError::bad_request("passkeys are not configured on this server"))?;
    let identifier = input.identifier.trim();
    let username = normalize_username(identifier);
    let email_normalized = auth::normalize_email(identifier);
    let user = users::Entity::find()
        .filter(
            users::Column::Username
                .eq(&username)
                .or(users::Column::EmailNormalized.eq(&email_normalized)),
        )
        .one(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?
        .ok_or_else(|| ApiError::not_found("no passkeys for this account"))?;
    let passkeys = user_passkeys(state, &user.id).await?;
    if passkeys.is_empty() {
        return Err(ApiError::not_found("no passkeys for this account"));
    }

    let (rcr, auth_state) = webauthn
        .start_passkey_authentication(&passkeys)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    let state_json =
        serde_json::to_string(&auth_state).map_err(|e| ApiError::internal(e.to_string()))?;
    let token =
        auth::create_webauthn_challenge(&state.db, Some(&user.id), "authenticate", state_json)
            .await?;
    let options = serde_json::to_value(&rcr).map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(Json(WebauthnStartResponse { token, options }))
}

#[post("/api/v2/auth/webauthn/login/finish", data = "<input>")]
pub async fn webauthn_login_finish(
    state: &State<PortState>,
    ip: ClientIp,
    ua: UserAgent,
    jar: &CookieJar<'_>,
    input: Json<WebauthnLoginFinishInput>,
) -> Result<Json<UserInfo>, ApiError> {
    let webauthn = state
        .webauthn
        .as_ref()
        .ok_or_else(|| ApiError::bad_request("passkeys are not configured on this server"))?;
    let (challenge_user, state_json) =
        auth::consume_webauthn_challenge(&state.db, &input.token, "authenticate")
            .await?
            .ok_or_else(|| ApiError::bad_request("invalid or expired challenge"))?;
    let user_id = challenge_user.ok_or_else(|| ApiError::internal("challenge missing user"))?;
    let auth_state: webauthn_rs::prelude::PasskeyAuthentication =
        serde_json::from_str(&state_json).map_err(|e| ApiError::internal(e.to_string()))?;

    let result = webauthn
        .finish_passkey_authentication(&input.credential, &auth_state)
        .map_err(|e| ApiError::bad_request(e.to_string()))?;

    let rows = webauthn_credentials::Entity::find()
        .filter(webauthn_credentials::Column::UserId.eq(&user_id))
        .all(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    for row in rows {
        let Ok(mut passkey) = serde_json::from_str::<Passkey>(&row.passkey_json) else {
            continue;
        };
        if passkey.update_credential(&result).is_some() {
            let passkey_json =
                serde_json::to_string(&passkey).map_err(|e| ApiError::internal(e.to_string()))?;
            let mut update: webauthn_credentials::ActiveModel = row.into();
            update.passkey_json = Set(passkey_json);
            update.last_used_at = Set(Some(chrono::Utc::now().to_rfc3339()));
            update
                .update(&state.db)
                .await
                .map_err(|e| ApiError::internal(e.to_string()))?;
            break;
        }
    }

    let user = users::Entity::find_by_id(&user_id)
        .one(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?
        .ok_or(ApiError::Unauthorized)?;

    // A passkey is itself strong, phishing-resistant authentication — no
    // separate TOTP step, same as how major providers treat passkey login.
    start_session(state, jar, &user.id, &session_ctx(&ip, &ua)).await?;
    notify(
        state,
        &user,
        "login_passkey",
        Some(&ip.0),
        ua.0.as_deref(),
        "New sign-in to your Caiven account",
        &format!("Your account was just signed in with a passkey from IP {}.\n\nIf this wasn't you, change your password immediately from Settings.", ip.0),
    )
    .await;
    Ok(Json(to_user_info(user)))
}

#[get("/api/v2/auth/webauthn/credentials")]
pub async fn list_passkeys(
    state: &State<PortState>,
    user: AuthUser,
) -> Result<Json<Vec<PasskeyInfo>>, ApiError> {
    let rows = webauthn_credentials::Entity::find()
        .filter(webauthn_credentials::Column::UserId.eq(&user.id))
        .order_by_desc(webauthn_credentials::Column::CreatedAt)
        .all(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(Json(
        rows.into_iter()
            .map(|r| PasskeyInfo {
                id: r.id,
                label: r.label,
                created_at: r.created_at,
                last_used_at: r.last_used_at,
            })
            .collect(),
    ))
}

#[delete("/api/v2/auth/webauthn/credentials/<id>")]
pub async fn delete_passkey(
    state: &State<PortState>,
    user: AuthUser,
    id: &str,
) -> Result<Status, ApiError> {
    let res = webauthn_credentials::Entity::delete_many()
        .filter(webauthn_credentials::Column::Id.eq(id))
        .filter(webauthn_credentials::Column::UserId.eq(&user.id))
        .exec(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    if res.rows_affected == 0 {
        return Err(ApiError::not_found("passkey not found"));
    }
    auth::audit(&state.db, &user.id, "passkey_removed", None, None, Some(id)).await;
    Ok(Status::NoContent)
}

// --- Account deletion & data export ---

#[delete("/api/v2/auth/account", data = "<input>")]
pub async fn delete_account(
    state: &State<PortState>,
    ip: ClientIp,
    ua: UserAgent,
    jar: &CookieJar<'_>,
    user: AuthUser,
    input: Json<DeleteAccountInput>,
) -> Result<Status, ApiError> {
    let model = users::Entity::find_by_id(&user.id)
        .one(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?
        .ok_or(ApiError::Unauthorized)?;
    if !auth::verify_login_password(
        input.current_password.clone(),
        Some(model.password_hash.clone()),
    )
    .await
    {
        return Err(ApiError::Unauthorized);
    }
    if model.mfa_enabled {
        let code = input
            .code
            .as_deref()
            .ok_or_else(|| ApiError::bad_request("two-factor code required"))?;
        let secret = model.mfa_totp_secret.as_deref().unwrap_or_default();
        let ok = auth::verify_totp_code(secret, code)
            || auth::consume_backup_code(&state.db, &user.id, code).await?;
        if !ok {
            return Err(ApiError::Unauthorized);
        }
    }

    // Record the audit entry (and email, best-effort) *before* deleting —
    // both reference the row we're about to remove.
    auth::audit(
        &state.db,
        &user.id,
        "account_deleted",
        Some(&ip.0),
        ua.0.as_deref(),
        None,
    )
    .await;
    alert_email(
        state,
        &model,
        "Your Caiven account was deleted",
        "This account and its sessions, tokens, and social contributions have been deleted. Published carts remain available under a generic \"legacy\" author.",
    )
    .await;

    let txn = state
        .db
        .begin()
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    db::ensure_legacy_user(&txn)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    carts::Entity::update_many()
        .col_expr(
            carts::Column::OwnerId,
            sea_orm::sea_query::Expr::value(db::LEGACY_USER_ID),
        )
        .filter(carts::Column::OwnerId.eq(&user.id))
        .exec(&txn)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    users::Entity::delete_by_id(&user.id)
        .exec(&txn)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    txn.commit()
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;

    jar.remove(Cookie::build(SESSION_COOKIE).path("/"));
    jar.remove(Cookie::build(CSRF_COOKIE).path("/"));
    Ok(Status::NoContent)
}

#[get("/api/v2/auth/export")]
pub async fn export_data(
    state: &State<PortState>,
    user: AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    let model = users::Entity::find_by_id(&user.id)
        .one(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?
        .ok_or(ApiError::Unauthorized)?;

    let owned_carts = carts::Entity::find()
        .filter(carts::Column::OwnerId.eq(&user.id))
        .all(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    let sessions_rows = sessions::Entity::find()
        .filter(sessions::Column::UserId.eq(&user.id))
        .all(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    let audit_rows = audit_log_entity::Entity::find()
        .filter(audit_log_entity::Column::UserId.eq(&user.id))
        .order_by_desc(audit_log_entity::Column::CreatedAt)
        .all(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "profile": {
            "id": model.id,
            "username": model.username,
            "email": model.email,
            "email_verified": model.email_verified,
            "is_admin": model.is_admin,
            "created_at": model.created_at,
        },
        "carts": owned_carts.iter().map(|c| serde_json::json!({
            "id": c.id,
            "title": c.title,
            "description": c.description,
            "tags": c.tags,
            "uploaded_at": c.uploaded_at,
            "downloads": c.downloads,
        })).collect::<Vec<_>>(),
        "sessions": sessions_rows.iter().map(|s| serde_json::json!({
            "created_at": s.created_at,
            "ip": s.ip,
            "user_agent": s.user_agent,
        })).collect::<Vec<_>>(),
        "audit_log": audit_rows.iter().map(|a| serde_json::json!({
            "event": a.event,
            "ip": a.ip,
            "created_at": a.created_at,
        })).collect::<Vec<_>>(),
    })))
}
