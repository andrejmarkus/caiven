use rocket::{State, delete, get, post, put, serde::json::Json};

use super::valid_id;
use crate::{
    PortState,
    auth::AuthUser,
    db,
    error::ApiError,
    models::{Cart, CommentInfo, CommentInput, RatingInput},
};

#[put("/api/v2/carts/<id>/rating", data = "<input>")]
pub async fn rate_cart(
    state: &State<PortState>,
    user: AuthUser,
    id: &str,
    input: Json<RatingInput>,
) -> Result<Json<Cart>, ApiError> {
    if !valid_id(id) {
        return Err(ApiError::bad_request("invalid id"));
    }
    if !(1..=5).contains(&input.score) {
        return Err(ApiError::bad_request("score must be 1-5"));
    }
    db::get_cart_model(&state.db, id)
        .await?
        .ok_or_else(|| ApiError::not_found("cart not found"))?;

    db::upsert_rating(&state.db, id, &user.id, input.score).await?;
    Ok(Json(db::get(&state.db, id).await?.expect("just rated")))
}

#[delete("/api/v2/carts/<id>/rating")]
pub async fn unrate_cart(
    state: &State<PortState>,
    user: AuthUser,
    id: &str,
) -> Result<Json<Cart>, ApiError> {
    if !valid_id(id) {
        return Err(ApiError::bad_request("invalid id"));
    }
    db::get_cart_model(&state.db, id)
        .await?
        .ok_or_else(|| ApiError::not_found("cart not found"))?;

    db::delete_rating(&state.db, id, &user.id).await?;
    Ok(Json(db::get(&state.db, id).await?.expect("just unrated")))
}

#[get("/api/v2/carts/<id>/comments")]
pub async fn list_comments(
    state: &State<PortState>,
    id: &str,
) -> Result<Json<Vec<CommentInfo>>, ApiError> {
    if !valid_id(id) {
        return Err(ApiError::bad_request("invalid id"));
    }
    db::get_cart_model(&state.db, id)
        .await?
        .ok_or_else(|| ApiError::not_found("cart not found"))?;

    let comments = db::list_comments(&state.db, id)
        .await?
        .into_iter()
        .map(|(c, author)| CommentInfo {
            id: c.id,
            author,
            body: c.body,
            created_at: c.created_at,
        })
        .collect();
    Ok(Json(comments))
}

#[post("/api/v2/carts/<id>/comments", data = "<input>")]
pub async fn add_comment(
    state: &State<PortState>,
    user: AuthUser,
    id: &str,
    input: Json<CommentInput>,
) -> Result<Json<CommentInfo>, ApiError> {
    if !valid_id(id) {
        return Err(ApiError::bad_request("invalid id"));
    }
    let body = input.body.trim();
    if body.is_empty() {
        return Err(ApiError::bad_request("comment cannot be empty"));
    }
    if body.len() > 1000 {
        return Err(ApiError::bad_request("comment max 1000 chars"));
    }
    db::get_cart_model(&state.db, id)
        .await?
        .ok_or_else(|| ApiError::not_found("cart not found"))?;

    let comment = db::add_comment(&state.db, id, &user.id, body).await?;
    Ok(Json(CommentInfo {
        id: comment.id,
        author: user.username,
        body: comment.body,
        created_at: comment.created_at,
    }))
}

#[delete("/api/v2/carts/<id>/comments/<comment_id>")]
pub async fn delete_comment(
    state: &State<PortState>,
    user: AuthUser,
    id: &str,
    comment_id: &str,
) -> Result<(), ApiError> {
    if !valid_id(id) {
        return Err(ApiError::bad_request("invalid id"));
    }
    let cart = db::get_cart_model(&state.db, id)
        .await?
        .ok_or_else(|| ApiError::not_found("cart not found"))?;
    let comment = db::get_comment(&state.db, comment_id)
        .await?
        .ok_or_else(|| ApiError::not_found("comment not found"))?;
    if comment.cart_id != id {
        return Err(ApiError::not_found("comment not found"));
    }

    let is_comment_owner = comment.user_id == user.id;
    let is_cart_owner = cart.owner_id.as_deref() == Some(user.id.as_str());
    if !is_comment_owner && !is_cart_owner && !user.is_admin {
        return Err(ApiError::forbidden("cannot delete this comment"));
    }

    db::delete_comment(&state.db, comment_id).await?;
    Ok(())
}
