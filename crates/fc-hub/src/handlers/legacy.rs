//! Pre-v2 routes (`/`, `/api/carts*`), kept as thin wrappers over the v2 db
//! and handler logic so existing `fc-engine publish` CLI/Studio builds keep
//! working unchanged (same paths, same per-user `X-Api-Key` header).

use rocket::{
    State, form::Form, get, post, response::content::RawHtml, serde::json::Json,
};

use super::{BinaryFile, valid_id};
use super::carts::{CartUpload, create_cart_impl};
use super::versions::{ScreenshotUpload, download_rom_impl, get_screenshot_impl, upload_screenshot_impl};
use crate::{HubState, auth::AuthUser, db, error::ApiError, gallery, models::{Cart, CartList}};

#[get("/?<page>&<q>")]
pub async fn gallery_page(
    state: &State<HubState>,
    page: Option<u32>,
    q: Option<String>,
) -> RawHtml<String> {
    let p = page.unwrap_or(0);
    let (carts, total) = db::list(&state.db, p, 24, q.as_deref(), None, None, db::Sort::New)
        .await
        .inspect_err(|e| log::error!("gallery DB error: {e}"))
        .unwrap_or_default();
    RawHtml(gallery::render(&carts, total, p, 24, q.as_deref()))
}

#[get("/api/carts?<page>&<per_page>&<q>")]
pub async fn list_carts(
    state: &State<HubState>,
    page: Option<u32>,
    per_page: Option<u32>,
    q: Option<String>,
) -> Result<Json<CartList>, ApiError> {
    let page = page.unwrap_or(0);
    let per_page = per_page.unwrap_or(20).min(100);
    let (carts, total) = db::list(&state.db, page, per_page, q.as_deref(), None, None, db::Sort::New).await?;
    Ok(Json(CartList {
        carts,
        total,
        page,
        per_page,
    }))
}

#[get("/api/carts/<id>")]
pub async fn get_cart(state: &State<HubState>, id: &str) -> Result<Json<Cart>, ApiError> {
    if !valid_id(id) {
        return Err(ApiError::bad_request("invalid id"));
    }
    db::get(&state.db, id)
        .await?
        .map(Json)
        .ok_or_else(|| ApiError::not_found("cart not found"))
}

#[post("/api/carts", data = "<upload>")]
pub async fn upload_cart(
    user: AuthUser,
    state: &State<HubState>,
    upload: Form<CartUpload<'_>>,
) -> Result<Json<Cart>, ApiError> {
    Ok(Json(create_cart_impl(state, &user, upload).await?))
}

#[get("/api/carts/<id>/rom")]
pub async fn download_rom(state: &State<HubState>, id: &str) -> Result<BinaryFile, ApiError> {
    download_rom_impl(state, id, None).await
}

#[post("/api/carts/<id>/screenshot", data = "<upload>")]
pub async fn upload_screenshot(
    user: AuthUser,
    state: &State<HubState>,
    id: &str,
    upload: Form<ScreenshotUpload<'_>>,
) -> Result<(), ApiError> {
    upload_screenshot_impl(state, &user, id, None, upload).await
}

#[get("/api/carts/<id>/screenshot")]
pub async fn get_screenshot(state: &State<HubState>, id: &str) -> Result<BinaryFile, ApiError> {
    get_screenshot_impl(state, id, None).await
}
