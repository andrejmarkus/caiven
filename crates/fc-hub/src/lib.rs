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
                handlers::legacy::gallery_page,
                handlers::legacy::list_carts,
                handlers::legacy::get_cart,
                handlers::legacy::upload_cart,
                handlers::legacy::download_rom,
                handlers::legacy::upload_screenshot,
                handlers::legacy::get_screenshot,
                handlers::auth::register,
                handlers::auth::login,
                handlers::auth::logout,
                handlers::auth::me,
                handlers::auth::list_tokens,
                handlers::auth::create_token,
                handlers::auth::revoke_token,
                handlers::carts::list_carts,
                handlers::carts::get_cart,
                handlers::carts::upload_cart,
                handlers::carts::update_cart,
                handlers::carts::delete_cart,
                handlers::versions::create_version,
                handlers::versions::download_rom,
                handlers::versions::upload_screenshot,
                handlers::versions::get_screenshot,
                handlers::discovery::list_tags,
                handlers::discovery::user_profile,
                handlers::social::rate_cart,
                handlers::social::unrate_cart,
                handlers::social::list_comments,
                handlers::social::add_comment,
                handlers::social::delete_comment,
            ],
        )
        .register("/", rocket::catchers![handlers::unauthorized])
}
