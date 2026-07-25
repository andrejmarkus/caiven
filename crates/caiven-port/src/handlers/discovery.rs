use rocket::{State, get, serde::json::Json};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};

use crate::{
    PortState,
    auth::AuthUser,
    db,
    entities::follows,
    error::ApiError,
    models::{TagCount, UserProfile},
};

#[get("/api/v2/tags")]
pub async fn list_tags(state: &State<PortState>) -> Result<Json<Vec<TagCount>>, ApiError> {
    Ok(Json(db::list_tags(&state.db).await?))
}

#[get("/api/v2/users/<username>?<page>&<per_page>")]
pub async fn user_profile(
    state: &State<PortState>,
    viewer: Option<AuthUser>,
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
    let follower_rows = follows::Entity::find()
        .filter(follows::Column::FollowedId.eq(&user.id))
        .all(&state.db)
        .await?;
    let following_count = follows::Entity::find()
        .filter(follows::Column::FollowerId.eq(&user.id))
        .count(&state.db)
        .await?;
    let followed_by_me = viewer
        .as_ref()
        .map(|v| follower_rows.iter().any(|f| f.follower_id == v.id))
        .unwrap_or(false);
    let total_plays = carts.iter().map(|c| c.plays).sum();
    Ok(Json(UserProfile {
        username: user.username,
        is_admin: user.is_admin,
        created_at: user.created_at,
        carts,
        total,
        total_plays,
        follower_count: follower_rows.len() as u64,
        following_count,
        followed_by_me,
    }))
}
