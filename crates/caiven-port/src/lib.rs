//! Caiven cart sharing port — library crate.
//!
//! The binary in `main.rs` wires CLI args, the database and the data
//! directory into [`PortState`] and launches [`build_rocket`]. Tests build the
//! same rocket against an in-memory database.

use std::path::PathBuf;

pub mod auth;
pub mod db;
pub mod entities;
pub mod error;
pub mod handlers;
pub mod models;

pub struct PortState {
    pub db: sea_orm::DatabaseConnection,
    pub data_dir: PathBuf,
    pub rate: auth::RateLimiter,
    pub web_dir: PathBuf,
}

/// Assemble the rocket with all routes and catchers mounted.
pub fn build_rocket(config: rocket::Config, state: PortState) -> rocket::Rocket<rocket::Build> {
    let web_dir = state.web_dir.clone();
    rocket::custom(config)
        .manage(state)
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
                handlers::auth::list_tokens,
                handlers::auth::create_token,
                handlers::auth::revoke_token,
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
                handlers::spa::fallback,
            ],
        )
        .register("/", rocket::catchers![handlers::unauthorized])
}
