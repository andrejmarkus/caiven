//! Minimal OAuth2 authorization-code + PKCE client for social login.
//!
//! No external OAuth crate — just three providers with well-known,
//! stable endpoints, hand-rolled to keep the dependency surface small.

use argon2::password_hash::rand_core::{OsRng, RngCore};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde::Deserialize;
use sha2::{Digest, Sha256};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Provider {
    Google,
    Github,
    Discord,
}

impl Provider {
    pub fn as_str(self) -> &'static str {
        match self {
            Provider::Google => "google",
            Provider::Github => "github",
            Provider::Discord => "discord",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "google" => Some(Provider::Google),
            "github" => Some(Provider::Github),
            "discord" => Some(Provider::Discord),
            _ => None,
        }
    }

    pub const ALL: [Provider; 3] = [Provider::Google, Provider::Github, Provider::Discord];

    fn authorize_url(self) -> &'static str {
        match self {
            Provider::Google => "https://accounts.google.com/o/oauth2/v2/auth",
            Provider::Github => "https://github.com/login/oauth/authorize",
            Provider::Discord => "https://discord.com/api/oauth2/authorize",
        }
    }

    fn token_url(self) -> &'static str {
        match self {
            Provider::Google => "https://oauth2.googleapis.com/token",
            Provider::Github => "https://github.com/login/oauth/access_token",
            Provider::Discord => "https://discord.com/api/oauth2/token",
        }
    }

    fn scope(self) -> &'static str {
        match self {
            Provider::Google => "openid email profile",
            Provider::Github => "read:user user:email",
            Provider::Discord => "identify email",
        }
    }
}

/// Client id/secret for one enabled provider.
#[derive(Clone)]
pub struct ProviderConfig {
    pub client_id: String,
    pub client_secret: String,
}

/// Normalized identity returned by a provider after exchange, regardless of
/// how each one shapes its userinfo response.
pub struct OAuthIdentity {
    pub subject: String,
    pub email: Option<String>,
    pub email_verified: bool,
    pub suggested_username: String,
}

impl OAuthIdentity {
    /// Email eligible for account linking. Provider ownership must be
    /// verified independently of any matching Caiven account.
    pub fn verified_email(&self) -> Option<&str> {
        self.email_verified
            .then_some(self.email.as_deref())
            .flatten()
    }
}

/// Random URL-safe PKCE code verifier (43 chars from 32 random bytes).
pub fn new_code_verifier() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

pub fn code_challenge_s256(verifier: &str) -> String {
    let digest = Sha256::digest(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(digest)
}

pub fn build_authorize_url(
    provider: Provider,
    cfg: &ProviderConfig,
    redirect_uri: &str,
    state: &str,
    code_challenge: &str,
) -> anyhow::Result<String> {
    let mut params = vec![
        ("client_id", cfg.client_id.as_str()),
        ("redirect_uri", redirect_uri),
        ("response_type", "code"),
        ("scope", provider.scope()),
        ("state", state),
        // Discord and GitHub both accept PKCE params even though only Google
        // strictly requires the modern flow; harmless to send everywhere.
        ("code_challenge", code_challenge),
        ("code_challenge_method", "S256"),
    ];
    if provider == Provider::Google {
        params.push(("access_type", "online"));
        params.push(("prompt", "select_account"));
    }
    Ok(reqwest::Url::parse_with_params(provider.authorize_url(), &params)?.into())
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
}

pub async fn exchange_and_fetch(
    client: &reqwest::Client,
    provider: Provider,
    cfg: &ProviderConfig,
    code: &str,
    redirect_uri: &str,
    code_verifier: &str,
) -> anyhow::Result<OAuthIdentity> {
    let params = [
        ("client_id", cfg.client_id.as_str()),
        ("client_secret", cfg.client_secret.as_str()),
        ("code", code),
        ("redirect_uri", redirect_uri),
        ("grant_type", "authorization_code"),
        ("code_verifier", code_verifier),
    ];

    let token: TokenResponse = client
        .post(provider.token_url())
        .header("Accept", "application/json")
        .form(&params)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    fetch_identity(client, provider, &token.access_token).await
}

async fn fetch_identity(
    client: &reqwest::Client,
    provider: Provider,
    access_token: &str,
) -> anyhow::Result<OAuthIdentity> {
    match provider {
        Provider::Google => {
            #[derive(Deserialize)]
            struct GoogleUser {
                sub: String,
                email: Option<String>,
                #[serde(default)]
                email_verified: bool,
                #[serde(default)]
                given_name: Option<String>,
            }
            let user: GoogleUser = client
                .get("https://openidconnect.googleapis.com/v1/userinfo")
                .bearer_auth(access_token)
                .send()
                .await?
                .error_for_status()?
                .json()
                .await?;
            let suggested = user
                .given_name
                .clone()
                .or_else(|| user.email.clone())
                .unwrap_or_else(|| format!("google-{}", &user.sub[..8.min(user.sub.len())]));
            Ok(OAuthIdentity {
                subject: user.sub,
                email: user.email,
                email_verified: user.email_verified,
                suggested_username: suggested,
            })
        }
        Provider::Github => {
            #[derive(Deserialize)]
            struct GithubUser {
                id: i64,
                login: String,
                email: Option<String>,
            }
            #[derive(Deserialize)]
            struct GithubEmail {
                email: String,
                primary: bool,
                verified: bool,
            }
            let user: GithubUser = client
                .get("https://api.github.com/user")
                .bearer_auth(access_token)
                .header("User-Agent", "caiven-port")
                .header("Accept", "application/vnd.github+json")
                .send()
                .await?
                .error_for_status()?
                .json()
                .await?;

            let (email, verified) = if let Some(e) = user.email {
                (Some(e), false)
            } else {
                let emails: Vec<GithubEmail> = client
                    .get("https://api.github.com/user/emails")
                    .bearer_auth(access_token)
                    .header("User-Agent", "caiven-port")
                    .header("Accept", "application/vnd.github+json")
                    .send()
                    .await?
                    .error_for_status()?
                    .json()
                    .await
                    .unwrap_or_default();
                emails
                    .into_iter()
                    .find(|e| e.primary)
                    .map(|e| (Some(e.email), e.verified))
                    .unwrap_or((None, false))
            };

            Ok(OAuthIdentity {
                subject: user.id.to_string(),
                email,
                email_verified: verified,
                suggested_username: user.login,
            })
        }
        Provider::Discord => {
            #[derive(Deserialize)]
            struct DiscordUser {
                id: String,
                username: String,
                email: Option<String>,
                #[serde(default)]
                verified: bool,
            }
            let user: DiscordUser = client
                .get("https://discord.com/api/users/@me")
                .bearer_auth(access_token)
                .send()
                .await?
                .error_for_status()?
                .json()
                .await?;
            Ok(OAuthIdentity {
                subject: user.id,
                email: user.email,
                email_verified: user.verified,
                suggested_username: user.username,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::OAuthIdentity;

    fn identity(email_verified: bool) -> OAuthIdentity {
        OAuthIdentity {
            subject: "provider-user".into(),
            email: Some("user@example.test".into()),
            email_verified,
            suggested_username: "user".into(),
        }
    }

    #[test]
    fn account_link_email_requires_provider_verification() {
        assert_eq!(identity(true).verified_email(), Some("user@example.test"));
        assert_eq!(identity(false).verified_email(), None);
    }

    #[test]
    fn authorize_url_encodes_query_params() {
        let cfg = super::ProviderConfig {
            client_id: "id&x".into(),
            client_secret: String::new(),
        };
        let url = super::build_authorize_url(
            super::Provider::Google,
            &cfg,
            "https://port.test/cb?a=1",
            "st",
            "ch",
        )
        .expect("authorize URL builds");
        assert!(url.starts_with("https://accounts.google.com/o/oauth2/v2/auth?client_id=id%26x&"));
        assert!(url.contains("redirect_uri=https%3A%2F%2Fport.test%2Fcb%3Fa%3D1&"));
        assert!(url.contains("scope=openid+email+profile&"));
        assert!(url.ends_with("&prompt=select_account"));
    }
}
