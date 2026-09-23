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

/// Reads an uploaded cart's temp file, enforces the size cap and the cheap
/// magic-byte pre-check, and returns its bytes plus content hash. Shared by
/// the cart-create and version-create routes so they can't drift apart.
pub(crate) async fn read_and_validate_cart_upload(
    cart: &Capped<TempFile<'_>>,
) -> Result<(Vec<u8>, String), ApiError> {
    if !cart.is_complete() {
        return Err(cart_too_large());
    }
    let cart_len = cart.n.written as usize;
    if cart_len > MAX_CART_BYTES {
        return Err(cart_too_large());
    }

    let tmp_path = cart
        .value
        .path()
        .ok_or_else(|| ApiError::internal("temp file unavailable"))?;

    let bytes = tokio::fs::read(tmp_path).await.map_err(ApiError::from)?;
    if bytes.len() < 6 {
        return Err(ApiError::bad_request("cart too small"));
    }
    if &bytes[..6] != b"CAIVEN" {
        return Err(ApiError::bad_request("not a valid Caiven cart"));
    }

    let content_hash = caiven_cart::content_hash(&bytes)
        .map_err(|_| ApiError::bad_request("not a valid Caiven cart"))?;
    Ok((bytes, content_hash))
}

/// Shared multipart cart+meta validation, used by both the `/api/v2/carts`
/// and legacy `/api/carts` create routes.
pub(crate) async fn create_cart_impl(
    state: &PortState,
    user: &AuthUser,
    upload: Form<CartUpload<'_>>,
) -> Result<Cart, ApiError> {
    let (bytes, content_hash) = read_and_validate_cart_upload(&upload.cart).await?;

    let meta: CartMeta = serde_json::from_str(&upload.meta)?;
    validate_meta(&meta)?;
    let lineage = match meta.parent_cart_id.as_deref() {
        Some(parent_id) => Some(remix_lineage(state, parent_id, &content_hash).await?),
        None => None,
    };

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
        lineage.as_ref(),
        &bytes,
        Some(&content_hash),
    )
    .await?;
    db::get(&state.db, &id)
        .await?
        .ok_or_else(|| ApiError::internal("insert failed"))
}

/// Resolves a remix's parent. Only carts whose owner opted in can be
/// remixed, and a remix must differ from the version it was taken from.
async fn remix_lineage(
    state: &PortState,
    parent_id: &str,
    content_hash: &str,
) -> Result<db::Lineage, ApiError> {
    if !valid_id(parent_id) {
        return Err(ApiError::bad_request("invalid parent cart id"));
    }
    let parent = db::get_cart_model(&state.db, parent_id)
        .await?
        .ok_or_else(|| ApiError::not_found("the cart you remixed no longer exists"))?;
    if !parent.remixable {
        return Err(ApiError::forbidden(
            "this cart's creator has not allowed remixes",
        ));
    }
    let parent_version = db::latest_version(&state.db, parent_id)
        .await?
        .ok_or_else(|| ApiError::not_found("the cart you remixed has no versions"))?;
    if parent_version.content_hash.as_deref() == Some(content_hash) {
        return Err(ApiError::bad_request(
            "your remix is identical to the original — change something before publishing",
        ));
    }
    Ok(db::Lineage {
        root_cart_id: parent.root_cart_id.unwrap_or_else(|| parent.id.clone()),
        parent_cart_id: parent.id,
        parent_version: parent_version.version,
    })
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

const RECENT_REMIXES: u64 = 6;

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
    let parent = match cart.parent_cart_id.as_deref() {
        Some(parent_id) => match db::get_cart_model(&state.db, parent_id).await? {
            Some(m) => Some(db::cart_ref(&state.db, m).await?),
            None => None,
        },
        None => None,
    };
    let (remix_count, recent_remixes) = db::remixes_of(&state.db, id, RECENT_REMIXES).await?;
    Ok(Json(CartDetail {
        cart,
        versions,
        own_rating,
        parent,
        remix_count,
        recent_remixes,
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
    user.require_full_scope()?;
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
    user.require_full_scope()?;
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
