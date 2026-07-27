//! Caiven cart sharing port — library crate.
//!
//! The binary in `main.rs` wires CLI args, the database and the data
//! directory into [`PortState`] and launches [`build_rocket`]. Tests build the
//! same rocket against an in-memory database.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use rocket::{
    Request, Response,
    fairing::{Fairing, Info, Kind},
    http::Header,
};

pub mod auth;
pub mod db;
pub mod entities;
pub mod error;
pub mod handlers;
pub mod mailer;
pub mod models;
pub mod oauth;
pub mod turnstile;

static LEGACY_DATA_DIR: OnceLock<PathBuf> = OnceLock::new();

/// Configure the on-disk data directory used only as a compatibility fallback
/// for SQLite installations upgraded from path-backed cartridge storage.
pub fn set_legacy_data_dir(path: PathBuf) {
    let _ = LEGACY_DATA_DIR.set(path);
}

pub(crate) fn legacy_data_dir() -> Option<&'static Path> {
    LEGACY_DATA_DIR.get().map(PathBuf::as_path)
}

/// Enabled OAuth social-login providers, one slot per supported provider.
#[derive(Default, Clone)]
pub struct OAuthProviders {
    pub google: Option<oauth::ProviderConfig>,
    pub github: Option<oauth::ProviderConfig>,
    pub discord: Option<oauth::ProviderConfig>,
}

impl OAuthProviders {
    pub fn get(&self, provider: oauth::Provider) -> Option<&oauth::ProviderConfig> {
        match provider {
            oauth::Provider::Google => self.google.as_ref(),
            oauth::Provider::Github => self.github.as_ref(),
            oauth::Provider::Discord => self.discord.as_ref(),
        }
    }

    pub fn enabled(&self) -> Vec<oauth::Provider> {
        oauth::Provider::ALL
            .into_iter()
            .filter(|p| self.get(*p).is_some())
            .collect()
    }
}

pub struct PortState {
    pub db: sea_orm::DatabaseConnection,
    pub rate: auth::RateLimiter,
    pub web_dir: PathBuf,
    pub secure_cookies: bool,
    /// Public origin (e.g. `https://port.caiven.dev`), used to build OAuth
    /// redirect URIs and links embedded in emails. Without it, OAuth and
    /// outbound email links are disabled.
    pub base_url: Option<String>,
    pub http: reqwest::Client,
    pub mailer: Option<mailer::Mailer>,
    pub turnstile_site_key: Option<String>,
    pub turnstile_secret: Option<String>,
    pub oauth: OAuthProviders,
    /// Passkey (WebAuthn) support. `None` when `base_url` is unset, since a
    /// relying-party origin is mandatory to configure it at all — matches
    /// how OAuth degrades gracefully without one.
    pub webauthn: Option<webauthn_rs::Webauthn>,
}

impl PortState {
    /// Minimal state for tests: no email, no Turnstile, no OAuth — matches
    /// production behavior with those env vars unset.
    pub fn for_testing(
        db: sea_orm::DatabaseConnection,
        web_dir: PathBuf,
        secure_cookies: bool,
    ) -> Self {
        PortState {
            db,
            rate: auth::RateLimiter::default(),
            web_dir,
            secure_cookies,
            base_url: None,
            http: reqwest::Client::new(),
            mailer: None,
            turnstile_site_key: None,
            turnstile_secret: None,
            oauth: OAuthProviders::default(),
            webauthn: None,
        }
    }
}

/// Builds the WebAuthn relying-party config from the public base URL.
/// Returns `None` (rather than erroring) when unset or unparseable, so
/// passkeys just don't show up instead of failing startup.
pub fn build_webauthn(base_url: Option<&str>) -> Option<webauthn_rs::Webauthn> {
    let base_url = base_url?;
    let origin = webauthn_rs::prelude::Url::parse(base_url).ok()?;
    let rp_id = origin.domain()?;
    webauthn_rs::WebauthnBuilder::new(rp_id, &origin)
        .ok()?
        .rp_name("Caiven Port")
        .build()
        .ok()
}

struct AuthNoStore;

#[rocket::async_trait]
impl Fairing for AuthNoStore {
    fn info(&self) -> Info {
        Info {
            name: "Disable caching for authentication responses",
            kind: Kind::Response,
        }
    }

    async fn on_response<'r>(&self, request: &'r Request<'_>, response: &mut Response<'r>) {
        if request.uri().path().as_str().starts_with("/api/v2/auth/") {
            response.set_header(Header::new("Cache-Control", "no-store"));
        }
    }
}

/// Baseline security response headers on every response. `script-src` has
/// no `unsafe-inline` so a reflected/stored-XSS bug can't execute inline
/// script; `style-src` allows it pragmatically for Tailwind/shadcn's
/// inline-styled components. `challenges.cloudflare.com` is allow-listed
/// for the Turnstile widget (script + frame + XHR).
struct SecurityHeaders;

#[rocket::async_trait]
impl Fairing for SecurityHeaders {
    fn info(&self) -> Info {
        Info {
            name: "Security response headers",
            kind: Kind::Response,
        }
    }

    async fn on_response<'r>(&self, _request: &'r Request<'_>, response: &mut Response<'r>) {
        response.set_header(Header::new("X-Content-Type-Options", "nosniff"));
        response.set_header(Header::new("X-Frame-Options", "DENY"));
        response.set_header(Header::new(
            "Referrer-Policy",
            "strict-origin-when-cross-origin",
        ));
        response.set_header(Header::new(
            "Permissions-Policy",
            "camera=(), microphone=(), geolocation=()",
        ));
        response.set_header(Header::new(
            "Content-Security-Policy",
            "default-src 'self'; \
             script-src 'self' https://challenges.cloudflare.com; \
             style-src 'self' 'unsafe-inline'; \
             img-src 'self' data: blob:; \
             connect-src 'self' https://challenges.cloudflare.com; \
             frame-src https://challenges.cloudflare.com; \
             base-uri 'self'; \
             form-action 'self'",
        ));
    }
}

/// Assemble the rocket with all routes and catchers mounted.
pub fn build_rocket(config: rocket::Config, state: PortState) -> rocket::Rocket<rocket::Build> {
    let web_dir = state.web_dir.clone();
    rocket::custom(config)
        .manage(state)
        .attach(AuthNoStore)
        .attach(SecurityHeaders)
        .mount("/", rocket::fs::FileServer::from(web_dir).rank(15))
        .mount(
            "/",
            rocket::routes![
                handlers::legacy::list_carts,
                handlers::legacy::get_cart,
                handlers::legacy::upload_cart,
                handlers::legacy::download_cart,
                handlers::legacy::upload_screenshot,
                handlers::legacy::get_screenshot,
                handlers::auth::register,
                handlers::auth::login,
                handlers::auth::logout,
                handlers::auth::me,
                handlers::auth::change_password,
                handlers::auth::list_sessions,
                handlers::auth::revoke_session,
                handlers::auth::revoke_all_sessions,
                handlers::auth::list_tokens,
                handlers::auth::create_token,
                handlers::auth::revoke_token,
                handlers::auth::auth_config,
                handlers::auth::verify_email,
                handlers::auth::resend_verification,
                handlers::auth::forgot_password,
                handlers::auth::reset_password,
                handlers::auth::oauth_start,
                handlers::auth::oauth_callback,
                handlers::auth::login_mfa,
                handlers::auth::set_password,
                handlers::auth::mfa_status,
                handlers::auth::mfa_setup,
                handlers::auth::mfa_confirm,
                handlers::auth::mfa_disable,
                handlers::auth::webauthn_register_start,
                handlers::auth::webauthn_register_finish,
                handlers::auth::webauthn_login_start,
                handlers::auth::webauthn_login_finish,
                handlers::auth::list_passkeys,
                handlers::auth::delete_passkey,
                handlers::auth::audit_log,
                handlers::auth::delete_account,
                handlers::auth::export_data,
                handlers::carts::list_carts,
                handlers::carts::get_cart,
                handlers::carts::upload_cart,
                handlers::carts::update_cart,
                handlers::carts::delete_cart,
                handlers::versions::create_version,
                handlers::versions::download_cart,
                handlers::versions::upload_screenshot,
                handlers::versions::get_screenshot,
                handlers::discovery::list_tags,
                handlers::discovery::user_profile,
                handlers::social::rate_cart,
                handlers::social::unrate_cart,
                handlers::social::list_comments,
                handlers::social::add_comment,
                handlers::social::delete_comment,
                handlers::community::record_play,
                handlers::community::follow_user,
                handlers::community::unfollow_user,
                handlers::community::list_collections,
                handlers::community::get_collection,
                handlers::community::create_collection,
                handlers::community::create_editorial_collection,
                handlers::community::update_collection,
                handlers::community::delete_collection,
                handlers::community::add_collection_cart,
                handlers::community::remove_collection_cart,
                handlers::community::reorder_collection,
                handlers::community::follow_collection,
                handlers::community::unfollow_collection,
                handlers::community::list_jams,
                handlers::community::get_jam,
                handlers::community::create_jam,
                handlers::community::update_jam,
                handlers::community::enter_jam,
                handlers::community::withdraw_jam_entry,
                handlers::community::feed,
                handlers::community::dashboard,
                handlers::spa::fallback,
            ],
        )
        .register("/", rocket::catchers![handlers::unauthorized])
}
