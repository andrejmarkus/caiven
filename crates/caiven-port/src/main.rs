use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use caiven_port::{PortState, build_rocket};
use migration::MigratorTrait;
use rocket::data::{Limits, ToByteUnit};
use sea_orm::Database;

#[derive(Parser)]
#[command(name = "caiven-port", about = "Caiven cart sharing port")]
struct Args {
    /// Address to listen on
    #[arg(long, default_value = "0.0.0.0")]
    address: std::net::IpAddr,

    /// Port to listen on
    #[arg(long, default_value_t = 8080)]
    port: u16,

    /// Directory for database and uploaded files
    #[arg(long, default_value = "data")]
    data_dir: PathBuf,

    /// Directory containing the built SPA (`npm run build` output)
    #[arg(long, default_value = "crates/caiven-port/web/dist")]
    web_dir: PathBuf,
}

#[rocket::main]
async fn main() -> Result<()> {
    env_logger::init();
    let args = Args::parse();

    tokio::fs::create_dir_all(args.data_dir.join("carts")).await?;
    tokio::fs::create_dir_all(args.data_dir.join("screenshots")).await?;

    let db_path = args.data_dir.join("port.db");
    let db_url = format!("sqlite://{}?mode=rwc", db_path.display());
    let db = Database::connect(&db_url).await?;
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
        data_dir: args.data_dir,
        rate: caiven_port::auth::RateLimiter::default(),
        web_dir: args.web_dir,
    };

    build_rocket(config, state)
        .launch()
        .await
        .map_err(|e| anyhow::anyhow!(e))?;

    Ok(())
}
