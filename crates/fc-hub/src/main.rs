use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use fc_hub::{HubState, build_rocket};
use migration::MigratorTrait;
use rocket::data::{Limits, ToByteUnit};
use sea_orm::Database;

#[derive(Parser)]
#[command(name = "fc-hub", about = "Fantasy Console cart sharing hub")]
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

    /// API key required for uploads
    #[arg(long, default_value = "changeme", env = "FC_HUB_API_KEY")]
    api_key: String,
}

#[rocket::main]
async fn main() -> Result<()> {
    env_logger::init();
    let args = Args::parse();

    if args.api_key == "changeme" {
        if cfg!(debug_assertions) {
            eprintln!("WARNING: default API key in use — set --api-key or FC_HUB_API_KEY");
        } else {
            anyhow::bail!(
                "refusing to start with the default API key in a release build — \
                 set --api-key or FC_HUB_API_KEY"
            );
        }
    }

    tokio::fs::create_dir_all(args.data_dir.join("roms")).await?;
    tokio::fs::create_dir_all(args.data_dir.join("screenshots")).await?;

    let db_path = args.data_dir.join("hub.db");
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

    let state = HubState {
        db,
        data_dir: args.data_dir,
        api_key: args.api_key,
    };

    build_rocket(config, state)
        .launch()
        .await
        .map_err(|e| anyhow::anyhow!(e))?;

    Ok(())
}
