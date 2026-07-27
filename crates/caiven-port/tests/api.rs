//! API integration tests: in-memory SQLite + rocket local client, exercising
//! accounts, tokens, upload validation and the download roundtrip.

#![allow(clippy::unwrap_used)]

use caiven_port::{
    PortState,
    auth::{self, CSRF_COOKIE, CSRF_HEADER, SESSION_COOKIE},
    build_rocket,
    entities::{email_tokens, sessions, users, webauthn_challenges, webauthn_credentials},
};
use migration::MigratorTrait;
use rocket::data::{Limits, ToByteUnit};
use rocket::http::{ContentType, Header, SameSite, Status};
use rocket::local::asynchronous::Client;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter, Set,
};

const BOUNDARY: &str = "X-CAIVEN-PORT-TEST-BOUNDARY";
// Deliberately not a famous/textbook phrase (unlike the XKCD "correct horse
// battery staple") — that one is genuinely flagged by the real Pwned
// Passwords API, which the breached-password check now calls for real in
// these tests.
const TEST_PASSWORD: &str = "Zqf7-Glimmer-Porpoise!";
const NEW_TEST_PASSWORD: &str = "Vhm4-Trellis-Porpoise!";

async fn test_client(data_dir: &std::path::Path) -> Client {
    test_client_with_cookie_security(data_dir, false).await
}

async fn test_client_with_cookie_security(
    data_dir: &std::path::Path,
    secure_cookies: bool,
) -> Client {
    let web_dir = data_dir.join("web");
    std::fs::create_dir_all(&web_dir).unwrap();

    let db = sea_orm::Database::connect("sqlite::memory:").await.unwrap();
    migration::Migrator::up(&db, None).await.unwrap();

    let limits = Limits::default()
        .limit("data-form", 2.mebibytes())
        .limit("file", 2.mebibytes());
    let config = rocket::Config {
        limits,
        log_level: rocket::config::LogLevel::Off,
        ..rocket::Config::debug_default()
    };
    let state = PortState::for_testing(db, web_dir, secure_cookies);
    Client::tracked(build_rocket(config, state)).await.unwrap()
}

fn synthetic_email(username: &str) -> String {
    let local: String = username
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|c| if c.is_whitespace() { '-' } else { c })
        .collect();
    format!("{local}@example.test")
}

async fn register(client: &Client, username: &str, password: &str) -> Status {
    client
        .post("/api/v2/auth/register")
        .header(ContentType::JSON)
        .body(
            serde_json::json!({
                "username": username,
                "password": password,
                "email": synthetic_email(username),
            })
            .to_string(),
        )
        .dispatch()
        .await
        .status()
}

/// Register a default user (session cookie lands in the tracked client) and
/// mint an API token for header-based upload auth.
async fn auth_token(client: &Client) -> String {
    assert_eq!(register(client, "tester", TEST_PASSWORD).await, Status::Ok);
    let resp = client
        .post("/api/v2/auth/tokens")
        .header(ContentType::JSON)
        .header(csrf_header(client))
        .body(r#"{"name":"test"}"#)
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Ok);
    let body: serde_json::Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    body["token"].as_str().unwrap().to_string()
}

/// Build a multipart/form-data body with a `cart` file field and a `meta`
/// JSON field, matching what the engine's publish command sends.
fn multipart_body(cart: &[u8], meta: &str) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(format!("--{BOUNDARY}\r\n").as_bytes());
    body.extend_from_slice(
        b"Content-Disposition: form-data; name=\"cart\"; filename=\"test.cav\"\r\n\
          Content-Type: application/octet-stream\r\n\r\n",
    );
    body.extend_from_slice(cart);
    body.extend_from_slice(format!("\r\n--{BOUNDARY}\r\n").as_bytes());
    body.extend_from_slice(b"Content-Disposition: form-data; name=\"meta\"\r\n\r\n");
    body.extend_from_slice(meta.as_bytes());
    body.extend_from_slice(format!("\r\n--{BOUNDARY}--\r\n").as_bytes());
    body
}

/// Echoes the client's CSRF cookie back as the header cookie-authenticated
/// mutations require. Mirrors what the frontend's `api.ts` does.
fn csrf_header(client: &Client) -> Header<'static> {
    let value = client
        .cookies()
        .get(CSRF_COOKIE)
        .map(|c| c.value().to_string())
        .unwrap_or_default();
    Header::new(CSRF_HEADER, value)
}

fn multipart_content_type() -> ContentType {
    ContentType::parse_flexible(&format!("multipart/form-data; boundary={BOUNDARY}")).unwrap()
}

/// Builds a real, parseable `.cav` with the given program bytes (via the
/// shared `caiven-cart` writer), so uploads pass content-hash validation
/// the same way a real Studio-published cart would.
fn build_cart(program: &[u8]) -> Vec<u8> {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.cav");
    let header = caiven_cart::CartHeader::new("T", "A");
    caiven_cart::write(&path, &header, program, &[]).unwrap();
    std::fs::read(&path).unwrap()
}

fn sample_cart() -> Vec<u8> {
    build_cart(&[0u8; 64])
}

/// Register a user, mint a token for it, then log out so the client's
/// cookie jar (shared across all these helpers) doesn't leak that user's
/// session into later requests authenticated by a *different* user's token.
async fn register_get_token_and_logout(client: &Client, username: &str) -> String {
    assert_eq!(register(client, username, TEST_PASSWORD).await, Status::Ok);
    let resp = client
        .post("/api/v2/auth/tokens")
        .header(ContentType::JSON)
        .header(csrf_header(client))
        .body(r#"{"name":"test"}"#)
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Ok);
    let body: serde_json::Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    let token = body["token"].as_str().unwrap().to_string();
    client.post("/api/v2/auth/logout").dispatch().await;
    token
}

async fn upload<'c>(
    client: &'c Client,
    token: &str,
    cart: &[u8],
    meta: &str,
) -> rocket::local::asynchronous::LocalResponse<'c> {
    client
        .post("/api/carts")
        .header(Header::new("X-Api-Key", token.to_string()))
        .header(multipart_content_type())
        .body(multipart_body(cart, meta))
        .dispatch()
        .await
}

// ── auth ──────────────────────────────────────────────────────────────────────

#[rocket::async_test]
async fn register_login_logout_flow() {
    let dir = tempfile::tempdir().unwrap();
    let client = test_client(dir.path()).await;

    assert_eq!(register(&client, "alice", TEST_PASSWORD).await, Status::Ok);

    let resp = client.get("/api/v2/auth/me").dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    let me: serde_json::Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(me["username"], "alice");

    let resp = client.post("/api/v2/auth/logout").dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    let resp = client.get("/api/v2/auth/me").dispatch().await;
    assert_eq!(resp.status(), Status::Unauthorized);

    let resp = client
        .post("/api/v2/auth/login")
        .header(ContentType::JSON)
        .body(format!(
            r#"{{"identifier":"alice","password":"{TEST_PASSWORD}"}}"#
        ))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Ok);
    let resp = client.get("/api/v2/auth/me").dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
}

#[rocket::async_test]
async fn first_user_is_admin_second_is_not() {
    let dir = tempfile::tempdir().unwrap();
    let client = test_client(dir.path()).await;

    let resp = client
        .post("/api/v2/auth/register")
        .header(ContentType::JSON)
        .body(format!(
            r#"{{"username":"first","password":"{TEST_PASSWORD}","email":"first@example.test"}}"#
        ))
        .dispatch()
        .await;
    let body: serde_json::Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["is_admin"], true);

    let resp = client
        .post("/api/v2/auth/register")
        .header(ContentType::JSON)
        .body(format!(
            r#"{{"username":"second","password":"{TEST_PASSWORD}","email":"second@example.test"}}"#
        ))
        .dispatch()
        .await;
    let body: serde_json::Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["is_admin"], false);
}

#[rocket::async_test]
async fn duplicate_username_is_409() {
    let dir = tempfile::tempdir().unwrap();
    let client = test_client(dir.path()).await;

    assert_eq!(register(&client, "bob", TEST_PASSWORD).await, Status::Ok);
    assert_eq!(
        register(&client, "bob", "Other-Password!").await,
        Status::Conflict
    );
}

#[rocket::async_test]
async fn invalid_username_and_short_password_are_400() {
    let dir = tempfile::tempdir().unwrap();
    let client = test_client(dir.path()).await;

    assert_eq!(
        register(&client, "Bad Name", TEST_PASSWORD).await,
        Status::BadRequest
    );
    assert_eq!(
        register(&client, "ok", TEST_PASSWORD).await,
        Status::BadRequest
    );
    assert_eq!(
        register(&client, "goodname", "short").await,
        Status::BadRequest
    );
}

#[rocket::async_test]
async fn wrong_password_is_401() {
    let dir = tempfile::tempdir().unwrap();
    let client = test_client(dir.path()).await;

    assert_eq!(register(&client, "carol", TEST_PASSWORD).await, Status::Ok);
    let resp = client
        .post("/api/v2/auth/login")
        .header(ContentType::JSON)
        .body(r#"{"identifier":"carol","password":"wrong password long enough"}"#)
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Unauthorized);
}

#[rocket::async_test]
async fn hardened_session_cookie_is_hashed_rotated_and_not_cached() {
    let dir = tempfile::tempdir().unwrap();
    let client = test_client_with_cookie_security(dir.path(), true).await;

    let resp = client
        .post("/api/v2/auth/register")
        .header(ContentType::JSON)
        .body(format!(
            r#"{{"username":"secureuser","password":"{TEST_PASSWORD}","email":"secureuser@example.test"}}"#
        ))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Ok);
    assert_eq!(resp.headers().get_one("Cache-Control"), Some("no-store"));
    let cookie = resp.cookies().get(SESSION_COOKIE).unwrap();
    assert_eq!(cookie.http_only(), Some(true));
    assert_eq!(cookie.same_site(), Some(SameSite::Lax));
    assert_eq!(cookie.secure(), Some(true));
    assert_eq!(cookie.path(), Some("/"));
    assert!(cookie.max_age().is_some());

    let first_token = client
        .cookies()
        .get(SESSION_COOKIE)
        .unwrap()
        .value()
        .to_string();
    let first_hash = auth::sha256_hex(&first_token);
    let state = client.rocket().state::<PortState>().unwrap();
    assert!(
        sessions::Entity::find_by_id(&first_hash)
            .one(&state.db)
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        sessions::Entity::find_by_id(&first_token)
            .one(&state.db)
            .await
            .unwrap()
            .is_none()
    );

    let resp = client
        .post("/api/v2/auth/login")
        .header(ContentType::JSON)
        .body(format!(
            r#"{{"identifier":"secureuser","password":"{TEST_PASSWORD}"}}"#
        ))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Ok);
    let second_token = client
        .cookies()
        .get(SESSION_COOKIE)
        .unwrap()
        .value()
        .to_string();
    assert_ne!(first_token, second_token);
    assert!(
        sessions::Entity::find_by_id(first_hash)
            .one(&state.db)
            .await
            .unwrap()
            .is_none()
    );
}

#[rocket::async_test]
async fn password_change_revokes_sessions_and_enforces_policy() {
    let dir = tempfile::tempdir().unwrap();
    let client = test_client(dir.path()).await;

    assert_eq!(
        register(&client, "shortpass", "fourteenchars!").await,
        Status::BadRequest
    );
    // Multi-byte chars only, but still satisfies uppercase (Ω) + special
    // (🔐) — exercises char-count (not byte-length) validation.
    let unicode_password = "Ω🔐-unicode-password".to_string();
    assert_eq!(
        register(&client, "unicodeuser", &unicode_password).await,
        Status::Ok
    );
    client.post("/api/v2/auth/logout").dispatch().await;

    assert_eq!(
        register(&client, "passworduser", TEST_PASSWORD).await,
        Status::Ok
    );
    let state = client.rocket().state::<PortState>().unwrap();
    let user = users::Entity::find()
        .filter(users::Column::Username.eq("passworduser"))
        .one(&state.db)
        .await
        .unwrap()
        .unwrap();
    auth::create_session(&state.db, &user.id, &auth::SessionContext::default())
        .await
        .unwrap();

    let resp = client
        .post("/api/v2/auth/password")
        .header(ContentType::JSON)
        .header(csrf_header(&client))
        .body(format!(
            r#"{{"current_password":"{TEST_PASSWORD}","new_password":"{NEW_TEST_PASSWORD}"}}"#
        ))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::NoContent);
    assert_eq!(
        sessions::Entity::find()
            .filter(sessions::Column::UserId.eq(&user.id))
            .count(&state.db)
            .await
            .unwrap(),
        1
    );

    client.post("/api/v2/auth/logout").dispatch().await;
    let resp = client
        .post("/api/v2/auth/login")
        .header(ContentType::JSON)
        .body(format!(
            r#"{{"identifier":"passworduser","password":"{TEST_PASSWORD}"}}"#
        ))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Unauthorized);
    let resp = client
        .post("/api/v2/auth/login")
        .header(ContentType::JSON)
        .body(format!(
            r#"{{"identifier":"passworduser","password":"{NEW_TEST_PASSWORD}"}}"#
        ))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Ok);
}

#[rocket::async_test]
async fn session_management_is_owner_scoped_and_capped() {
    let dir = tempfile::tempdir().unwrap();
    let client = test_client(dir.path()).await;
    assert_eq!(
        register(&client, "sessionuser", TEST_PASSWORD).await,
        Status::Ok
    );
    let state = client.rocket().state::<PortState>().unwrap();
    let user = users::Entity::find()
        .filter(users::Column::Username.eq("sessionuser"))
        .one(&state.db)
        .await
        .unwrap()
        .unwrap();
    let (_, api_token) = auth::create_token(&state.db, &user.id, "session-test")
        .await
        .unwrap();

    for _ in 0..25 {
        auth::create_session(&state.db, &user.id, &auth::SessionContext::default())
            .await
            .unwrap();
    }
    assert_eq!(
        sessions::Entity::find()
            .filter(sessions::Column::UserId.eq(&user.id))
            .count(&state.db)
            .await
            .unwrap(),
        20
    );

    let other = users::ActiveModel {
        id: Set(uuid::Uuid::new_v4().to_string()),
        username: Set("otheruser".into()),
        password_hash: Set(auth::hash_password(TEST_PASSWORD).unwrap()),
        is_admin: Set(false),
        created_at: Set(chrono::Utc::now().to_rfc3339()),
        email: Set(None),
        email_verified: Set(false),
        email_normalized: Set(None),
        mfa_totp_secret: Set(None),
        mfa_enabled: Set(false),
        password_set: Set(true),
    }
    .insert(&state.db)
    .await
    .unwrap();
    let other_token = auth::create_session(&state.db, &other.id, &auth::SessionContext::default())
        .await
        .unwrap();
    let other_id = auth::sha256_hex(&other_token);

    let resp = client
        .get("/api/v2/auth/sessions")
        .header(Header::new("X-Api-Key", api_token.clone()))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Ok);
    let listed: serde_json::Value =
        serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(listed.as_array().unwrap().len(), 20);

    let resp = client
        .delete(format!("/api/v2/auth/sessions/{other_id}"))
        .header(Header::new("X-Api-Key", api_token.clone()))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::NotFound);
    assert!(
        sessions::Entity::find_by_id(&other_id)
            .one(&state.db)
            .await
            .unwrap()
            .is_some()
    );

    let resp = client
        .delete("/api/v2/auth/sessions")
        .header(Header::new("X-Api-Key", api_token))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::NoContent);
    assert_eq!(
        sessions::Entity::find()
            .filter(sessions::Column::UserId.eq(&user.id))
            .count(&state.db)
            .await
            .unwrap(),
        0
    );
    assert!(
        sessions::Entity::find_by_id(other_id)
            .one(&state.db)
            .await
            .unwrap()
            .is_some()
    );
}

#[rocket::async_test]
async fn login_canonicalizes_identity() {
    let dir = tempfile::tempdir().unwrap();
    let client = test_client(dir.path()).await;

    assert_eq!(
        register(&client, "  MixedCase  ", TEST_PASSWORD).await,
        Status::Ok
    );
    client.post("/api/v2/auth/logout").dispatch().await;
    let resp = client
        .post("/api/v2/auth/login")
        .header(ContentType::JSON)
        .body(format!(
            r#"{{"identifier":" MIXEDCASE ","password":"{TEST_PASSWORD}"}}"#
        ))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Ok);
    let body: serde_json::Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["user"]["username"], "mixedcase");
}

#[rocket::async_test]
async fn revoked_token_is_401() {
    let dir = tempfile::tempdir().unwrap();
    let client = test_client(dir.path()).await;
    let token = auth_token(&client).await;

    let resp = upload(
        &client,
        &token,
        &sample_cart(),
        r#"{"title":"T","author":"A"}"#,
    )
    .await;
    assert_eq!(resp.status(), Status::Ok);

    let resp = client.get("/api/v2/auth/tokens").dispatch().await;
    let tokens: serde_json::Value =
        serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    let token_id = tokens[0]["id"].as_str().unwrap().to_string();

    let resp = client
        .delete(format!("/api/v2/auth/tokens/{token_id}"))
        .header(csrf_header(&client))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Ok);

    // Session cookie would still authenticate; check the token alone via a
    // fresh non-tracked request path: logout first, then try the token.
    client.post("/api/v2/auth/logout").dispatch().await;
    let resp = upload(
        &client,
        &token,
        &sample_cart(),
        r#"{"title":"T","author":"A"}"#,
    )
    .await;
    assert_eq!(resp.status(), Status::Unauthorized);
}

// ── carts ─────────────────────────────────────────────────────────────────────

#[rocket::async_test]
async fn upload_without_auth_is_401() {
    let dir = tempfile::tempdir().unwrap();
    let client = test_client(dir.path()).await;

    let resp = client
        .post("/api/carts")
        .header(multipart_content_type())
        .body(multipart_body(
            &sample_cart(),
            r#"{"title":"T","author":"A"}"#,
        ))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Unauthorized);
}

#[rocket::async_test]
async fn upload_and_download_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let client = test_client(dir.path()).await;
    let token = auth_token(&client).await;
    let cart_bytes = sample_cart();

    let resp = upload(
        &client,
        &token,
        &cart_bytes,
        r#"{"title":"Catch","author":"Andrej","description":"demo","tags":["arcade"]}"#,
    )
    .await;
    assert_eq!(resp.status(), Status::Ok);
    let cart: serde_json::Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(cart["title"], "Catch");
    assert_eq!(cart["cart_size"], cart_bytes.len() as i64);
    let id = cart["id"].as_str().unwrap().to_string();

    let resp = client.get(format!("/api/carts/{id}")).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);

    let resp = client.get(format!("/api/carts/{id}/cart")).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    assert_eq!(resp.into_bytes().await.unwrap(), cart_bytes);

    let resp = client.get("/api/carts?q=Catch").dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    let list: serde_json::Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(list["total"], 1);
}

#[rocket::async_test]
async fn invalid_cart_magic_is_400() {
    let dir = tempfile::tempdir().unwrap();
    let client = test_client(dir.path()).await;
    let token = auth_token(&client).await;

    let resp = upload(
        &client,
        &token,
        b"NOTAROM-BYTES",
        r#"{"title":"T","author":"A"}"#,
    )
    .await;
    assert_eq!(resp.status(), Status::BadRequest);
}

#[rocket::async_test]
async fn empty_title_is_400() {
    let dir = tempfile::tempdir().unwrap();
    let client = test_client(dir.path()).await;
    let token = auth_token(&client).await;

    let resp = upload(
        &client,
        &token,
        &sample_cart(),
        r#"{"title":"  ","author":"A"}"#,
    )
    .await;
    assert_eq!(resp.status(), Status::BadRequest);
}

#[rocket::async_test]
async fn oversize_cart_is_413() {
    let dir = tempfile::tempdir().unwrap();
    let client = test_client(dir.path()).await;
    let token = auth_token(&client).await;

    let mut cart_bytes = sample_cart();
    cart_bytes.resize(1024 * 1024 + 1, 0);
    let resp = upload(
        &client,
        &token,
        &cart_bytes,
        r#"{"title":"T","author":"A"}"#,
    )
    .await;
    assert_eq!(resp.status(), Status::PayloadTooLarge);
}

#[rocket::async_test]
async fn malformed_id_is_400_and_unknown_id_is_404() {
    let dir = tempfile::tempdir().unwrap();
    let client = test_client(dir.path()).await;

    let resp = client.get("/api/carts/not-a-uuid").dispatch().await;
    assert_eq!(resp.status(), Status::BadRequest);

    let resp = client
        .get("/api/carts/00000000-0000-0000-0000-000000000000")
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::NotFound);
}

// ── carts v2: ownership + versioning + discovery ────────────────────────────

#[rocket::async_test]
async fn ownership_enforced_admin_can_override() {
    let dir = tempfile::tempdir().unwrap();
    let client = test_client(dir.path()).await;

    // First registered user becomes admin.
    let admin_token = register_get_token_and_logout(&client, "admin").await;
    let owner_token = register_get_token_and_logout(&client, "owner").await;
    let other_token = register_get_token_and_logout(&client, "other").await;

    let resp = upload(
        &client,
        &owner_token,
        &sample_cart(),
        r#"{"title":"Mine","author":"Owner"}"#,
    )
    .await;
    assert_eq!(resp.status(), Status::Ok);
    let cart: serde_json::Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    let id = cart["id"].as_str().unwrap().to_string();

    let resp = client
        .patch(format!("/api/v2/carts/{id}"))
        .header(Header::new("X-Api-Key", other_token.clone()))
        .header(ContentType::JSON)
        .body(r#"{"title":"Hacked"}"#)
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Forbidden);

    let resp = client
        .patch(format!("/api/v2/carts/{id}"))
        .header(Header::new("X-Api-Key", owner_token.clone()))
        .header(ContentType::JSON)
        .body(r#"{"title":"Renamed"}"#)
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Ok);
    let updated: serde_json::Value =
        serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(updated["title"], "Renamed");

    let resp = client
        .delete(format!("/api/v2/carts/{id}"))
        .header(Header::new("X-Api-Key", other_token.clone()))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Forbidden);

    let resp = client
        .delete(format!("/api/v2/carts/{id}"))
        .header(Header::new("X-Api-Key", admin_token.clone()))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Ok);

    let resp = client.get(format!("/api/v2/carts/{id}")).dispatch().await;
    assert_eq!(resp.status(), Status::NotFound);
}

#[rocket::async_test]
async fn versioning_upload_list_download_and_delete() {
    let dir = tempfile::tempdir().unwrap();
    let client = test_client(dir.path()).await;
    let token = auth_token(&client).await;

    let cart_v1 = sample_cart();
    let resp = upload(
        &client,
        &token,
        &cart_v1,
        r#"{"title":"Game","author":"A"}"#,
    )
    .await;
    assert_eq!(resp.status(), Status::Ok);
    let cart: serde_json::Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    let id = cart["id"].as_str().unwrap().to_string();

    let cart_v2 = build_cart(&[1u8; 80]);
    let resp = client
        .post(format!("/api/v2/carts/{id}/versions"))
        .header(Header::new("X-Api-Key", token.clone()))
        .header(multipart_content_type())
        .body(multipart_body(&cart_v2, r#"{"changelog":"fix bug"}"#))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Ok);
    let v2: serde_json::Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(v2["version"], 2);
    assert_eq!(v2["changelog"], "fix bug");

    let resp = client.get(format!("/api/v2/carts/{id}")).dispatch().await;
    let detail: serde_json::Value =
        serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(detail["versions"].as_array().unwrap().len(), 2);
    assert_eq!(detail["latest_version"], 2);

    let resp = client
        .get(format!("/api/v2/carts/{id}/cart"))
        .dispatch()
        .await;
    assert_eq!(resp.into_bytes().await.unwrap(), cart_v2);

    let resp = client
        .get(format!("/api/v2/carts/{id}/cart?version=1"))
        .dispatch()
        .await;
    assert_eq!(resp.into_bytes().await.unwrap(), cart_v1);

    let resp = client
        .delete(format!("/api/v2/carts/{id}"))
        .header(Header::new("X-Api-Key", token.clone()))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Ok);

    let resp = client
        .get(format!("/api/v2/carts/{id}/cart"))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::NotFound);
}

#[rocket::async_test]
async fn discovery_tag_author_filters_and_lookups() {
    let dir = tempfile::tempdir().unwrap();
    let client = test_client(dir.path()).await;
    let token = auth_token(&client).await; // registers "tester"

    let resp = upload(
        &client,
        &token,
        &sample_cart(),
        r#"{"title":"Alpha","author":"Zed","tags":["Arcade","Retro"]}"#,
    )
    .await;
    assert_eq!(resp.status(), Status::Ok);
    let resp = upload(
        &client,
        &token,
        &sample_cart(),
        r#"{"title":"Beta","author":"Amy","tags":["Puzzle"]}"#,
    )
    .await;
    assert_eq!(resp.status(), Status::Ok);

    let resp = client.get("/api/v2/carts?tag=retro").dispatch().await;
    let list: serde_json::Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(list["total"], 1);
    assert_eq!(list["carts"][0]["title"], "Alpha");

    let resp = client.get("/api/v2/carts?author=tester").dispatch().await;
    let list: serde_json::Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(list["total"], 2);
    assert!(
        list["carts"]
            .as_array()
            .unwrap()
            .iter()
            .all(|cart| cart["author"] == "tester")
    );

    let resp = client.get("/api/v2/tags").dispatch().await;
    let tags: serde_json::Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    let tag_names: Vec<&str> = tags
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["tag"].as_str().unwrap())
        .collect();
    assert!(tag_names.contains(&"retro"));
    assert!(tag_names.contains(&"puzzle"));

    let resp = client.get("/api/v2/users/tester").dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    let profile: serde_json::Value =
        serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(profile["total"], 2);

    let resp = client.get("/api/v2/users/nobody").dispatch().await;
    assert_eq!(resp.status(), Status::NotFound);
}

#[rocket::async_test]
async fn sort_popular_orders_by_downloads() {
    let dir = tempfile::tempdir().unwrap();
    let client = test_client(dir.path()).await;
    let token = auth_token(&client).await;

    let resp = upload(
        &client,
        &token,
        &sample_cart(),
        r#"{"title":"Quiet","author":"A"}"#,
    )
    .await;
    let quiet: serde_json::Value =
        serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    let resp = upload(
        &client,
        &token,
        &sample_cart(),
        r#"{"title":"Popular","author":"A"}"#,
    )
    .await;
    let popular: serde_json::Value =
        serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    for _ in 0..3 {
        client
            .get(format!(
                "/api/v2/carts/{}/cart",
                popular["id"].as_str().unwrap()
            ))
            .dispatch()
            .await;
    }

    let resp = client.get("/api/v2/carts?sort=popular").dispatch().await;
    let list: serde_json::Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(list["carts"][0]["title"], "Popular");
    assert_eq!(list["carts"][1]["title"], "Quiet");
    let _ = quiet;
}

// ── social: ratings + comments ──────────────────────────────────────────────

#[rocket::async_test]
async fn rating_upsert_is_one_per_user_and_averages() {
    let dir = tempfile::tempdir().unwrap();
    let client = test_client(dir.path()).await;

    let owner_token = register_get_token_and_logout(&client, "owner").await;
    let alice_token = register_get_token_and_logout(&client, "alice").await;
    let bob_token = register_get_token_and_logout(&client, "bob").await;

    let resp = upload(
        &client,
        &owner_token,
        &sample_cart(),
        r#"{"title":"Game","author":"A"}"#,
    )
    .await;
    let cart: serde_json::Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    let id = cart["id"].as_str().unwrap().to_string();

    let resp = client
        .put(format!("/api/v2/carts/{id}/rating"))
        .header(Header::new("X-Api-Key", alice_token.clone()))
        .header(ContentType::JSON)
        .body(r#"{"score":4}"#)
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Ok);
    let rated: serde_json::Value =
        serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(rated["rating_count"], 1);
    assert_eq!(rated["rating_avg"], 4.0);

    let resp = client
        .put(format!("/api/v2/carts/{id}/rating"))
        .header(Header::new("X-Api-Key", bob_token.clone()))
        .header(ContentType::JSON)
        .body(r#"{"score":2}"#)
        .dispatch()
        .await;
    let rated: serde_json::Value =
        serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(rated["rating_count"], 2);
    assert_eq!(rated["rating_avg"], 3.0);

    // Alice changes her mind: 4 -> 5. Still one rating from her, avg updates.
    let resp = client
        .put(format!("/api/v2/carts/{id}/rating"))
        .header(Header::new("X-Api-Key", alice_token.clone()))
        .header(ContentType::JSON)
        .body(r#"{"score":5}"#)
        .dispatch()
        .await;
    let rated: serde_json::Value =
        serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(rated["rating_count"], 2);
    assert_eq!(rated["rating_avg"], 3.5);

    let resp = client.get(format!("/api/v2/carts/{id}")).dispatch().await;
    let detail: serde_json::Value =
        serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(detail["own_rating"], serde_json::Value::Null);

    let resp = client
        .get(format!("/api/v2/carts/{id}"))
        .header(Header::new("X-Api-Key", alice_token.clone()))
        .dispatch()
        .await;
    let detail: serde_json::Value =
        serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(detail["own_rating"], 5);

    let resp = client
        .delete(format!("/api/v2/carts/{id}/rating"))
        .header(Header::new("X-Api-Key", bob_token.clone()))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Ok);
    let rated: serde_json::Value =
        serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(rated["rating_count"], 1);
    assert_eq!(rated["rating_avg"], 5.0);
}

#[rocket::async_test]
async fn rating_out_of_range_is_400_and_requires_auth() {
    let dir = tempfile::tempdir().unwrap();
    let client = test_client(dir.path()).await;
    let token = register_get_token_and_logout(&client, "tester").await;

    let resp = upload(
        &client,
        &token,
        &sample_cart(),
        r#"{"title":"Game","author":"A"}"#,
    )
    .await;
    let cart: serde_json::Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    let id = cart["id"].as_str().unwrap().to_string();

    let resp = client
        .put(format!("/api/v2/carts/{id}/rating"))
        .header(ContentType::JSON)
        .body(r#"{"score":3}"#)
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Unauthorized);

    let resp = client
        .put(format!("/api/v2/carts/{id}/rating"))
        .header(Header::new("X-Api-Key", token.clone()))
        .header(ContentType::JSON)
        .body(r#"{"score":6}"#)
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::BadRequest);
}

#[rocket::async_test]
async fn comments_add_list_and_delete_permissions() {
    let dir = tempfile::tempdir().unwrap();
    let client = test_client(dir.path()).await;

    let owner_token = register_get_token_and_logout(&client, "owner").await;
    let commenter_token = register_get_token_and_logout(&client, "commenter").await;
    let stranger_token = register_get_token_and_logout(&client, "stranger").await;

    let resp = upload(
        &client,
        &owner_token,
        &sample_cart(),
        r#"{"title":"Game","author":"A"}"#,
    )
    .await;
    let cart: serde_json::Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    let id = cart["id"].as_str().unwrap().to_string();

    let resp = client
        .post(format!("/api/v2/carts/{id}/comments"))
        .header(Header::new("X-Api-Key", commenter_token.clone()))
        .header(ContentType::JSON)
        .body(r#"{"body":"Great game!"}"#)
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Ok);
    let comment: serde_json::Value =
        serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(comment["author"], "commenter");
    assert_eq!(comment["body"], "Great game!");
    let comment_id = comment["id"].as_str().unwrap().to_string();

    let resp = client
        .post(format!("/api/v2/carts/{id}/comments"))
        .header(ContentType::JSON)
        .body(r#"{"body":"anonymous"}"#)
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Unauthorized);

    let resp = client
        .post(format!("/api/v2/carts/{id}/comments"))
        .header(Header::new("X-Api-Key", commenter_token.clone()))
        .header(ContentType::JSON)
        .body(r#"{"body":"   "}"#)
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::BadRequest);

    let resp = client
        .get(format!("/api/v2/carts/{id}/comments"))
        .dispatch()
        .await;
    let list: serde_json::Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(list.as_array().unwrap().len(), 1);

    // Stranger (not the commenter or cart owner) can't delete.
    let resp = client
        .delete(format!("/api/v2/carts/{id}/comments/{comment_id}"))
        .header(Header::new("X-Api-Key", stranger_token.clone()))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Forbidden);

    // Cart owner can delete someone else's comment.
    let resp = client
        .delete(format!("/api/v2/carts/{id}/comments/{comment_id}"))
        .header(Header::new("X-Api-Key", owner_token.clone()))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Ok);

    let resp = client
        .get(format!("/api/v2/carts/{id}/comments"))
        .dispatch()
        .await;
    let list: serde_json::Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(list.as_array().unwrap().len(), 0);
}

#[rocket::async_test]
async fn legacy_carts_are_migrated_to_legacy_owner_with_v1() {
    let db = sea_orm::Database::connect("sqlite::memory:").await.unwrap();
    // Apply only the pre-v2 schema (carts + auth tables).
    migration::Migrator::up(&db, Some(2)).await.unwrap();

    // Seed a cart row in the old shape, bypassing entities (which now expect
    // the v2 schema) to simulate data uploaded before accounts existed.
    db.execute_unprepared(
        "INSERT INTO carts (id, title, author, description, tags, uploaded_at, downloads, has_screenshot, rom_size) \
         VALUES ('11111111-1111-1111-1111-111111111111', 'Old Game', 'Retro Dev', '', '', \
                 '2024-01-01T00:00:00Z', 3, 1, 512)",
    )
    .await
    .unwrap();

    // Now apply the v2 migration, which should adopt the row under `legacy`.
    migration::Migrator::up(&db, None).await.unwrap();

    let cart = caiven_port::db::get(&db, "11111111-1111-1111-1111-111111111111")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(cart.owner.as_deref(), Some("legacy"));
    assert_eq!(cart.latest_version, 1);
    assert_eq!(cart.cart_size, 512);
    assert!(cart.has_screenshot);
}

#[rocket::async_test]
async fn existing_cart_versions_receive_content_hashes() {
    let db = sea_orm::Database::connect("sqlite::memory:").await.unwrap();
    migration::Migrator::up(&db, Some(12)).await.unwrap();
    db.execute_unprepared(
        "INSERT INTO carts \
            (id, title, author, description, tags, uploaded_at, downloads, owner_id, \
             rating_count, rating_sum, plays) \
         VALUES \
            ('blob-cart', 'Blob', 'author', '', '', '2026-01-01T00:00:00Z', 0, NULL, 0, 0, 0), \
            ('legacy-cart', 'Legacy', 'author', '', '', '2026-01-01T00:00:00Z', 0, NULL, 0, 0, 0)",
    )
    .await
    .unwrap();
    db.execute_unprepared(
        "INSERT INTO cart_versions \
            (id, cart_id, version, cart_size, changelog, has_screenshot, created_at, \
             legacy_cart_path, editor_username) \
         VALUES \
            ('blob-version', 'blob-cart', 1, 0, '', 0, '2026-01-01T00:00:00Z', NULL, 'author'), \
            ('legacy-version', 'legacy-cart', 1, 0, '', 0, '2026-01-01T00:00:00Z', \
             'carts/legacy.cav', 'author')",
    )
    .await
    .unwrap();
    let cart = sample_cart();
    db.execute(sea_orm::Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Sqlite,
        "INSERT INTO cart_blobs (version_id, cart_data, screenshot_data) VALUES (?, ?, NULL)",
        ["blob-version".into(), cart.clone().into()],
    ))
    .await
    .unwrap();

    migration::Migrator::up(&db, None).await.unwrap();
    let expected = caiven_cart::content_hash(&cart).unwrap();
    let blob_hash: Option<String> = db
        .query_one(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Sqlite,
            "SELECT content_hash FROM cart_versions WHERE id = 'blob-version'",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "content_hash")
        .unwrap();
    assert_eq!(blob_hash.as_deref(), Some(expected.as_str()));

    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("carts")).unwrap();
    std::fs::write(dir.path().join("carts/legacy.cav"), &cart).unwrap();
    assert_eq!(
        caiven_port::db::backfill_legacy_cart_content_hashes(&db, dir.path())
            .await
            .unwrap(),
        1
    );
    let legacy_hash: Option<String> = db
        .query_one(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Sqlite,
            "SELECT content_hash FROM cart_versions WHERE id = 'legacy-version'",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "content_hash")
        .unwrap();
    assert_eq!(legacy_hash.as_deref(), Some(expected.as_str()));
}

#[rocket::async_test]
async fn play_event_is_idempotent_per_cart_session() {
    let dir = tempfile::tempdir().unwrap();
    let client = test_client(dir.path()).await;
    let token = auth_token(&client).await;
    let response = upload(
        &client,
        &token,
        &sample_cart(),
        r#"{"title":"Playable","author":"tester","description":"","tags":[]}"#,
    )
    .await;
    let uploaded: serde_json::Value =
        serde_json::from_str(&response.into_string().await.unwrap()).unwrap();
    let id = uploaded["id"].as_str().unwrap();

    for expected_counted in [true, false] {
        let response = client
            .post(format!("/api/v2/carts/{id}/play"))
            .header(ContentType::JSON)
            .body(r#"{"session_id":"11111111-1111-4111-8111-111111111111"}"#)
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        let body: serde_json::Value =
            serde_json::from_str(&response.into_string().await.unwrap()).unwrap();
        assert_eq!(body["counted"], expected_counted);
        assert_eq!(body["plays"], 1);
    }
}

#[rocket::async_test]
async fn player_collection_is_public_and_contains_ordered_carts() {
    let dir = tempfile::tempdir().unwrap();
    let client = test_client(dir.path()).await;
    let token = auth_token(&client).await;
    let response = upload(
        &client,
        &token,
        &sample_cart(),
        r#"{"title":"Collected","author":"tester","description":"","tags":[]}"#,
    )
    .await;
    let uploaded: serde_json::Value =
        serde_json::from_str(&response.into_string().await.unwrap()).unwrap();
    let id = uploaded["id"].as_str().unwrap();

    let response = client
        .post("/api/v2/collections")
        .header(Header::new("X-Api-Key", token.clone()))
        .header(ContentType::JSON)
        .body(r#"{"title":"Tiny favorites","description":"Public shelf"}"#)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);
    let collection: serde_json::Value =
        serde_json::from_str(&response.into_string().await.unwrap()).unwrap();
    let slug = collection["slug"].as_str().unwrap();

    let response = client
        .post(format!("/api/v2/collections/{slug}/carts"))
        .header(Header::new("X-Api-Key", token))
        .header(ContentType::JSON)
        .body(format!(r#"{{"cart_id":"{id}"}}"#))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);

    client.post("/api/v2/auth/logout").dispatch().await;
    let response = client
        .get(format!("/api/v2/collections/{slug}"))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);
    let public: serde_json::Value =
        serde_json::from_str(&response.into_string().await.unwrap()).unwrap();
    assert_eq!(public["kind"], "player");
    assert_eq!(public["carts"][0]["id"], id);
}

#[rocket::async_test]
async fn admin_can_create_open_jam_and_owner_can_enter_cart() {
    let dir = tempfile::tempdir().unwrap();
    let client = test_client(dir.path()).await;
    let token = auth_token(&client).await;
    let response = upload(
        &client,
        &token,
        &sample_cart(),
        r#"{"title":"Jam Cart","author":"tester","description":"","tags":[]}"#,
    )
    .await;
    let uploaded: serde_json::Value =
        serde_json::from_str(&response.into_string().await.unwrap()).unwrap();
    let id = uploaded["id"].as_str().unwrap();
    let starts = (chrono::Utc::now() - chrono::Duration::hours(1)).to_rfc3339();
    let closes = (chrono::Utc::now() + chrono::Duration::hours(2)).to_rfc3339();
    let ends = (chrono::Utc::now() + chrono::Duration::hours(3)).to_rfc3339();
    let response = client
        .post("/api/v2/admin/jams")
        .header(Header::new("X-Api-Key", token.clone()))
        .header(ContentType::JSON)
        .body(
            serde_json::json!({
                "title": "One Screen",
                "description": "One frame",
                "rules": "No camera",
                "starts_at": starts,
                "submissions_close_at": closes,
                "ends_at": ends
            })
            .to_string(),
        )
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);
    let jam: serde_json::Value =
        serde_json::from_str(&response.into_string().await.unwrap()).unwrap();
    let slug = jam["slug"].as_str().unwrap();
    assert_eq!(jam["status"], "open");

    let response = client
        .post(format!("/api/v2/jams/{slug}/entries"))
        .header(Header::new("X-Api-Key", token))
        .header(ContentType::JSON)
        .body(format!(r#"{{"cart_id":"{id}"}}"#))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);
    let jam: serde_json::Value =
        serde_json::from_str(&response.into_string().await.unwrap()).unwrap();
    assert_eq!(jam["entry_count"], 1);
}

// ── modern auth: email, verification, reset, antibot config ────────────────

#[rocket::async_test]
async fn auth_config_reports_no_antibot_or_oauth_by_default() {
    let dir = tempfile::tempdir().unwrap();
    let client = test_client(dir.path()).await;

    let resp = client.get("/api/v2/auth/config").dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    let cfg: serde_json::Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(cfg["turnstile_site_key"], serde_json::Value::Null);
    assert_eq!(cfg["providers"], serde_json::json!([]));
}

#[rocket::async_test]
async fn register_rejects_invalid_email() {
    let dir = tempfile::tempdir().unwrap();
    let client = test_client(dir.path()).await;

    let resp = client
        .post("/api/v2/auth/register")
        .header(ContentType::JSON)
        .body(format!(
            r#"{{"username":"noemail","password":"{TEST_PASSWORD}","email":"not-an-email"}}"#
        ))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::BadRequest);
}

#[rocket::async_test]
async fn register_rejects_duplicate_email_across_usernames() {
    let dir = tempfile::tempdir().unwrap();
    let client = test_client(dir.path()).await;

    let resp = client
        .post("/api/v2/auth/register")
        .header(ContentType::JSON)
        .body(format!(
            r#"{{"username":"dupone","password":"{TEST_PASSWORD}","email":"shared@example.test"}}"#
        ))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Ok);
    client.post("/api/v2/auth/logout").dispatch().await;

    let resp = client
        .post("/api/v2/auth/register")
        .header(ContentType::JSON)
        .body(format!(
            r#"{{"username":"duptwo","password":"{TEST_PASSWORD}","email":"shared@example.test"}}"#
        ))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Conflict);
}

#[rocket::async_test]
async fn login_by_email_identifier_works() {
    let dir = tempfile::tempdir().unwrap();
    let client = test_client(dir.path()).await;

    assert_eq!(
        register(&client, "emaillogin", TEST_PASSWORD).await,
        Status::Ok
    );
    client.post("/api/v2/auth/logout").dispatch().await;

    let resp = client
        .post("/api/v2/auth/login")
        .header(ContentType::JSON)
        .body(format!(
            r#"{{"identifier":"EMAILLOGIN@Example.Test","password":"{TEST_PASSWORD}"}}"#
        ))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Ok);
    let body: serde_json::Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["user"]["username"], "emaillogin");
}

#[rocket::async_test]
async fn without_smtp_new_accounts_are_auto_verified() {
    let dir = tempfile::tempdir().unwrap();
    let client = test_client(dir.path()).await;

    let resp = client
        .post("/api/v2/auth/register")
        .header(ContentType::JSON)
        .body(format!(
            r#"{{"username":"autoverify","password":"{TEST_PASSWORD}","email":"autoverify@example.test"}}"#
        ))
        .dispatch()
        .await;
    let body: serde_json::Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["email_verified"], true);
}

#[rocket::async_test]
async fn unverified_email_blocks_writes_but_not_reads() {
    let dir = tempfile::tempdir().unwrap();
    let client = test_client(dir.path()).await;
    let token = auth_token(&client).await; // "tester", auto-verified

    let state = client.rocket().state::<PortState>().unwrap();
    let user = users::Entity::find()
        .filter(users::Column::Username.eq("tester"))
        .one(&state.db)
        .await
        .unwrap()
        .unwrap();
    let mut update: users::ActiveModel = user.into();
    update.email_verified = Set(false);
    update.update(&state.db).await.unwrap();

    let resp = upload(
        &client,
        &token,
        &sample_cart(),
        r#"{"title":"T","author":"A"}"#,
    )
    .await;
    assert_eq!(resp.status(), Status::Forbidden);

    // Reads stay open regardless of verification state.
    let resp = client.get("/api/v2/carts").dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
}

#[rocket::async_test]
async fn email_verification_token_is_single_use() {
    let dir = tempfile::tempdir().unwrap();
    let client = test_client(dir.path()).await;

    assert_eq!(
        register(&client, "verifyme", TEST_PASSWORD).await,
        Status::Ok
    );
    let state = client.rocket().state::<PortState>().unwrap();
    let user = users::Entity::find()
        .filter(users::Column::Username.eq("verifyme"))
        .one(&state.db)
        .await
        .unwrap()
        .unwrap();
    let mut update: users::ActiveModel = user.clone().into();
    update.email_verified = Set(false);
    update.update(&state.db).await.unwrap();

    let token = auth::create_email_token(&state.db, &user.id, "verify", 24)
        .await
        .unwrap();

    let resp = client
        .post("/api/v2/auth/verify-email")
        .header(ContentType::JSON)
        .body(format!(r#"{{"token":"{token}"}}"#))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::NoContent);

    let refreshed = users::Entity::find_by_id(&user.id)
        .one(&state.db)
        .await
        .unwrap()
        .unwrap();
    assert!(refreshed.email_verified);

    // Reusing the same token fails.
    let resp = client
        .post("/api/v2/auth/verify-email")
        .header(ContentType::JSON)
        .body(format!(r#"{{"token":"{token}"}}"#))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::BadRequest);
}

#[rocket::async_test]
async fn forgot_password_is_always_204_and_does_not_enumerate() {
    let dir = tempfile::tempdir().unwrap();
    let client = test_client(dir.path()).await;
    assert_eq!(
        register(&client, "forgotuser", TEST_PASSWORD).await,
        Status::Ok
    );

    let resp = client
        .post("/api/v2/auth/forgot-password")
        .header(ContentType::JSON)
        .body(r#"{"email":"forgotuser@example.test"}"#)
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::NoContent);

    let resp = client
        .post("/api/v2/auth/forgot-password")
        .header(ContentType::JSON)
        .body(r#"{"email":"nobody-here@example.test"}"#)
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::NoContent);
}

#[rocket::async_test]
async fn password_reset_token_resets_password_revokes_sessions_and_is_single_use() {
    let dir = tempfile::tempdir().unwrap();
    let client = test_client(dir.path()).await;
    assert_eq!(
        register(&client, "resetuser", TEST_PASSWORD).await,
        Status::Ok
    );

    let state = client.rocket().state::<PortState>().unwrap();
    let user = users::Entity::find()
        .filter(users::Column::Username.eq("resetuser"))
        .one(&state.db)
        .await
        .unwrap()
        .unwrap();
    auth::create_session(&state.db, &user.id, &auth::SessionContext::default())
        .await
        .unwrap();
    assert_eq!(
        sessions::Entity::find()
            .filter(sessions::Column::UserId.eq(&user.id))
            .count(&state.db)
            .await
            .unwrap(),
        2 // the register-time session plus the extra one above
    );

    let token = auth::create_email_token(&state.db, &user.id, "reset", 1)
        .await
        .unwrap();
    // create_email_token invalidated the earlier verify-kind tokens only, so
    // this is the only live `reset` token — sanity check it's stored hashed.
    assert!(
        email_tokens::Entity::find()
            .filter(email_tokens::Column::UserId.eq(&user.id))
            .filter(email_tokens::Column::Kind.eq("reset"))
            .one(&state.db)
            .await
            .unwrap()
            .is_some_and(|row| row.token_hash != token)
    );

    let resp = client
        .post("/api/v2/auth/reset-password")
        .header(ContentType::JSON)
        .body(format!(
            r#"{{"token":"{token}","new_password":"{NEW_TEST_PASSWORD}"}}"#
        ))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::NoContent);

    assert_eq!(
        sessions::Entity::find()
            .filter(sessions::Column::UserId.eq(&user.id))
            .count(&state.db)
            .await
            .unwrap(),
        0
    );

    let resp = client
        .post("/api/v2/auth/login")
        .header(ContentType::JSON)
        .body(format!(
            r#"{{"identifier":"resetuser","password":"{NEW_TEST_PASSWORD}"}}"#
        ))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Ok);

    // Reusing the reset token fails.
    let resp = client
        .post("/api/v2/auth/reset-password")
        .header(ContentType::JSON)
        .body(format!(
            r#"{{"token":"{token}","new_password":"Another-New-Password!"}}"#
        ))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::BadRequest);
}

// ── security hardening: 2FA, CSRF, session metadata, set-password ──────────

fn totp_code_for(secret: &str) -> String {
    let bytes = totp_rs::Secret::Encoded(secret.to_string())
        .to_bytes()
        .unwrap();
    let totp = totp_rs::TOTP::new(
        totp_rs::Algorithm::SHA1,
        6,
        1,
        30,
        bytes,
        Some("Caiven Port".to_string()),
        "test".to_string(),
    )
    .unwrap();
    totp.generate_current().unwrap()
}

#[rocket::async_test]
async fn mfa_setup_confirm_login_and_backup_code_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let client = test_client(dir.path()).await;
    assert_eq!(
        register(&client, "mfauser", TEST_PASSWORD).await,
        Status::Ok
    );

    let resp = client
        .post("/api/v2/auth/mfa/setup")
        .header(csrf_header(&client))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Ok);
    let setup: serde_json::Value =
        serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    let secret = setup["secret"].as_str().unwrap().to_string();
    assert!(
        setup["otpauth_url"]
            .as_str()
            .unwrap()
            .starts_with("otpauth://")
    );

    // Wrong code is rejected.
    let resp = client
        .post("/api/v2/auth/mfa/confirm")
        .header(ContentType::JSON)
        .header(csrf_header(&client))
        .body(r#"{"code":"000000"}"#)
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::BadRequest);

    let resp = client
        .post("/api/v2/auth/mfa/confirm")
        .header(ContentType::JSON)
        .header(csrf_header(&client))
        .body(format!(r#"{{"code":"{}"}}"#, totp_code_for(&secret)))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Ok);
    let confirmed: serde_json::Value =
        serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    let backup_codes: Vec<String> = confirmed["backup_codes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert_eq!(backup_codes.len(), 10);

    client.post("/api/v2/auth/logout").dispatch().await;

    // Login now stops at the MFA step.
    let resp = client
        .post("/api/v2/auth/login")
        .header(ContentType::JSON)
        .body(format!(
            r#"{{"identifier":"mfauser","password":"{TEST_PASSWORD}"}}"#
        ))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Ok);
    let outcome: serde_json::Value =
        serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(outcome["mfa_required"], true);
    assert_eq!(outcome["user"], serde_json::Value::Null);
    let pending_token = outcome["pending_token"].as_str().unwrap().to_string();

    // Wrong code fails the second step.
    let resp = client
        .post("/api/v2/auth/login/mfa")
        .header(ContentType::JSON)
        .body(format!(
            r#"{{"pending_token":"{pending_token}","code":"000000"}}"#
        ))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Unauthorized);

    // A backup code completes login and is then single-use.
    let resp = client
        .post("/api/v2/auth/login/mfa")
        .header(ContentType::JSON)
        .body(format!(
            r#"{{"pending_token":"{pending_token}","code":"{}"}}"#,
            backup_codes[0]
        ))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Ok);
    let user: serde_json::Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(user["username"], "mfauser");

    // The pending_token was consumed by the first successful call above, so
    // reusing it (even with a fresh backup code) fails outright.
    let resp = client
        .post("/api/v2/auth/login/mfa")
        .header(ContentType::JSON)
        .body(format!(
            r#"{{"pending_token":"{pending_token}","code":"{}"}}"#,
            backup_codes[1]
        ))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::BadRequest);

    // Disabling requires the password and a valid code/backup code.
    let resp = client
        .post("/api/v2/auth/mfa/disable")
        .header(ContentType::JSON)
        .header(csrf_header(&client))
        .body(format!(
            r#"{{"current_password":"{TEST_PASSWORD}","code":"{}"}}"#,
            backup_codes[1]
        ))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::NoContent);

    let resp = client.get("/api/v2/auth/mfa/status").dispatch().await;
    let status: serde_json::Value =
        serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(status["enabled"], false);
}

#[rocket::async_test]
async fn csrf_header_required_for_cookie_auth_mutations_but_not_api_key() {
    let dir = tempfile::tempdir().unwrap();
    let client = test_client(dir.path()).await;
    assert_eq!(
        register(&client, "csrfuser", TEST_PASSWORD).await,
        Status::Ok
    );

    // Missing header: rejected.
    let resp = client.post("/api/v2/auth/mfa/setup").dispatch().await;
    assert_eq!(resp.status(), Status::Forbidden);

    // Wrong header value: rejected.
    let resp = client
        .post("/api/v2/auth/mfa/setup")
        .header(Header::new(CSRF_HEADER, "not-the-real-token"))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Forbidden);

    // Correct header: allowed.
    let resp = client
        .post("/api/v2/auth/mfa/setup")
        .header(csrf_header(&client))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Ok);

    // A GET (safe method) needs no CSRF header even when cookie-authenticated.
    let resp = client.get("/api/v2/auth/mfa/status").dispatch().await;
    assert_eq!(resp.status(), Status::Ok);

    // X-Api-Key auth is exempt from CSRF entirely, cookies or not.
    let resp = client
        .post("/api/v2/auth/tokens")
        .header(ContentType::JSON)
        .header(csrf_header(&client))
        .body(r#"{"name":"api-token"}"#)
        .dispatch()
        .await;
    let token = serde_json::from_str::<serde_json::Value>(&resp.into_string().await.unwrap())
        .unwrap()["token"]
        .as_str()
        .unwrap()
        .to_string();
    client.post("/api/v2/auth/logout").dispatch().await;
    let resp = client
        .post("/api/v2/auth/tokens")
        .header(ContentType::JSON)
        .header(Header::new("X-Api-Key", token))
        .body(r#"{"name":"second-token"}"#)
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Ok);
}

#[rocket::async_test]
async fn set_password_only_works_once_for_passwordless_accounts() {
    let dir = tempfile::tempdir().unwrap();
    let client = test_client(dir.path()).await;
    assert_eq!(
        register(&client, "oauthlike", TEST_PASSWORD).await,
        Status::Ok
    );

    // Simulate an OAuth-created account: no password set yet.
    let state = client.rocket().state::<PortState>().unwrap();
    let user = users::Entity::find()
        .filter(users::Column::Username.eq("oauthlike"))
        .one(&state.db)
        .await
        .unwrap()
        .unwrap();
    let mut update: users::ActiveModel = user.into();
    update.password_set = Set(false);
    update.update(&state.db).await.unwrap();

    // change_password refuses; set_password succeeds exactly once.
    let resp = client
        .post("/api/v2/auth/password")
        .header(ContentType::JSON)
        .header(csrf_header(&client))
        .body(format!(
            r#"{{"current_password":"{TEST_PASSWORD}","new_password":"{NEW_TEST_PASSWORD}"}}"#
        ))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::BadRequest);

    let resp = client
        .post("/api/v2/auth/set-password")
        .header(ContentType::JSON)
        .header(csrf_header(&client))
        .body(format!(r#"{{"new_password":"{NEW_TEST_PASSWORD}"}}"#))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::NoContent);

    let resp = client
        .post("/api/v2/auth/set-password")
        .header(ContentType::JSON)
        .header(csrf_header(&client))
        .body(r#"{"new_password":"Yet-Another-Password!"}"#)
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::BadRequest);
}

#[rocket::async_test]
async fn sessions_record_user_agent_and_ip() {
    let dir = tempfile::tempdir().unwrap();
    let client = test_client(dir.path()).await;

    let resp = client
        .post("/api/v2/auth/register")
        .header(ContentType::JSON)
        .header(Header::new("User-Agent", "test-agent/1.0"))
        .body(format!(
            r#"{{"username":"uauser","password":"{TEST_PASSWORD}","email":"uauser@example.test"}}"#
        ))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Ok);

    let resp = client.get("/api/v2/auth/sessions").dispatch().await;
    let sessions: serde_json::Value =
        serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(sessions[0]["user_agent"], "test-agent/1.0");
    assert!(sessions[0]["ip"].is_string());
    assert!(sessions[0]["last_seen_at"].is_string());
}

#[rocket::async_test]
async fn studio_link_polling_covers_advertised_lifetime() {
    let dir = tempfile::tempdir().unwrap();
    let client = test_client(dir.path()).await;
    let start = client.post("/api/v2/auth/studio-link").dispatch().await;
    assert_eq!(start.status(), Status::Ok);
    let link: serde_json::Value =
        serde_json::from_str(&start.into_string().await.unwrap()).unwrap();
    let request = serde_json::json!({
        "request_id": link["request_id"],
        "poll_secret": link["poll_secret"],
    })
    .to_string();

    // Studio polls every two seconds for a ten-minute request lifetime.
    for _ in 0..300 {
        let poll = client
            .post("/api/v2/auth/studio-link/poll")
            .header(ContentType::JSON)
            .body(request.clone())
            .dispatch()
            .await;
        assert_eq!(poll.status(), Status::Ok);
    }
}

// ── security round 3: breached-password, audit log, passkeys, deletion/export ──

#[rocket::async_test]
async fn audit_log_records_login_and_password_change() {
    let dir = tempfile::tempdir().unwrap();
    let client = test_client(dir.path()).await;
    assert_eq!(
        register(&client, "audituser", TEST_PASSWORD).await,
        Status::Ok
    );
    client.post("/api/v2/auth/logout").dispatch().await;

    let resp = client
        .post("/api/v2/auth/login")
        .header(ContentType::JSON)
        .body(format!(
            r#"{{"identifier":"audituser","password":"{TEST_PASSWORD}"}}"#
        ))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Ok);

    let resp = client
        .post("/api/v2/auth/password")
        .header(ContentType::JSON)
        .header(csrf_header(&client))
        .body(format!(
            r#"{{"current_password":"{TEST_PASSWORD}","new_password":"{NEW_TEST_PASSWORD}"}}"#
        ))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::NoContent);

    let resp = client.get("/api/v2/auth/audit-log").dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    let entries: serde_json::Value =
        serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    let events: Vec<&str> = entries
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["event"].as_str().unwrap())
        .collect();
    // Newest first: password_changed happened after login.
    assert_eq!(events[0], "password_changed");
    assert!(events.contains(&"login"));
}

#[rocket::async_test]
async fn account_deletion_reassigns_carts_to_legacy_and_wipes_account() {
    let dir = tempfile::tempdir().unwrap();
    let client = test_client(dir.path()).await;
    assert_eq!(
        register(&client, "deleteme", TEST_PASSWORD).await,
        Status::Ok
    );

    let resp = client
        .post("/api/carts")
        .header(csrf_header(&client))
        .header(multipart_content_type())
        .body(multipart_body(
            &sample_cart(),
            r#"{"title":"Orphaned","author":"deleteme"}"#,
        ))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Ok);
    let cart: serde_json::Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    let id = cart["id"].as_str().unwrap().to_string();

    let resp = client
        .delete("/api/v2/auth/account")
        .header(ContentType::JSON)
        .header(csrf_header(&client))
        .body(format!(r#"{{"current_password":"{TEST_PASSWORD}"}}"#))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::NoContent);

    let resp = client.get(format!("/api/v2/carts/{id}")).dispatch().await;
    let detail: serde_json::Value =
        serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(detail["owner"], "legacy");

    // The account no longer exists.
    let resp = client
        .post("/api/v2/auth/login")
        .header(ContentType::JSON)
        .body(format!(
            r#"{{"identifier":"deleteme","password":"{TEST_PASSWORD}"}}"#
        ))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Unauthorized);
}

#[rocket::async_test]
async fn data_export_includes_profile_and_owned_carts() {
    let dir = tempfile::tempdir().unwrap();
    let client = test_client(dir.path()).await;
    assert_eq!(
        register(&client, "exportuser", TEST_PASSWORD).await,
        Status::Ok
    );

    let resp = client
        .post("/api/carts")
        .header(csrf_header(&client))
        .header(multipart_content_type())
        .body(multipart_body(
            &sample_cart(),
            r#"{"title":"Mine","author":"exportuser"}"#,
        ))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Ok);

    let resp = client.get("/api/v2/auth/export").dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    let export: serde_json::Value =
        serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(export["profile"]["username"], "exportuser");
    assert_eq!(export["carts"].as_array().unwrap().len(), 1);
    assert_eq!(export["carts"][0]["title"], "Mine");
}

#[rocket::async_test]
async fn webauthn_login_start_400_when_not_configured() {
    let dir = tempfile::tempdir().unwrap();
    let client = test_client(dir.path()).await;
    assert_eq!(
        register(&client, "nopasskeys", TEST_PASSWORD).await,
        Status::Ok
    );

    // Test PortState has no CAIVEN_BASE_URL, so webauthn is unconfigured
    // regardless of whether the account has passkeys.
    let resp = client
        .post("/api/v2/auth/webauthn/login/start")
        .header(ContentType::JSON)
        .body(r#"{"identifier":"nopasskeys"}"#)
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::BadRequest);
}

#[rocket::async_test]
async fn webauthn_login_start_is_rate_limited() {
    let dir = tempfile::tempdir().unwrap();
    let client = test_client(dir.path()).await;
    for _ in 0..10 {
        let response = client
            .post("/api/v2/auth/webauthn/login/start")
            .header(ContentType::JSON)
            .body(r#"{"identifier":"unknown"}"#)
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::BadRequest);
    }
    let response = client
        .post("/api/v2/auth/webauthn/login/start")
        .header(ContentType::JSON)
        .body(r#"{"identifier":"unknown"}"#)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::TooManyRequests);
}

#[rocket::async_test]
async fn creating_webauthn_challenge_removes_expired_rows() {
    let dir = tempfile::tempdir().unwrap();
    let client = test_client(dir.path()).await;
    let state = client.rocket().state::<PortState>().unwrap();
    webauthn_challenges::ActiveModel {
        id: Set("expired".into()),
        user_id: Set(None),
        kind: Set("authenticate".into()),
        state_json: Set("{}".into()),
        expires_at: Set((chrono::Utc::now() - chrono::Duration::minutes(1)).to_rfc3339()),
        created_at: Set((chrono::Utc::now() - chrono::Duration::minutes(6)).to_rfc3339()),
    }
    .insert(&state.db)
    .await
    .unwrap();

    auth::create_webauthn_challenge(&state.db, None, "authenticate", "{}".into())
        .await
        .unwrap();
    assert!(
        webauthn_challenges::Entity::find_by_id("expired")
            .one(&state.db)
            .await
            .unwrap()
            .is_none()
    );
}

#[rocket::async_test]
async fn passkey_list_and_delete_are_owner_scoped() {
    let dir = tempfile::tempdir().unwrap();
    let client = test_client(dir.path()).await;
    let owner_token = register_get_token_and_logout(&client, "pkowner").await;
    let other_token = register_get_token_and_logout(&client, "pkother").await;

    let state = client.rocket().state::<PortState>().unwrap();
    let owner = users::Entity::find()
        .filter(users::Column::Username.eq("pkowner"))
        .one(&state.db)
        .await
        .unwrap()
        .unwrap();
    let cred_id = uuid::Uuid::new_v4().to_string();
    webauthn_credentials::ActiveModel {
        id: Set(cred_id.clone()),
        user_id: Set(owner.id.clone()),
        label: Set("Test key".into()),
        passkey_json: Set("{}".into()),
        created_at: Set(chrono::Utc::now().to_rfc3339()),
        last_used_at: Set(None),
    }
    .insert(&state.db)
    .await
    .unwrap();

    let resp = client
        .get("/api/v2/auth/webauthn/credentials")
        .header(Header::new("X-Api-Key", owner_token.clone()))
        .dispatch()
        .await;
    let list: serde_json::Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(list.as_array().unwrap().len(), 1);
    assert_eq!(list[0]["label"], "Test key");

    let resp = client
        .get("/api/v2/auth/webauthn/credentials")
        .header(Header::new("X-Api-Key", other_token.clone()))
        .dispatch()
        .await;
    let list: serde_json::Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(list.as_array().unwrap().len(), 0);

    let resp = client
        .delete(format!("/api/v2/auth/webauthn/credentials/{cred_id}"))
        .header(Header::new("X-Api-Key", other_token))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::NotFound);

    let resp = client
        .delete(format!("/api/v2/auth/webauthn/credentials/{cred_id}"))
        .header(Header::new("X-Api-Key", owner_token))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::NoContent);
}
