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
pub mod models;

static LEGACY_DATA_DIR: OnceLock<PathBuf> = OnceLock::new();

/// Configure the on-disk data directory used only as a compatibility fallback
/// for SQLite installations upgraded from path-backed cartridge storage.
pub fn set_legacy_data_dir(path: PathBuf) {
    let _ = LEGACY_DATA_DIR.set(path);
}

pub(crate) fn legacy_data_dir() -> Option<&'static Path> {
    LEGACY_DATA_DIR.get().map(PathBuf::as_path)
}

pub struct PortState {
    pub db: sea_orm::DatabaseConnection,
    pub rate: auth::RateLimiter,
    pub web_dir: PathBuf,
    pub secure_cookies: bool,
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

/// Assemble the rocket with all routes and catchers mounted.
pub fn build_rocket(config: rocket::Config, state: PortState) -> rocket::Rocket<rocket::Build> {
    let web_dir = state.web_dir.clone();
    rocket::custom(config)
        .manage(state)
        .attach(AuthNoStore)
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
