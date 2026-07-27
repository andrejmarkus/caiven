//! Cloudflare Turnstile antibot verification.
//!
//! When no secret key is configured (local dev / self-hosted without
//! Turnstile), verification is skipped and always succeeds.

use serde::Deserialize;

const SITEVERIFY_URL: &str = "https://challenges.cloudflare.com/turnstile/v0/siteverify";

#[derive(Deserialize)]
struct SiteverifyResponse {
    success: bool,
}

/// Verifies a Turnstile response token. Returns `true` (allow) when no
/// secret is configured, or when the token was accepted by Cloudflare.
pub async fn verify(
    client: &reqwest::Client,
    secret: Option<&str>,
    token: &str,
    remote_ip: &str,
) -> bool {
    let Some(secret) = secret else {
        return true;
    };
    if token.is_empty() {
        return false;
    }

    let form = [
        ("secret", secret),
        ("response", token),
        ("remoteip", remote_ip),
    ];
    let result = client.post(SITEVERIFY_URL).form(&form).send().await;
    match result {
        Ok(resp) => resp
            .json::<SiteverifyResponse>()
            .await
            .map(|r| r.success)
            .unwrap_or(false),
        Err(e) => {
            log::warn!("turnstile siteverify request failed: {e}");
            false
        }
    }
}
