use std::time::Duration;

use rocket::{
    State, delete, get,
    http::{Cookie, CookieJar, SameSite},
    post,
    serde::json::Json,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
};
use uuid::Uuid;

use crate::{
    PortState,
    auth::{self, AuthUser, ClientIp, SESSION_COOKIE},
    entities::{api_tokens, users},
    error::ApiError,
    models::{Credentials, TokenCreate, TokenCreated, TokenInfo, UserInfo},
};

const REGISTER_LIMIT: u32 = 5;
const REGISTER_WINDOW: Duration = Duration::from_secs(3600);
const LOGIN_FAIL_LIMIT: u32 = 10;
const LOGIN_FAIL_WINDOW: Duration = Duration::from_secs(15 * 60);

fn validate_credentials(creds: &Credentials) -> Result<(), ApiError> {
    let name = &creds.username;
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
    if creds.password.len() < 8 || creds.password.len() > 128 {
        return Err(ApiError::bad_request("password must be 8-128 chars"));
    }
    Ok(())
}

async fn start_session(
    state: &PortState,
    jar: &CookieJar<'_>,
    user_id: &str,
) -> Result<(), ApiError> {
    let sid = auth::create_session(&state.db, user_id).await?;
    jar.add(
        Cookie::build((SESSION_COOKIE, sid))
            .http_only(true)
            .same_site(SameSite::Lax)
            .path("/"),
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
    validate_credentials(&creds)?;

    let existing = users::Entity::find()
        .filter(users::Column::Username.eq(&creds.username))
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
        username: Set(creds.username.clone()),
        password_hash: Set(auth::hash_password(&creds.password).map_err(ApiError::internal)?),
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

    let user = users::Entity::find()
        .filter(users::Column::Username.eq(&creds.username))
        .one(&state.db)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;

    let valid = user
        .as_ref()
        .is_some_and(|u| auth::verify_password(&creds.password, &u.password_hash));
    if !valid {
        state.rate.hit("login_fail", &ip.0, LOGIN_FAIL_WINDOW);
        return Err(ApiError::Unauthorized);
    }
    let user = user.expect("checked above");

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
