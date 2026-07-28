use caiven_cart::MAX_CART_BYTES;
use rocket::{
    FromForm, State, data::Capped, delete, form::Form, fs::TempFile, get, patch, post,
    serde::json::Json,
};
use uuid::Uuid;

use super::valid_id;
use crate::{
    PortState,
    auth::{AuthUser, VerifiedUser},
    db,
    entities::carts,
    error::ApiError,
    models::{Cart, CartDetail, CartList, CartMeta, CartPatch, CartVersionInfo},
};

pub(crate) fn validate_meta(meta: &CartMeta) -> Result<(), ApiError> {
    if meta.title.trim().is_empty() {
        return Err(ApiError::bad_request("title required"));
    }
    if meta.title.len() > 64 {
        return Err(ApiError::bad_request("title max 64 chars"));
    }
    if meta.description.len() > 512 {
        return Err(ApiError::bad_request("description max 512 chars"));
    }
    Ok(())
}

pub(crate) fn require_owner(user: &AuthUser, cart: &carts::Model) -> Result<(), ApiError> {
    if cart.owner_id.as_deref() == Some(user.id.as_str()) || user.is_admin {
        Ok(())
    } else {
        Err(ApiError::forbidden("not the owner of this cart"))
    }
}

#[derive(FromForm)]
pub struct CartUpload<'v> {
    pub cart: Capped<TempFile<'v>>,
    pub meta: String,
}

/// Shared multipart cart+meta validation, used by both the `/api/v2/carts`
/// and legacy `/api/carts` create routes.
pub(crate) async fn create_cart_impl(
    state: &PortState,
    user: &AuthUser,
    upload: Form<CartUpload<'_>>,
) -> Result<Cart, ApiError> {
    if !upload.cart.is_complete() {
        return Err(cart_too_large());
    }
    let cart_len = upload.cart.n.written as usize;
    if cart_len > MAX_CART_BYTES {
        return Err(cart_too_large());
    }

    let tmp_path = upload
        .cart
        .value
        .path()
        .ok_or_else(|| ApiError::internal("temp file unavailable"))?;

    let bytes = tokio::fs::read(tmp_path)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    if bytes.len() < 6 {
        return Err(ApiError::bad_request("cart too small"));
    }
    if &bytes[..6] != b"CAIVEN" {
        return Err(ApiError::bad_request("not a valid Caiven cart"));
    }

    let meta: CartMeta = serde_json::from_str(&upload.meta)?;
    validate_meta(&meta)?;

    let content_hash = caiven_cart::content_hash(&bytes)
        .map_err(|_| ApiError::bad_request("not a valid Caiven cart"))?;
    if let Some((title, author)) =
        db::find_other_owner_by_content_hash(&state.db, &content_hash, &user.id).await?
    {
        return Err(ApiError::conflict(format!(
            "This cart's content matches an existing published cart \"{title}\" by {author}"
        )));
    }

    let id = Uuid::new_v4().to_string();
    db::insert_cart(
        &state.db,
        &user.id,
        &user.username,
        &id,
        &meta,
        &bytes,
        Some(&content_hash),
    )
    .await?;
    db::get(&state.db, &id)
        .await?
        .ok_or_else(|| ApiError::internal("insert failed"))
}

pub(crate) fn cart_too_large() -> ApiError {
    ApiError::PayloadTooLarge(format!("cart max {} KiB", MAX_CART_BYTES / 1024))
}

#[get("/api/v2/carts?<page>&<per_page>&<q>&<tag>&<author>&<sort>")]
#[allow(clippy::too_many_arguments)]
pub async fn list_carts(
    state: &State<PortState>,
    page: Option<u32>,
    per_page: Option<u32>,
    q: Option<String>,
    tag: Option<String>,
    author: Option<String>,
    sort: Option<String>,
) -> Result<Json<CartList>, ApiError> {
    let page = page.unwrap_or(0);
    let per_page = per_page.unwrap_or(20).min(100);
    let (carts, total) = db::list(
        &state.db,
        page,
        per_page,
        q.as_deref(),
        tag.as_deref(),
        author.as_deref(),
        db::Sort::parse(sort.as_deref()),
    )
    .await?;
    Ok(Json(CartList {
        carts,
        total,
        page,
        per_page,
    }))
}

#[get("/api/v2/carts/<id>")]
pub async fn get_cart(
    state: &State<PortState>,
    user: Option<AuthUser>,
    id: &str,
) -> Result<Json<CartDetail>, ApiError> {
    if !valid_id(id) {
        return Err(ApiError::bad_request("invalid id"));
    }
    let cart = db::get(&state.db, id)
        .await?
        .ok_or_else(|| ApiError::not_found("cart not found"))?;
    let versions = db::list_versions(&state.db, id)
        .await?
        .into_iter()
        .map(CartVersionInfo::from)
        .collect();
    let own_rating = match &user {
        Some(u) => db::get_own_rating(&state.db, id, &u.id).await?,
        None => None,
    };
    Ok(Json(CartDetail {
        cart,
        versions,
        own_rating,
    }))
}

#[post("/api/v2/carts", data = "<upload>")]
pub async fn upload_cart(
    user: VerifiedUser,
    state: &State<PortState>,
    upload: Form<CartUpload<'_>>,
) -> Result<Json<Cart>, ApiError> {
    Ok(Json(create_cart_impl(state, &user.0, upload).await?))
}

#[patch("/api/v2/carts/<id>", data = "<patch>")]
pub async fn update_cart(
    user: AuthUser,
    state: &State<PortState>,
    id: &str,
    patch: Json<CartPatch>,
) -> Result<Json<Cart>, ApiError> {
    if !valid_id(id) {
        return Err(ApiError::bad_request("invalid id"));
    }
    if let Some(title) = &patch.title
        && (title.trim().is_empty() || title.len() > 64)
    {
        return Err(ApiError::bad_request("title must be 1-64 chars"));
    }
    if let Some(description) = &patch.description
        && description.len() > 512
    {
        return Err(ApiError::bad_request("description max 512 chars"));
    }

    let cart = db::get_cart_model(&state.db, id)
        .await?
        .ok_or_else(|| ApiError::not_found("cart not found"))?;
    require_owner(&user, &cart)?;

    db::update_cart(&state.db, id, &patch).await?;
    Ok(Json(db::get(&state.db, id).await?.expect("just updated")))
}

#[delete("/api/v2/carts/<id>")]
pub async fn delete_cart(
    user: AuthUser,
    state: &State<PortState>,
    id: &str,
) -> Result<(), ApiError> {
    if !valid_id(id) {
        return Err(ApiError::bad_request("invalid id"));
    }
    let cart = db::get_cart_model(&state.db, id)
        .await?
        .ok_or_else(|| ApiError::not_found("cart not found"))?;
    require_owner(&user, &cart)?;

    db::delete_cart(&state.db, id).await?;
    Ok(())
}
