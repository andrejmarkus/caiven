use std::time::Duration;

use rocket::{
    State, delete, get,
    http::{Cookie, CookieJar, SameSite, Status},
    post,
    serde::json::Json,
    time::Duration as CookieDuration,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
};
use uuid::Uuid;

use crate::{
    PortState,
    auth::{self, AuthUser, ClientIp, SESSION_COOKIE},
    entities::{api_tokens, sessions, users},
    error::ApiError,
    models::{
        Credentials, PasswordChange, SessionInfo, TokenCreate, TokenCreated, TokenInfo, UserInfo,
    },
};

const REGISTER_LIMIT: u32 = 5;
const REGISTER_WINDOW: Duration = Duration::from_secs(3600);
const LOGIN_FAIL_LIMIT: u32 = 10;
const LOGIN_FAIL_WINDOW: Duration = Duration::from_secs(15 * 60);

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
    if !(15..=128).contains(&length) {
        return Err(ApiError::bad_request("password must be 15-128 characters"));
    }
    Ok(())
}

async fn start_session(
    state: &PortState,
    jar: &CookieJar<'_>,
    user_id: &str,
) -> Result<(), ApiError> {
    if let Some(cookie) = jar.get(SESSION_COOKIE) {
        auth::delete_session(&state.db, cookie.value()).await?;
    }
    let sid = auth::create_session(&state.db, user_id).await?;
    jar.add(
        Cookie::build((SESSION_COOKIE, sid))
            .http_only(true)
            .same_site(SameSite::Lax)
            .secure(state.secure_cookies)
            .path("/")
            .max_age(CookieDuration::days(auth::SESSION_DAYS)),
    );
    Ok(())
}

#[post("/api/v2/auth/register", data = "<creds>")]
pub async fn register(
    state: &State<PortState>,
    ip: ClientIp,
    jar: &CookieJar<'_>,
    creds: Json<Credentials>,
) -> Result<Json<UserInfo>, ApiError> {
    if state.rate.hit("register", &ip.0, REGISTER_WINDOW) > REGISTER_LIMIT {
        return Err(ApiError::TooManyRequests("try again later".into()));
    }
    let username = normalize_username(&creds.username);
    validate_username(&username)?;
    validate_password(&creds.password)?;

    let existing = users::Entity::find()
        .filter(users::Column::Username.eq(&username))
        .one(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    if existing.is_some() {
        return Err(ApiError::conflict("username taken"));
    }

    // First account on a fresh port becomes the admin.
    let user_count = users::Entity::find()
        .count(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;

    let user = users::ActiveModel {
        id: Set(Uuid::new_v4().to_string()),
        username: Set(username),
        password_hash: Set(auth::hash_password_async(creds.password.clone())
            .await
            .map_err(ApiError::internal)?),
        is_admin: Set(user_count == 0),
        created_at: Set(chrono::Utc::now().to_rfc3339()),
    }
    .insert(&state.db)
    .await
    .map_err(|_| ApiError::conflict("username taken"))?;

    start_session(state, jar, &user.id).await?;
    Ok(Json(UserInfo {
        id: user.id,
        username: user.username,
        is_admin: user.is_admin,
    }))
}

#[post("/api/v2/auth/login", data = "<creds>")]
pub async fn login(
    state: &State<PortState>,
    ip: ClientIp,
    jar: &CookieJar<'_>,
    creds: Json<Credentials>,
) -> Result<Json<UserInfo>, ApiError> {
    if state.rate.count("login_fail", &ip.0, LOGIN_FAIL_WINDOW) >= LOGIN_FAIL_LIMIT {
        return Err(ApiError::TooManyRequests(
            "too many failed logins, try again later".into(),
        ));
    }

    let username = normalize_username(&creds.username);
    let login_key = format!("{}:{username}", ip.0);
    if state
        .rate
        .count("login_identity_fail", &login_key, LOGIN_FAIL_WINDOW)
        >= LOGIN_FAIL_LIMIT
    {
        return Err(ApiError::TooManyRequests(
            "too many failed logins, try again later".into(),
        ));
    }

    let user = users::Entity::find()
        .filter(users::Column::Username.eq(&username))
        .one(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;

    let valid = auth::verify_login_password(
        creds.password.clone(),
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

    start_session(state, jar, &user.id).await?;
    Ok(Json(UserInfo {
        id: user.id,
        username: user.username,
        is_admin: user.is_admin,
    }))
}

#[post("/api/v2/auth/logout")]
pub async fn logout(state: &State<PortState>, jar: &CookieJar<'_>) -> Result<(), ApiError> {
    if let Some(cookie) = jar.get(SESSION_COOKIE) {
        auth::delete_session(&state.db, cookie.value()).await?;
        jar.remove(Cookie::build(SESSION_COOKIE).path("/"));
    }
    Ok(())
}

#[get("/api/v2/auth/me")]
pub async fn me(user: AuthUser) -> Json<UserInfo> {
    Json(UserInfo {
        id: user.id,
        username: user.username,
        is_admin: user.is_admin,
    })
}

#[post("/api/v2/auth/password", data = "<req>")]
pub async fn change_password(
    state: &State<PortState>,
    jar: &CookieJar<'_>,
    user: AuthUser,
    req: Json<PasswordChange>,
) -> Result<Status, ApiError> {
    validate_password(&req.new_password)?;
    let model = users::Entity::find_by_id(&user.id)
        .one(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?
        .ok_or(ApiError::Unauthorized)?;

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

    let mut update: users::ActiveModel = model.into();
    update.password_hash = Set(auth::hash_password_async(req.new_password.clone())
        .await
        .map_err(ApiError::internal)?);
    update
        .update(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;

    auth::delete_all_sessions(&state.db, &user.id).await?;
    start_session(state, jar, &user.id).await?;
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
    jar: &CookieJar<'_>,
    user: AuthUser,
) -> Result<Status, ApiError> {
    auth::delete_all_sessions(&state.db, &user.id).await?;
    jar.remove(Cookie::build(SESSION_COOKIE).path("/"));
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
