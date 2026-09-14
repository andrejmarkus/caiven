//! Minimal, uncached probes for supervisors and reverse proxies.

use std::time::Duration;

use rocket::{State, get, http::Status, response::status::Custom, serde::json::Json};
use serde::Serialize;

use crate::PortState;

#[derive(Serialize)]
pub struct Health {
    status: &'static str,
}

#[get("/healthz")]
pub fn live() -> Json<Health> {
    Json(Health { status: "ok" })
}

#[get("/readyz")]
pub async fn ready(state: &State<PortState>) -> Custom<Json<Health>> {
    // A saturated pool or stalled database must not hold a probe indefinitely.
    match tokio::time::timeout(Duration::from_secs(2), state.db.ping()).await {
        Ok(Ok(())) => Custom(Status::Ok, Json(Health { status: "ok" })),
        _ => Custom(
            Status::ServiceUnavailable,
            Json(Health {
                status: "unavailable",
            }),
        ),
    }
}
