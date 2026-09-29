use rocket::{
    FromForm, State, data::Capped, form::Form, fs::TempFile, get, post, serde::json::Json,
};

use super::{BinaryFile, safe_filename, valid_id};
use crate::{
    PortState,
    auth::AuthUser,
    db,
    entities::cart_versions,
    error::ApiError,
    handlers::carts::{read_and_validate_cart_upload, require_owner},
    models::{CartPatch, CartVersionInfo, VersionMeta},
};

#[derive(FromForm)]
pub struct VersionUpload<'v> {
    pub cart: Capped<TempFile<'v>>,
    pub meta: String,
}

async fn resolve_version(
    state: &PortState,
    id: &str,
    version: Option<i32>,
) -> Result<cart_versions::Model, ApiError> {
    let found = match version {
        Some(v) => db::get_version(&state.db, id, v).await?,
        None => db::latest_version(&state.db, id).await?,
    };
    found.ok_or_else(|| ApiError::not_found("version not found"))
}

#[get("/api/v1/carts/<id>/cart?<version>")]
pub async fn download_cart(
    state: &State<PortState>,
    id: &str,
    version: Option<i32>,
) -> Result<BinaryFile, ApiError> {
    if !valid_id(id) {
        return Err(ApiError::bad_request("invalid id"));
    }
    let v = resolve_version(state, id, version).await?;
    let bytes = db::get_cart_blob(&state.db, &v.id)
        .await?
        .ok_or_else(|| ApiError::not_found("cart not found"))?;

    let title = db::get(&state.db, id)
        .await
        .ok()
        .flatten()
        .map(|c| c.title)
        .unwrap_or_else(|| id.to_string());

    let _ = db::increment_downloads(&state.db, id).await;

    Ok(BinaryFile {
        disposition: format!("attachment; filename=\"{}.cav\"", safe_filename(&title)),
        content_type: "application/octet-stream",
        cache: None,
        bytes,
    })
}

#[get("/api/v1/carts/<id>/screenshot?<version>")]
pub async fn get_screenshot(
    state: &State<PortState>,
    id: &str,
    version: Option<i32>,
) -> Result<BinaryFile, ApiError> {
    if !valid_id(id) {
        return Err(ApiError::bad_request("invalid id"));
    }
    let v = resolve_version(state, id, version).await?;
    let bytes = db::get_screenshot_blob(&state.db, &v.id)
        .await?
        .ok_or_else(|| ApiError::not_found("screenshot not found"))?;

    Ok(BinaryFile {
        content_type: "image/png",
        disposition: "inline".into(),
        cache: Some("public, max-age=86400"),
        bytes,
    })
}

#[derive(FromForm)]
pub struct ScreenshotUpload<'v> {
    pub screenshot: Capped<TempFile<'v>>,
}

#[post("/api/v1/carts/<id>/screenshot?<version>", data = "<upload>")]
pub async fn upload_screenshot(
    user: AuthUser,
    state: &State<PortState>,
    id: &str,
    version: Option<i32>,
    upload: Form<ScreenshotUpload<'_>>,
) -> Result<(), ApiError> {
    if !valid_id(id) {
        return Err(ApiError::bad_request("invalid id"));
    }
    let cart = db::get_cart_model(&state.db, id)
        .await?
        .ok_or_else(|| ApiError::not_found("cart not found"))?;
    require_owner(&user, &cart)?;
    let v = resolve_version(state, id, version).await?;

    if !upload.screenshot.is_complete() || upload.screenshot.n.written > 512 * 1024 {
        return Err(ApiError::PayloadTooLarge("screenshot max 512KB".into()));
    }

    let tmp_path = upload
        .screenshot
        .value
        .path()
        .ok_or_else(|| ApiError::internal("temp file unavailable"))?;

    let bytes = tokio::fs::read(tmp_path).await.map_err(ApiError::from)?;
    if bytes.len() < 8 {
        return Err(ApiError::bad_request("file too small"));
    }
    if &bytes[..8] != b"\x89PNG\r\n\x1a\n" {
        return Err(ApiError::bad_request("must be a PNG"));
    }

    db::set_screenshot(&state.db, &v.id, &bytes).await?;
    Ok(())
}

#[post("/api/v1/carts/<id>/versions", data = "<upload>")]
pub async fn create_version(
    user: AuthUser,
    state: &State<PortState>,
    id: &str,
    upload: Form<VersionUpload<'_>>,
) -> Result<Json<CartVersionInfo>, ApiError> {
    if !valid_id(id) {
        return Err(ApiError::bad_request("invalid id"));
    }
    let cart = db::get_cart_model(&state.db, id)
        .await?
        .ok_or_else(|| ApiError::not_found("cart not found"))?;
    require_owner(&user, &cart)?;

    let (bytes, content_hash) = read_and_validate_cart_upload(&upload.cart).await?;

    // meta may be an empty body ({} or "") for a no-changelog bump.
    let meta: VersionMeta = if upload.meta.trim().is_empty() {
        VersionMeta::default()
    } else {
        serde_json::from_str(&upload.meta)?
    };

    // An admin versioning a deleted account's cart has no owner to compare.
    if let Some(owner_id) = &cart.owner_id
        && let Some((title, author)) =
            db::find_other_owner_by_content_hash(&state.db, &content_hash, owner_id, false).await?
    {
        return Err(ApiError::conflict(format!(
            "This cart's content matches an existing published cart \"{title}\" by {author}"
        )));
    }

    let version = db::insert_version(
        &state.db,
        id,
        &meta.changelog,
        &user.username,
        &bytes,
        &content_hash,
    )
    .await?;
    if meta.remixable.is_some() {
        let patch = CartPatch {
            title: None,
            description: None,
            tags: None,
            remixable: meta.remixable,
        };
        db::update_cart(&state.db, id, &patch).await?;
    }
    let v = db::get_version(&state.db, id, version)
        .await?
        .ok_or_else(|| ApiError::internal("insert failed"))?;
    Ok(Json(CartVersionInfo::from(v)))
}
