//! Pre-v2 routes (`/`, `/api/carts*`), kept as thin wrappers over the v2 db
//! and handler logic so existing `caiven-studio publish` CLI/Caiven Studio builds keep
//! working unchanged (same paths, same per-user `X-Api-Key` header).

use rocket::{State, form::Form, get, post, serde::json::Json};

use super::carts::{CartUpload, create_cart_impl};
use super::versions::{
    ScreenshotUpload, download_cart_impl, get_screenshot_impl, upload_screenshot_impl,
};
use super::{BinaryFile, valid_id};
use crate::{
    PortState,
    auth::AuthUser,
    db,
    error::ApiError,
    models::{Cart, CartList},
};

#[get("/api/carts?<page>&<per_page>&<q>")]
pub async fn list_carts(
    state: &State<PortState>,
    page: Option<u32>,
    per_page: Option<u32>,
    q: Option<String>,
) -> Result<Json<CartList>, ApiError> {
    let page = page.unwrap_or(0);
    let per_page = per_page.unwrap_or(20).min(100);
    let (carts, total) = db::list(
        &state.db,
        page,
        per_page,
        q.as_deref(),
        None,
        None,
        db::Sort::New,
    )
    .await?;
    Ok(Json(CartList {
        carts,
        total,
        page,
        per_page,
    }))
}

#[get("/api/carts/<id>")]
pub async fn get_cart(state: &State<PortState>, id: &str) -> Result<Json<Cart>, ApiError> {
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
    state: &State<PortState>,
    upload: Form<CartUpload<'_>>,
) -> Result<Json<Cart>, ApiError> {
    Ok(Json(create_cart_impl(state, &user, upload).await?))
}

#[get("/api/carts/<id>/cart")]
pub async fn download_cart(state: &State<PortState>, id: &str) -> Result<BinaryFile, ApiError> {
    download_cart_impl(state, id, None).await
}

#[post("/api/carts/<id>/screenshot", data = "<upload>")]
pub async fn upload_screenshot(
    user: AuthUser,
    state: &State<PortState>,
    id: &str,
    upload: Form<ScreenshotUpload<'_>>,
) -> Result<(), ApiError> {
    upload_screenshot_impl(state, &user, id, None, upload).await
}

#[get("/api/carts/<id>/screenshot")]
pub async fn get_screenshot(state: &State<PortState>, id: &str) -> Result<BinaryFile, ApiError> {
    get_screenshot_impl(state, id, None).await
}
