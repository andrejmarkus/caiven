use rocket::{State, get, serde::json::Json};

use crate::{HubState, db, error::ApiError, models::{TagCount, UserProfile}};

#[get("/api/v2/tags")]
pub async fn list_tags(state: &State<HubState>) -> Result<Json<Vec<TagCount>>, ApiError> {
    Ok(Json(db::list_tags(&state.db).await?))
}

#[get("/api/v2/users/<username>?<page>&<per_page>")]
pub async fn user_profile(
    state: &State<HubState>,
    username: &str,
    page: Option<u32>,
    per_page: Option<u32>,
) -> Result<Json<UserProfile>, ApiError> {
    let user = db::get_user_by_username(&state.db, username)
        .await?
        .ok_or_else(|| ApiError::not_found("user not found"))?;
    let page = page.unwrap_or(0);
    let per_page = per_page.unwrap_or(20).min(100);
    let (carts, total) = db::list_by_owner(&state.db, &user.id, page, per_page).await?;
    Ok(Json(UserProfile {
        username: user.username,
        is_admin: user.is_admin,
        created_at: user.created_at,
        carts,
        total,
    }))
}
