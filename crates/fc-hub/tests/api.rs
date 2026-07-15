//! API integration tests: in-memory SQLite + rocket local client, exercising
//! accounts, tokens, upload validation and the download roundtrip.

#![allow(clippy::unwrap_used)]

use fc_hub::{HubState, auth::RateLimiter, build_rocket};
use migration::MigratorTrait;
use rocket::data::{Limits, ToByteUnit};
use rocket::http::{ContentType, Header, Status};
use rocket::local::asynchronous::Client;

const BOUNDARY: &str = "X-FC-HUB-TEST-BOUNDARY";

async fn test_client(data_dir: &std::path::Path) -> Client {
    std::fs::create_dir_all(data_dir.join("roms")).unwrap();
    std::fs::create_dir_all(data_dir.join("screenshots")).unwrap();

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
    let state = HubState {
        db,
        data_dir: data_dir.to_path_buf(),
        rate: RateLimiter::default(),
    };
    Client::tracked(build_rocket(config, state)).await.unwrap()
}

async fn register(client: &Client, username: &str, password: &str) -> Status {
    client
        .post("/api/v2/auth/register")
        .header(ContentType::JSON)
        .body(format!(
            r#"{{"username":"{username}","password":"{password}"}}"#
        ))
        .dispatch()
        .await
        .status()
}

/// Register a default user (session cookie lands in the tracked client) and
/// mint an API token for header-based upload auth.
async fn auth_token(client: &Client) -> String {
    assert_eq!(register(client, "tester", "password123").await, Status::Ok);
    let resp = client
        .post("/api/v2/auth/tokens")
        .header(ContentType::JSON)
        .body(r#"{"name":"test"}"#)
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Ok);
    let body: serde_json::Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    body["token"].as_str().unwrap().to_string()
}

/// Build a multipart/form-data body with a `rom` file field and a `meta`
/// JSON field, matching what the engine's publish command sends.
fn multipart_body(rom: &[u8], meta: &str) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(format!("--{BOUNDARY}\r\n").as_bytes());
    body.extend_from_slice(
        b"Content-Disposition: form-data; name=\"rom\"; filename=\"test.rom\"\r\n\
          Content-Type: application/octet-stream\r\n\r\n",
    );
    body.extend_from_slice(rom);
    body.extend_from_slice(format!("\r\n--{BOUNDARY}\r\n").as_bytes());
    body.extend_from_slice(b"Content-Disposition: form-data; name=\"meta\"\r\n\r\n");
    body.extend_from_slice(meta.as_bytes());
    body.extend_from_slice(format!("\r\n--{BOUNDARY}--\r\n").as_bytes());
    body
}

fn multipart_content_type() -> ContentType {
    ContentType::parse_flexible(&format!("multipart/form-data; boundary={BOUNDARY}")).unwrap()
}

fn sample_rom() -> Vec<u8> {
    let mut rom = b"SPEAR2".to_vec();
    rom.extend_from_slice(&[0u8; 64]);
    rom
}

async fn upload<'c>(
    client: &'c Client,
    token: &str,
    rom: &[u8],
    meta: &str,
) -> rocket::local::asynchronous::LocalResponse<'c> {
    client
        .post("/api/carts")
        .header(Header::new("X-Api-Key", token.to_string()))
        .header(multipart_content_type())
        .body(multipart_body(rom, meta))
        .dispatch()
        .await
}

// ── auth ──────────────────────────────────────────────────────────────────────

#[rocket::async_test]
async fn register_login_logout_flow() {
    let dir = tempfile::tempdir().unwrap();
    let client = test_client(dir.path()).await;

    assert_eq!(register(&client, "alice", "password123").await, Status::Ok);

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
        .body(r#"{"username":"alice","password":"password123"}"#)
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
        .body(r#"{"username":"first","password":"password123"}"#)
        .dispatch()
        .await;
    let body: serde_json::Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["is_admin"], true);

    let resp = client
        .post("/api/v2/auth/register")
        .header(ContentType::JSON)
        .body(r#"{"username":"second","password":"password123"}"#)
        .dispatch()
        .await;
    let body: serde_json::Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(body["is_admin"], false);
}

#[rocket::async_test]
async fn duplicate_username_is_409() {
    let dir = tempfile::tempdir().unwrap();
    let client = test_client(dir.path()).await;

    assert_eq!(register(&client, "bob", "password123").await, Status::Ok);
    assert_eq!(
        register(&client, "bob", "otherpassword").await,
        Status::Conflict
    );
}

#[rocket::async_test]
async fn invalid_username_and_short_password_are_400() {
    let dir = tempfile::tempdir().unwrap();
    let client = test_client(dir.path()).await;

    assert_eq!(register(&client, "Bad Name", "password123").await, Status::BadRequest);
    assert_eq!(register(&client, "ok", "password123").await, Status::BadRequest);
    assert_eq!(register(&client, "goodname", "short").await, Status::BadRequest);
}

#[rocket::async_test]
async fn wrong_password_is_401() {
    let dir = tempfile::tempdir().unwrap();
    let client = test_client(dir.path()).await;

    assert_eq!(register(&client, "carol", "password123").await, Status::Ok);
    let resp = client
        .post("/api/v2/auth/login")
        .header(ContentType::JSON)
        .body(r#"{"username":"carol","password":"wrongpassword"}"#)
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Unauthorized);
}

#[rocket::async_test]
async fn revoked_token_is_401() {
    let dir = tempfile::tempdir().unwrap();
    let client = test_client(dir.path()).await;
    let token = auth_token(&client).await;

    let resp = upload(&client, &token, &sample_rom(), r#"{"title":"T","author":"A"}"#).await;
    assert_eq!(resp.status(), Status::Ok);

    let resp = client.get("/api/v2/auth/tokens").dispatch().await;
    let tokens: serde_json::Value =
        serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    let token_id = tokens[0]["id"].as_str().unwrap().to_string();

    let resp = client
        .delete(format!("/api/v2/auth/tokens/{token_id}"))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Ok);

    // Session cookie would still authenticate; check the token alone via a
    // fresh non-tracked request path: logout first, then try the token.
    client.post("/api/v2/auth/logout").dispatch().await;
    let resp = upload(&client, &token, &sample_rom(), r#"{"title":"T","author":"A"}"#).await;
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
            &sample_rom(),
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
    let rom = sample_rom();

    let resp = upload(
        &client,
        &token,
        &rom,
        r#"{"title":"Catch","author":"Andrej","description":"demo","tags":["arcade"]}"#,
    )
    .await;
    assert_eq!(resp.status(), Status::Ok);
    let cart: serde_json::Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(cart["title"], "Catch");
    assert_eq!(cart["rom_size"], rom.len() as i64);
    let id = cart["id"].as_str().unwrap().to_string();

    let resp = client.get(format!("/api/carts/{id}")).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);

    let resp = client.get(format!("/api/carts/{id}/rom")).dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    assert_eq!(resp.into_bytes().await.unwrap(), rom);

    let resp = client.get("/api/carts?q=Catch").dispatch().await;
    assert_eq!(resp.status(), Status::Ok);
    let list: serde_json::Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(list["total"], 1);
}

#[rocket::async_test]
async fn invalid_rom_magic_is_400() {
    let dir = tempfile::tempdir().unwrap();
    let client = test_client(dir.path()).await;
    let token = auth_token(&client).await;

    let resp = upload(&client, &token, b"NOTAROM-BYTES", r#"{"title":"T","author":"A"}"#).await;
    assert_eq!(resp.status(), Status::BadRequest);
}

#[rocket::async_test]
async fn empty_title_is_400() {
    let dir = tempfile::tempdir().unwrap();
    let client = test_client(dir.path()).await;
    let token = auth_token(&client).await;

    let resp = upload(&client, &token, &sample_rom(), r#"{"title":"  ","author":"A"}"#).await;
    assert_eq!(resp.status(), Status::BadRequest);
}

#[rocket::async_test]
async fn oversize_rom_is_413() {
    let dir = tempfile::tempdir().unwrap();
    let client = test_client(dir.path()).await;
    let token = auth_token(&client).await;

    let mut rom = sample_rom();
    rom.resize(1024 * 1024 + 1, 0);
    let resp = upload(&client, &token, &rom, r#"{"title":"T","author":"A"}"#).await;
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
