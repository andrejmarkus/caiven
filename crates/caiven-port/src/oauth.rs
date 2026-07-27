//! Minimal OAuth2 authorization-code + PKCE client for social login.
//!
//! No external OAuth crate — just three providers with well-known,
//! stable endpoints, hand-rolled to keep the dependency surface small.

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

/// Random URL-safe PKCE code verifier (43 chars from 32 random bytes).
pub fn new_code_verifier() -> String {
    let mut bytes = [0u8; 32];
    use rand_core_compat::fill_random;
    fill_random(&mut bytes);
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
) -> String {
    let mut url = url_lite::Builder::new(provider.authorize_url());
    url.push("client_id", &cfg.client_id);
    url.push("redirect_uri", redirect_uri);
    url.push("response_type", "code");
    url.push("scope", provider.scope());
    url.push("state", state);
    // Discord and GitHub both accept PKCE params even though only Google
    // strictly requires the modern flow; harmless to send everywhere.
    url.push("code_challenge", code_challenge);
    url.push("code_challenge_method", "S256");
    if provider == Provider::Google {
        url.push("access_type", "online");
        url.push("prompt", "select_account");
    }
    url.finish()
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

/// Tiny query-string builder so we don't pull in `url` just for this.
mod url_lite {
    pub struct Builder {
        base: String,
        first: bool,
    }

    impl Builder {
        pub fn new(base: &str) -> Self {
            Builder {
                base: base.to_string(),
                first: true,
            }
        }

        pub fn push(&mut self, key: &str, value: &str) {
            self.base.push(if self.first { '?' } else { '&' });
            self.first = false;
            self.base.push_str(key);
            self.base.push('=');
            self.base
                .push_str(&percent_encode(value));
        }

        pub fn finish(self) -> String {
            self.base
        }
    }

    fn percent_encode(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        for b in s.bytes() {
            match b {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    out.push(b as char)
                }
                _ => out.push_str(&format!("%{b:02X}")),
            }
        }
        out
    }
}

/// Thin wrapper so `new_code_verifier` doesn't need `argon2`'s rand_core
/// re-export threaded through the module signature.
mod rand_core_compat {
    use argon2::password_hash::rand_core::{OsRng, RngCore};

    pub fn fill_random(bytes: &mut [u8]) {
        OsRng.fill_bytes(bytes);
    }
}
