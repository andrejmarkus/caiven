//! API integration tests: in-memory SQLite + rocket local client, exercising
//! accounts, tokens, upload validation and the download roundtrip.

#![allow(clippy::unwrap_used)]

use fc_hub::{HubState, auth::RateLimiter, build_rocket};
use migration::MigratorTrait;
use rocket::data::{Limits, ToByteUnit};
use rocket::http::{ContentType, Header, Status};
use rocket::local::asynchronous::Client;
use sea_orm::ConnectionTrait;

const BOUNDARY: &str = "X-FC-HUB-TEST-BOUNDARY";

async fn test_client(data_dir: &std::path::Path) -> Client {
    std::fs::create_dir_all(data_dir.join("roms")).unwrap();
    std::fs::create_dir_all(data_dir.join("screenshots")).unwrap();
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
    let state = HubState {
        db,
        data_dir: data_dir.to_path_buf(),
        rate: RateLimiter::default(),
        web_dir,
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

/// Register a user, mint a token for it, then log out so the client's
/// cookie jar (shared across all these helpers) doesn't leak that user's
/// session into later requests authenticated by a *different* user's token.
async fn register_get_token_and_logout(client: &Client, username: &str) -> String {
    assert_eq!(register(client, username, "password123").await, Status::Ok);
    let resp = client
        .post("/api/v2/auth/tokens")
        .header(ContentType::JSON)
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

    assert_eq!(
        register(&client, "Bad Name", "password123").await,
        Status::BadRequest
    );
    assert_eq!(
        register(&client, "ok", "password123").await,
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

    let resp = upload(
        &client,
        &token,
        &sample_rom(),
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
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Ok);

    // Session cookie would still authenticate; check the token alone via a
    // fresh non-tracked request path: logout first, then try the token.
    client.post("/api/v2/auth/logout").dispatch().await;
    let resp = upload(
        &client,
        &token,
        &sample_rom(),
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
        &sample_rom(),
        r#"{"title":"  ","author":"A"}"#,
    )
    .await;
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
        &sample_rom(),
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

    let rom_v1 = sample_rom();
    let resp = upload(&client, &token, &rom_v1, r#"{"title":"Game","author":"A"}"#).await;
    assert_eq!(resp.status(), Status::Ok);
    let cart: serde_json::Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    let id = cart["id"].as_str().unwrap().to_string();

    let mut rom_v2 = b"SPEAR2".to_vec();
    rom_v2.extend_from_slice(&[1u8; 80]);
    let resp = client
        .post(format!("/api/v2/carts/{id}/versions"))
        .header(Header::new("X-Api-Key", token.clone()))
        .header(multipart_content_type())
        .body(multipart_body(&rom_v2, r#"{"changelog":"fix bug"}"#))
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
        .get(format!("/api/v2/carts/{id}/rom"))
        .dispatch()
        .await;
    assert_eq!(resp.into_bytes().await.unwrap(), rom_v2);

    let resp = client
        .get(format!("/api/v2/carts/{id}/rom?version=1"))
        .dispatch()
        .await;
    assert_eq!(resp.into_bytes().await.unwrap(), rom_v1);

    let resp = client
        .delete(format!("/api/v2/carts/{id}"))
        .header(Header::new("X-Api-Key", token.clone()))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Ok);

    let resp = client
        .get(format!("/api/v2/carts/{id}/rom"))
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
        &sample_rom(),
        r#"{"title":"Alpha","author":"Zed","tags":["Arcade","Retro"]}"#,
    )
    .await;
    assert_eq!(resp.status(), Status::Ok);
    let resp = upload(
        &client,
        &token,
        &sample_rom(),
        r#"{"title":"Beta","author":"Amy","tags":["Puzzle"]}"#,
    )
    .await;
    assert_eq!(resp.status(), Status::Ok);

    let resp = client.get("/api/v2/carts?tag=retro").dispatch().await;
    let list: serde_json::Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(list["total"], 1);
    assert_eq!(list["carts"][0]["title"], "Alpha");

    let resp = client.get("/api/v2/carts?author=Amy").dispatch().await;
    let list: serde_json::Value = serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    assert_eq!(list["total"], 1);
    assert_eq!(list["carts"][0]["title"], "Beta");

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
        &sample_rom(),
        r#"{"title":"Quiet","author":"A"}"#,
    )
    .await;
    let quiet: serde_json::Value =
        serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();
    let resp = upload(
        &client,
        &token,
        &sample_rom(),
        r#"{"title":"Popular","author":"A"}"#,
    )
    .await;
    let popular: serde_json::Value =
        serde_json::from_str(&resp.into_string().await.unwrap()).unwrap();

    for _ in 0..3 {
        client
            .get(format!(
                "/api/v2/carts/{}/rom",
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
        &sample_rom(),
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
        &sample_rom(),
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
        &sample_rom(),
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

    let cart = fc_hub::db::get(&db, "11111111-1111-1111-1111-111111111111")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(cart.owner.as_deref(), Some("legacy"));
    assert_eq!(cart.latest_version, 1);
    assert_eq!(cart.rom_size, 512);
    assert!(cart.has_screenshot);
}
