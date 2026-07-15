use rocket::{
    FromForm, State,
    data::Capped,
    form::Form,
    fs::TempFile,
    get, post,
    response::content::RawHtml,
    serde::json::Json,
};
use tokio::io::AsyncReadExt;
use uuid::Uuid;

use super::{BinaryFile, move_file, safe_filename, valid_id};
use crate::{
    HubState,
    auth::AuthUser,
    db,
    error::ApiError,
    gallery,
    models::{Cart, CartList, CartMeta},
};

fn validate_meta(meta: &CartMeta) -> Result<(), ApiError> {
    if meta.title.trim().is_empty() {
        return Err(ApiError::bad_request("title required"));
    }
    if meta.author.trim().is_empty() {
        return Err(ApiError::bad_request("author required"));
    }
    if meta.title.len() > 64 {
        return Err(ApiError::bad_request("title max 64 chars"));
    }
    if meta.author.len() > 64 {
        return Err(ApiError::bad_request("author max 64 chars"));
    }
    if meta.description.len() > 512 {
        return Err(ApiError::bad_request("description max 512 chars"));
    }
    Ok(())
}

#[get("/?<page>&<q>")]
pub async fn gallery_page(
    state: &State<HubState>,
    page: Option<u32>,
    q: Option<String>,
) -> RawHtml<String> {
    let p = page.unwrap_or(0);
    let (carts, total) = db::list(&state.db, p, 24, q.as_deref())
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
    let (carts, total) = db::list(&state.db, page, per_page, q.as_deref()).await?;
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

#[derive(FromForm)]
pub struct CartUpload<'v> {
    pub rom: Capped<TempFile<'v>>,
    pub meta: String,
}

#[post("/api/carts", data = "<upload>")]
pub async fn upload_cart(
    _user: AuthUser,
    state: &State<HubState>,
    upload: Form<CartUpload<'_>>,
) -> Result<Json<Cart>, ApiError> {
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

    let meta: CartMeta = serde_json::from_str(&upload.meta)?;
    validate_meta(&meta)?;

    let id = Uuid::new_v4().to_string();
    let dest = state.data_dir.join("roms").join(format!("{}.rom", id));
    move_file(tmp_path, &dest)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;

    if let Err(e) = db::insert(&state.db, &id, &meta, rom_len).await {
        let _ = tokio::fs::remove_file(&dest).await;
        return Err(ApiError::from(e));
    }
    let cart = db::get(&state.db, &id)
        .await?
        .ok_or_else(|| ApiError::internal("insert failed"))?;
    Ok(Json(cart))
}

#[get("/api/carts/<id>/rom")]
pub async fn download_rom(state: &State<HubState>, id: &str) -> Result<BinaryFile, ApiError> {
    if !valid_id(id) {
        return Err(ApiError::bad_request("invalid id"));
    }
    let path = state.data_dir.join("roms").join(format!("{}.rom", id));
    let bytes = tokio::fs::read(&path)
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

#[derive(FromForm)]
pub struct ScreenshotUpload<'v> {
    pub screenshot: Capped<TempFile<'v>>,
}

#[post("/api/carts/<id>/screenshot", data = "<upload>")]
pub async fn upload_screenshot(
    _user: AuthUser,
    state: &State<HubState>,
    id: &str,
    upload: Form<ScreenshotUpload<'_>>,
) -> Result<(), ApiError> {
    if !valid_id(id) {
        return Err(ApiError::bad_request("invalid id"));
    }
    db::get(&state.db, id)
        .await?
        .ok_or_else(|| ApiError::not_found("cart not found"))?;

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

    let dest = state
        .data_dir
        .join("screenshots")
        .join(format!("{}.png", id));
    move_file(tmp_path, &dest)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;

    if let Err(e) = db::set_has_screenshot(&state.db, id).await {
        let _ = tokio::fs::remove_file(&dest).await;
        return Err(ApiError::from(e));
    }
    Ok(())
}

#[get("/api/carts/<id>/screenshot")]
pub async fn get_screenshot(state: &State<HubState>, id: &str) -> Result<BinaryFile, ApiError> {
    if !valid_id(id) {
        return Err(ApiError::bad_request("invalid id"));
    }
    let bytes = tokio::fs::read(
        state
            .data_dir
            .join("screenshots")
            .join(format!("{}.png", id)),
    )
    .await
    .map_err(|_| ApiError::not_found("screenshot not found"))?;

    Ok(BinaryFile {
        content_type: "image/png",
        disposition: "inline".into(),
        cache: Some("public, max-age=86400"),
        bytes,
    })
}
