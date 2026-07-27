use std::path::PathBuf;

use anyhow::Result;
use caiven_port::{PortState, build_rocket};
use clap::Parser;
use migration::MigratorTrait;
use rocket::data::{Limits, ToByteUnit};
use sea_orm::{ConnectOptions, Database};

#[derive(Parser)]
#[command(name = "caiven-port", about = "Caiven cart sharing port")]
struct Args {
    /// Address to listen on
    #[arg(long, default_value = "0.0.0.0")]
    address: std::net::IpAddr,

    /// Port to listen on
    #[arg(long, default_value_t = 8080)]
    port: u16,

    /// PostgreSQL connection string (e.g. postgres://user:pass@host/db). If
    /// unset, falls back to an on-disk SQLite database under `--data-dir`,
    /// for zero-setup local development.
    #[arg(long, env = "DATABASE_URL")]
    database_url: Option<String>,

    /// Directory for the fallback SQLite database, used only when
    /// `--database-url` is not set.
    #[arg(long, default_value = "data")]
    data_dir: PathBuf,

    /// Directory containing the built SPA (`npm run build` output)
    #[arg(long, default_value = "crates/caiven-port/web/dist")]
    web_dir: PathBuf,

    /// Mark authentication cookies Secure. Enable whenever port is served
    /// through HTTPS, including behind a trusted TLS reverse proxy.
    #[arg(long, env = "CAIVEN_SECURE_COOKIES", default_value_t = false)]
    secure_cookies: bool,

    /// Public origin (e.g. `https://port.example.com`), used to build OAuth
    /// redirect URIs and links embedded in emails. Required for OAuth login
    /// and for confirmation/reset links to work outside of local dev.
    #[arg(long, env = "CAIVEN_BASE_URL")]
    base_url: Option<String>,

    /// SMTP host for outbound email. When unset, verification/reset links
    /// are logged to stdout instead of emailed (local dev fallback).
    #[arg(long, env = "SMTP_HOST")]
    smtp_host: Option<String>,
    #[arg(long, env = "SMTP_PORT", default_value_t = 587)]
    smtp_port: u16,
    #[arg(long, env = "SMTP_USERNAME")]
    smtp_username: Option<String>,
    #[arg(long, env = "SMTP_PASSWORD")]
    smtp_password: Option<String>,
    #[arg(long, env = "SMTP_FROM")]
    smtp_from: Option<String>,

    /// Cloudflare Turnstile keys. When unset, antibot verification is
    /// skipped (local dev fallback).
    #[arg(long, env = "TURNSTILE_SITE_KEY")]
    turnstile_site_key: Option<String>,
    #[arg(long, env = "TURNSTILE_SECRET_KEY")]
    turnstile_secret_key: Option<String>,

    /// Social login credentials, one pair per provider. A provider is
    /// enabled only when both its id and secret are set.
    #[arg(long, env = "GOOGLE_CLIENT_ID")]
    google_client_id: Option<String>,
    #[arg(long, env = "GOOGLE_CLIENT_SECRET")]
    google_client_secret: Option<String>,
    #[arg(long, env = "GITHUB_CLIENT_ID")]
    github_client_id: Option<String>,
    #[arg(long, env = "GITHUB_CLIENT_SECRET")]
    github_client_secret: Option<String>,
    #[arg(long, env = "DISCORD_CLIENT_ID")]
    discord_client_id: Option<String>,
    #[arg(long, env = "DISCORD_CLIENT_SECRET")]
    discord_client_secret: Option<String>,
}

fn provider_pair(id: Option<String>, secret: Option<String>) -> Option<caiven_port::oauth::ProviderConfig> {
    match (id, secret) {
        (Some(client_id), Some(client_secret)) => Some(caiven_port::oauth::ProviderConfig {
            client_id,
            client_secret,
        }),
        _ => None,
    }
}

#[rocket::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    env_logger::init();
    let args = Args::parse();
    caiven_port::set_legacy_data_dir(args.data_dir.clone());

    let db_url = match &args.database_url {
        Some(url) => url.clone(),
        None => {
            tokio::fs::create_dir_all(args.data_dir.join("carts")).await?;
            tokio::fs::create_dir_all(args.data_dir.join("screenshots")).await?;
            let db_path = args.data_dir.join("port.db");
            // sqlx's sqlite URL parser rejects backslashes, which
            // `Path::display()` emits on Windows — normalize to forward
            // slashes for the connection string.
            format!(
                "sqlite://{}?mode=rwc",
                db_path.display().to_string().replace('\\', "/")
            )
        }
    };

    let mut opt = ConnectOptions::new(db_url);
    opt.max_connections(10);
    let db = Database::connect(opt).await?;
    migration::Migrator::up(&db, None).await?;

    let limits = Limits::default()
        .limit("data-form", 2.mebibytes())
        .limit("file", 2.mebibytes());

    let config = rocket::Config {
        address: args.address,
        port: args.port,
        limits,
        log_level: rocket::config::LogLevel::Normal,
        ..Default::default()
    };

    let mailer = match (
        &args.smtp_host,
        &args.smtp_username,
        &args.smtp_password,
        &args.smtp_from,
    ) {
        (Some(host), Some(username), Some(password), Some(from)) => {
            let cfg = caiven_port::mailer::SmtpConfig {
                host: host.clone(),
                port: args.smtp_port,
                username: username.clone(),
                password: password.clone(),
                from: from.clone(),
            };
            Some(caiven_port::mailer::Mailer::new(&cfg)?)
        }
        _ => {
            log::warn!(
                "SMTP not fully configured; verification and password reset links will be logged instead of emailed"
            );
            None
        }
    };

    let oauth = caiven_port::OAuthProviders {
        google: provider_pair(args.google_client_id, args.google_client_secret),
        github: provider_pair(args.github_client_id, args.github_client_secret),
        discord: provider_pair(args.discord_client_id, args.discord_client_secret),
    };

    let webauthn = caiven_port::build_webauthn(args.base_url.as_deref());
    if webauthn.is_none() {
        log::warn!("CAIVEN_BASE_URL not set (or unparseable); passkey login is disabled");
    }

    let state = PortState {
        db,
        rate: caiven_port::auth::RateLimiter::default(),
        web_dir: args.web_dir,
        secure_cookies: args.secure_cookies,
        base_url: args.base_url,
        http: reqwest::Client::new(),
        mailer,
        turnstile_site_key: args.turnstile_site_key,
        turnstile_secret: args.turnstile_secret_key,
        oauth,
        webauthn,
    };

    build_rocket(config, state)
        .launch()
        .await
        .map_err(|e| anyhow::anyhow!(e))?;

    Ok(())
}
