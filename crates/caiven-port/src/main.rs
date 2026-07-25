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
}

#[rocket::main]
async fn main() -> Result<()> {
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

    let state = PortState {
        db,
        rate: caiven_port::auth::RateLimiter::default(),
        web_dir: args.web_dir,
        secure_cookies: args.secure_cookies,
    };

    build_rocket(config, state)
        .launch()
        .await
        .map_err(|e| anyhow::anyhow!(e))?;

    Ok(())
}
