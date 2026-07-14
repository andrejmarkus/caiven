//! Fantasy Console cart sharing hub — library crate.
//!
//! The binary in `main.rs` wires CLI args, the database and the data
//! directory into [`HubState`] and launches [`build_rocket`]. Tests build the
//! same rocket against an in-memory database.

use std::path::PathBuf;

pub mod db;
pub mod entities;
pub mod error;
pub mod gallery;
pub mod handlers;
pub mod models;

pub struct HubState {
    pub db: sea_orm::DatabaseConnection,
    pub data_dir: PathBuf,
    pub api_key: String,
}

/// Assemble the rocket with all routes and catchers mounted.
pub fn build_rocket(config: rocket::Config, state: HubState) -> rocket::Rocket<rocket::Build> {
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
}
