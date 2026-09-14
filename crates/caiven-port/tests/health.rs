//! Operational probes must distinguish a running process from a usable DB.
#![allow(clippy::unwrap_used)]

use caiven_port::{PortState, build_rocket};
use rocket::{http::Status, local::asynchronous::Client};
use sea_orm::TransactionTrait;

#[rocket::async_test]
async fn probes_report_database_outage_without_leaking_details() {
    let web = tempfile::tempdir().unwrap();
    let db = sea_orm::Database::connect("sqlite::memory:").await.unwrap();
    let state = PortState::for_testing(db.clone(), web.path().to_path_buf(), false);
    let config = rocket::Config {
        log_level: rocket::config::LogLevel::Off,
        ..rocket::Config::debug_default()
    };
    let client = Client::tracked(build_rocket(config, state)).await.unwrap();
    for path in ["/healthz", "/readyz"] {
        let response = client.get(path).dispatch().await;
        assert_eq!(response.status(), Status::Ok, "{path}");
        assert_eq!(
            response.headers().get_one("Cache-Control"),
            Some("no-store")
        );
        assert_eq!(
            response.into_json::<serde_json::Value>().await.unwrap(),
            serde_json::json!({"status": "ok"})
        );
    }
    db.close().await.unwrap();
    let response = client.get("/readyz").dispatch().await;
    assert_eq!(response.status(), Status::ServiceUnavailable);
    assert_eq!(
        response.headers().get_one("Cache-Control"),
        Some("no-store")
    );
    assert_eq!(
        response.into_json::<serde_json::Value>().await.unwrap(),
        serde_json::json!({"status": "unavailable"})
    );
    assert_eq!(client.get("/healthz").dispatch().await.status(), Status::Ok);
}

#[rocket::async_test]
async fn readiness_deadline_bounds_pool_saturation_and_recovers() {
    let web = tempfile::tempdir().unwrap();
    let mut options = sea_orm::ConnectOptions::new("sqlite::memory:");
    options
        .max_connections(1)
        .acquire_timeout(std::time::Duration::from_secs(10));
    let db = sea_orm::Database::connect(options).await.unwrap();
    let state = PortState::for_testing(db.clone(), web.path().to_path_buf(), false);
    let config = rocket::Config {
        log_level: rocket::config::LogLevel::Off,
        ..rocket::Config::debug_default()
    };
    let client = Client::tracked(build_rocket(config, state)).await.unwrap();
    // Occupy the only connection beyond the probe's two-second budget.
    let transaction = db.begin().await.unwrap();
    let response = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        client.get("/readyz").dispatch(),
    )
    .await
    .expect("readiness waited for the pool's ten-second timeout");
    assert_eq!(response.status(), Status::ServiceUnavailable);
    transaction.rollback().await.unwrap();
    assert_eq!(client.get("/readyz").dispatch().await.status(), Status::Ok);
}
