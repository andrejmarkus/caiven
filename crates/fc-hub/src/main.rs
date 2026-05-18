use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use rocket::data::{Limits, ToByteUnit};
use sea_orm::Database;
use migration::MigratorTrait;

mod db;
mod entities;
mod error;
mod gallery;
mod handlers;
mod models;

pub struct HubState {
    pub db: sea_orm::DatabaseConnection,
    pub data_dir: PathBuf,
    pub api_key: String,
}

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
        eprintln!("WARNING: default API key in use — set --api-key or FC_HUB_API_KEY");
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

    rocket::custom(config)
        .manage(state)
        .mount(
            "/",
            rocket::routes![
                handlers::gallery_page,
                handlers::list_carts,
                handlers::get_cart,
                handlers::upload_cart,
                handlers::download_rom,
                handlers::upload_screenshot,
                handlers::get_screenshot,
            ],
        )
        .register("/", rocket::catchers![handlers::unauthorized])
        .launch()
        .await
        .map_err(|e| anyhow::anyhow!(e))?;

    Ok(())
}
