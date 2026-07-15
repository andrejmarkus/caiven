use rocket::{
    FromForm, State, data::Capped, form::Form, fs::TempFile, get, post, serde::json::Json,
};
use tokio::io::AsyncReadExt;

use super::{BinaryFile, move_file, safe_filename, valid_id};
use crate::{
    HubState,
    auth::AuthUser,
    db,
    error::ApiError,
    handlers::carts::require_owner,
    models::{CartVersionInfo, VersionMeta},
};

#[derive(FromForm)]
pub struct VersionUpload<'v> {
    pub rom: Capped<TempFile<'v>>,
    pub meta: String,
}

async fn resolve_version(
    state: &HubState,
    id: &str,
    version: Option<i32>,
) -> Result<crate::entities::cart_versions::Model, ApiError> {
    let found = match version {
        Some(v) => db::get_version(&state.db, id, v).await?,
        None => db::latest_version(&state.db, id).await?,
    };
    found.ok_or_else(|| ApiError::not_found("version not found"))
}

pub(crate) async fn download_rom_impl(
    state: &HubState,
    id: &str,
    version: Option<i32>,
) -> Result<BinaryFile, ApiError> {
    if !valid_id(id) {
        return Err(ApiError::bad_request("invalid id"));
    }
    let v = resolve_version(state, id, version).await?;
    let bytes = tokio::fs::read(state.data_dir.join(&v.rom_path))
        .await
        .map_err(|_| ApiError::not_found("ROM not found"))?;

    let title = db::get(&state.db, id)
        .await
        .ok()
        .flatten()
        .map(|c| c.title)
        .unwrap_or_else(|| id.to_string());

    let _ = db::increment_downloads(&state.db, id).await;

    Ok(BinaryFile {
        disposition: format!("attachment; filename=\"{}.rom\"", safe_filename(&title)),
        content_type: "application/octet-stream",
        cache: None,
        bytes,
    })
}

pub(crate) async fn get_screenshot_impl(
    state: &HubState,
    id: &str,
    version: Option<i32>,
) -> Result<BinaryFile, ApiError> {
    if !valid_id(id) {
        return Err(ApiError::bad_request("invalid id"));
    }
    let v = resolve_version(state, id, version).await?;
    if !v.has_screenshot {
        return Err(ApiError::not_found("screenshot not found"));
    }
    let bytes = tokio::fs::read(state.data_dir.join(db::screenshot_rel_path(id, v.version)))
        .await
        .map_err(|_| ApiError::not_found("screenshot not found"))?;

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

pub(crate) async fn upload_screenshot_impl(
    state: &HubState,
    user: &AuthUser,
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
    require_owner(user, &cart)?;
    let v = resolve_version(state, id, version).await?;

    if !upload.screenshot.is_complete() || upload.screenshot.n.written > 512 * 1024 {
        return Err(ApiError::PayloadTooLarge("screenshot max 512KB".into()));
    }

    let tmp_path = upload
        .screenshot
        .value
        .path()
        .ok_or_else(|| ApiError::internal("temp file unavailable"))?;

    let mut f = tokio::fs::File::open(tmp_path)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    let mut magic = [0u8; 8];
    f.read_exact(&mut magic)
        .await
        .map_err(|_| ApiError::bad_request("file too small"))?;
    drop(f);

    if &magic != b"\x89PNG\r\n\x1a\n" {
        return Err(ApiError::bad_request("must be a PNG"));
    }

    let dest = state.data_dir.join(db::screenshot_rel_path(id, v.version));
    move_file(tmp_path, &dest)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;

    if let Err(e) = db::set_version_has_screenshot(&state.db, id, v.version).await {
        let _ = tokio::fs::remove_file(&dest).await;
        return Err(ApiError::from(e));
    }
    Ok(())
}

// ── v2 routes ───────────────────────────────────────────────────────────────

#[post("/api/v2/carts/<id>/versions", data = "<upload>")]
pub async fn create_version(
    user: AuthUser,
    state: &State<HubState>,
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

    if !upload.rom.is_complete() {
        return Err(ApiError::PayloadTooLarge("ROM max 1MB".into()));
    }
    let rom_len = upload.rom.n.written as usize;
    if rom_len > 1024 * 1024 {
        return Err(ApiError::PayloadTooLarge("ROM max 1MB".into()));
    }

    let tmp_path = upload
        .rom
        .value
        .path()
        .ok_or_else(|| ApiError::internal("temp file unavailable"))?;

    let mut f = tokio::fs::File::open(tmp_path)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    let mut magic = [0u8; 6];
    f.read_exact(&mut magic)
        .await
        .map_err(|_| ApiError::bad_request("ROM too small"))?;
    drop(f);
    if &magic != b"SPEAR2" {
        return Err(ApiError::bad_request("not a valid FC ROM"));
    }

    // meta may be an empty body ({} or "") for a no-changelog bump.
    let meta: VersionMeta = if upload.meta.trim().is_empty() {
        VersionMeta::default()
    } else {
        serde_json::from_str(&upload.meta)?
    };
    let next = latest_or_one(state, id).await? + 1;
    let dest = state.data_dir.join(db::rom_rel_path(id, next));
    move_file(tmp_path, &dest)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;

    let version = match db::insert_version(&state.db, id, &meta.changelog, rom_len).await {
        Ok(v) => v,
        Err(e) => {
            let _ = tokio::fs::remove_file(&dest).await;
            return Err(ApiError::from(e));
        }
    };
    let v = db::get_version(&state.db, id, version)
        .await?
        .ok_or_else(|| ApiError::internal("insert failed"))?;
    Ok(Json(CartVersionInfo::from(v)))
}

async fn latest_or_one(state: &HubState, id: &str) -> Result<i32, ApiError> {
    Ok(db::latest_version(&state.db, id)
        .await?
        .map(|v| v.version)
        .unwrap_or(0))
}

#[get("/api/v2/carts/<id>/rom?<version>")]
pub async fn download_rom(
    state: &State<HubState>,
    id: &str,
    version: Option<i32>,
) -> Result<BinaryFile, ApiError> {
    download_rom_impl(state, id, version).await
}

#[post("/api/v2/carts/<id>/screenshot?<version>", data = "<upload>")]
pub async fn upload_screenshot(
    user: AuthUser,
    state: &State<HubState>,
    id: &str,
    version: Option<i32>,
    upload: Form<ScreenshotUpload<'_>>,
) -> Result<(), ApiError> {
    upload_screenshot_impl(state, &user, id, version, upload).await
}

#[get("/api/v2/carts/<id>/screenshot?<version>")]
pub async fn get_screenshot(
    state: &State<HubState>,
    id: &str,
    version: Option<i32>,
) -> Result<BinaryFile, ApiError> {
    get_screenshot_impl(state, id, version).await
}
