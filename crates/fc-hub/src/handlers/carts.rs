use rocket::{
    FromForm, State, data::Capped, delete, form::Form, fs::TempFile, get, patch, post,
    serde::json::Json,
};
use tokio::io::AsyncReadExt;
use uuid::Uuid;

use super::{move_file, valid_id};
use crate::{
    HubState,
    auth::AuthUser,
    db,
    entities::carts,
    error::ApiError,
    models::{Cart, CartDetail, CartList, CartMeta, CartPatch, CartVersionInfo},
};

pub(crate) fn validate_meta(meta: &CartMeta) -> Result<(), ApiError> {
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

pub(crate) fn require_owner(user: &AuthUser, cart: &carts::Model) -> Result<(), ApiError> {
    if cart.owner_id.as_deref() == Some(user.id.as_str()) || user.is_admin {
        Ok(())
    } else {
        Err(ApiError::forbidden("not the owner of this cart"))
    }
}

#[derive(FromForm)]
pub struct CartUpload<'v> {
    pub rom: Capped<TempFile<'v>>,
    pub meta: String,
}

/// Shared multipart rom+meta validation, used by both the `/api/v2/carts`
/// and legacy `/api/carts` create routes.
pub(crate) async fn create_cart_impl(
    state: &HubState,
    user: &AuthUser,
    upload: Form<CartUpload<'_>>,
) -> Result<Cart, ApiError> {
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
    let dest = state.data_dir.join(db::rom_rel_path(&id, 1));
    move_file(tmp_path, &dest)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;

    if let Err(e) = db::insert_cart(&state.db, &user.id, &id, &meta, rom_len).await {
        let _ = tokio::fs::remove_file(&dest).await;
        return Err(ApiError::from(e));
    }
    db::get(&state.db, &id)
        .await?
        .ok_or_else(|| ApiError::internal("insert failed"))
}

#[get("/api/v2/carts?<page>&<per_page>&<q>&<tag>&<author>&<sort>")]
#[allow(clippy::too_many_arguments)]
pub async fn list_carts(
    state: &State<HubState>,
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
    state: &State<HubState>,
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
    user: AuthUser,
    state: &State<HubState>,
    upload: Form<CartUpload<'_>>,
) -> Result<Json<Cart>, ApiError> {
    Ok(Json(create_cart_impl(state, &user, upload).await?))
}

#[patch("/api/v2/carts/<id>", data = "<patch>")]
pub async fn update_cart(
    user: AuthUser,
    state: &State<HubState>,
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
    state: &State<HubState>,
    id: &str,
) -> Result<(), ApiError> {
    if !valid_id(id) {
        return Err(ApiError::bad_request("invalid id"));
    }
    let cart = db::get_cart_model(&state.db, id)
        .await?
        .ok_or_else(|| ApiError::not_found("cart not found"))?;
    require_owner(&user, &cart)?;

    let files = db::delete_cart(&state.db, id).await?;
    for (rom_path, screenshot_path) in files {
        let _ = tokio::fs::remove_file(state.data_dir.join(rom_path)).await;
        if let Some(p) = screenshot_path {
            let _ = tokio::fs::remove_file(state.data_dir.join(p)).await;
        }
    }
    Ok(())
}
