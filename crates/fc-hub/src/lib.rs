//! Fantasy Console cart sharing hub — library crate.
//!
//! The binary in `main.rs` wires CLI args, the database and the data
//! directory into [`HubState`] and launches [`build_rocket`]. Tests build the
//! same rocket against an in-memory database.

use std::path::PathBuf;

pub mod auth;
pub mod db;
pub mod entities;
pub mod error;
pub mod gallery;
pub mod handlers;
pub mod models;

pub struct HubState {
    pub db: sea_orm::DatabaseConnection,
    pub data_dir: PathBuf,
    pub rate: auth::RateLimiter,
}

/// Assemble the rocket with all routes and catchers mounted.
pub fn build_rocket(config: rocket::Config, state: HubState) -> rocket::Rocket<rocket::Build> {
    rocket::custom(config)
        .manage(state)
        .mount(
            "/",
            rocket::routes![
                handlers::carts::gallery_page,
                handlers::carts::list_carts,
                handlers::carts::get_cart,
                handlers::carts::upload_cart,
                handlers::carts::download_rom,
                handlers::carts::upload_screenshot,
                handlers::carts::get_screenshot,
                handlers::auth::register,
                handlers::auth::login,
                handlers::auth::logout,
                handlers::auth::me,
                handlers::auth::list_tokens,
                handlers::auth::create_token,
                handlers::auth::revoke_token,
            ],
        )
        .register("/", rocket::catchers![handlers::unauthorized])
}
