//! Operator identity for the legal pages and the EU DSA Art. 16
//! notice-and-action endpoint: anyone can report content they believe is
//! illegal or breaks the Terms, and the report is mailed to the operator.

use std::time::Duration;

use rocket::{State, get, http::Status, post, serde::json::Json};
use serde::Deserialize;

use crate::{LegalInfo, PortState, auth, auth::ClientIp, error::ApiError, turnstile};

const REPORT_LIMIT: u32 = 5;
const REPORT_WINDOW: Duration = Duration::from_secs(3600);
const REPORT_CATEGORIES: &[&str] = &[
    "copyright",
    "illegal",
    "harassment",
    "child_safety",
    "other",
];

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct ReportInput {
    pub url: String,
    pub category: String,
    pub explanation: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub email: String,
    pub good_faith: bool,
    #[serde(default)]
    pub turnstile_token: Option<String>,
}

#[get("/api/v1/legal")]
pub fn legal_info(state: &State<PortState>) -> Json<LegalInfo> {
    Json(state.legal.clone())
}

fn one_line(s: &str, max: usize) -> Result<String, ApiError> {
    let s = s.trim();
    if s.chars().count() > max || s.chars().any(char::is_control) {
        return Err(ApiError::bad_request(format!(
            "fields must be one line, at most {max} chars"
        )));
    }
    Ok(s.to_string())
}

#[post("/api/v1/reports", data = "<input>")]
pub async fn submit_report(
    state: &State<PortState>,
    ip: ClientIp,
    input: Json<ReportInput>,
) -> Result<Status, ApiError> {
    if state.rate.hit("report", &ip.0, REPORT_WINDOW) > REPORT_LIMIT {
        return Err(ApiError::TooManyRequests("try again later".into()));
    }
    let url = one_line(&input.url, 500)?;
    let name = one_line(&input.name, 100)?;
    let email = one_line(&input.email, 254)?;
    let explanation = input.explanation.trim();
    if url.is_empty() {
        return Err(ApiError::bad_request("link to the content is required"));
    }
    if !REPORT_CATEGORIES.contains(&input.category.as_str()) {
        return Err(ApiError::bad_request("unknown category"));
    }
    if explanation.chars().count() < 10 || explanation.chars().count() > 5000 {
        return Err(ApiError::bad_request("explanation must be 10-5000 chars"));
    }
    if !email.is_empty() && !auth::is_valid_email(&email) {
        return Err(ApiError::bad_request("invalid email"));
    }
    if !input.good_faith {
        return Err(ApiError::bad_request("good-faith statement is required"));
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
    let Some(inbox) = state.legal.contact_email.as_deref() else {
        return Err(ApiError::Internal("reporting is not configured".into()));
    };

    let body = format!(
        "Content: {url}\nCategory: {}\nReporter: {}\nReporter email: {}\nGood-faith statement: confirmed\n\n{explanation}",
        input.category,
        if name.is_empty() {
            "(anonymous)"
        } else {
            &name
        },
        if email.is_empty() { "(none)" } else { &email },
    );
    let subject = format!("Content report ({}): {url}", input.category);
    match state.mailer.as_ref() {
        // The email is the only record of the notice, so a send failure must surface.
        Some(m) => m
            .send_security_alert(inbox, &subject, &body)
            .await
            .map_err(|e| ApiError::Internal(format!("could not deliver report: {e}")))?,
        None => log::info!("[dev] content report for {inbox}: {subject}\n{body}"),
    }
    if !email.is_empty() {
        crate::mailer::send_or_log_alert(
            state.mailer.as_ref(),
            &email,
            "We received your report",
            &format!(
                "Thanks — we received your report about {url}. We review every report and will email you our decision."
            ),
        )
        .await;
    }
    Ok(Status::NoContent)
}
