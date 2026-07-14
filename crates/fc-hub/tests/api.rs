//! API integration tests: in-memory SQLite + rocket local client, exercising
//! auth, upload validation and the download roundtrip.

use fc_hub::{HubState, build_rocket};
use migration::MigratorTrait;
use rocket::data::{Limits, ToByteUnit};
use rocket::http::{ContentType, Header, Status};
use rocket::local::asynchronous::Client;

const API_KEY: &str = "test-key";
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
        api_key: API_KEY.into(),
    };
    Client::tracked(build_rocket(config, state)).await.unwrap()
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
    rom: &[u8],
    meta: &str,
) -> rocket::local::asynchronous::LocalResponse<'c> {
    client
        .post("/api/carts")
        .header(Header::new("X-Api-Key", API_KEY))
        .header(multipart_content_type())
        .body(multipart_body(rom, meta))
        .dispatch()
        .await
}

#[rocket::async_test]
async fn upload_without_api_key_is_401() {
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
    let rom = sample_rom();

    let resp = upload(
        &client,
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

    let resp = upload(&client, b"NOTAROM-BYTES", r#"{"title":"T","author":"A"}"#).await;
    assert_eq!(resp.status(), Status::BadRequest);
}

#[rocket::async_test]
async fn empty_title_is_400() {
    let dir = tempfile::tempdir().unwrap();
    let client = test_client(dir.path()).await;

    let resp = upload(&client, &sample_rom(), r#"{"title":"  ","author":"A"}"#).await;
    assert_eq!(resp.status(), Status::BadRequest);
}

#[rocket::async_test]
async fn oversize_rom_is_413() {
    let dir = tempfile::tempdir().unwrap();
    let client = test_client(dir.path()).await;

    let mut rom = sample_rom();
    rom.resize(1024 * 1024 + 1, 0);
    let resp = upload(&client, &rom, r#"{"title":"T","author":"A"}"#).await;
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
