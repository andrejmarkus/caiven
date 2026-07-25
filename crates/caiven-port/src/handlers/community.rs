use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Duration, Utc};
use rocket::{State, delete, get, patch, post, put, serde::json::Json};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
    TransactionTrait,
};
use uuid::Uuid;

use crate::{
    PortState,
    auth::{AuthUser, sha256_hex},
    db,
    entities::{
        cart_versions, collection_carts, collection_follows, collections, follows, jam_entries,
        jams, play_events, users,
    },
    error::ApiError,
    models::{
        CollectionCartInput, CollectionCreate, CollectionInfo, CollectionOrderInput,
        CollectionPatch, DailyMetric, DashboardInfo, FeedEvent, FeedPage, JamCreate, JamEntryInput,
        JamInfo, JamPatch, MetricWindow, PlayInput, PlayResult,
    },
};

fn now() -> String {
    Utc::now().to_rfc3339()
}

fn slug_base(title: &str) -> String {
    let mut out = String::new();
    let mut dash = false;
    for c in title.trim().to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
            dash = false;
        } else if !dash && !out.is_empty() {
            out.push('-');
            dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        "collection".into()
    } else {
        out
    }
}

async fn unique_collection_slug(db: &DatabaseConnection, title: &str) -> Result<String, ApiError> {
    let base = slug_base(title);
    if collections::Entity::find()
        .filter(collections::Column::Slug.eq(&base))
        .one(db)
        .await?
        .is_none()
    {
        return Ok(base);
    }
    Ok(format!(
        "{base}-{}",
        &Uuid::new_v4().simple().to_string()[..6]
    ))
}

fn validate_collection(input: &CollectionCreate) -> Result<(), ApiError> {
    if input.title.trim().is_empty() || input.title.chars().count() > 80 {
        return Err(ApiError::bad_request("collection title must be 1-80 chars"));
    }
    if input.description.chars().count() > 500 {
        return Err(ApiError::bad_request(
            "collection description max 500 chars",
        ));
    }
    Ok(())
}

async fn collection_info(
    db: &DatabaseConnection,
    model: collections::Model,
    viewer: Option<&AuthUser>,
) -> Result<CollectionInfo, ApiError> {
    let owner = users::Entity::find_by_id(&model.owner_id)
        .one(db)
        .await?
        .map(|u| u.username)
        .unwrap_or_else(|| "unknown".into());
    let rows = collection_carts::Entity::find()
        .filter(collection_carts::Column::CollectionId.eq(&model.id))
        .order_by_asc(collection_carts::Column::Position)
        .all(db)
        .await?;
    let mut carts = Vec::with_capacity(rows.len());
    for row in &rows {
        if let Some(cart) = db::get(db, &row.cart_id).await? {
            carts.push(cart);
        }
    }
    let followers = collection_follows::Entity::find()
        .filter(collection_follows::Column::CollectionId.eq(&model.id))
        .all(db)
        .await?;
    let followed_by_me = viewer
        .map(|u| followers.iter().any(|f| f.user_id == u.id))
        .unwrap_or(false);
    Ok(CollectionInfo {
        slug: model.slug,
        title: model.title,
        description: model.description,
        kind: model.kind,
        featured_rank: model.featured_rank,
        owner,
        cart_count: carts.len() as u64,
        follower_count: followers.len() as u64,
        followed_by_me,
        carts,
        created_at: model.created_at,
        updated_at: model.updated_at,
    })
}

async fn collection_for_slug(
    db: &DatabaseConnection,
    slug: &str,
) -> Result<collections::Model, ApiError> {
    collections::Entity::find()
        .filter(collections::Column::Slug.eq(slug))
        .one(db)
        .await?
        .ok_or_else(|| ApiError::not_found("collection not found"))
}

fn require_collection_owner(user: &AuthUser, model: &collections::Model) -> Result<(), ApiError> {
    if user.is_admin || model.owner_id == user.id {
        Ok(())
    } else {
        Err(ApiError::forbidden("not the owner of this collection"))
    }
}

#[post("/api/v2/carts/<id>/play", data = "<input>")]
pub async fn record_play(
    state: &State<PortState>,
    user: Option<AuthUser>,
    id: &str,
    input: Json<PlayInput>,
) -> Result<Json<PlayResult>, ApiError> {
    if input.session_id.len() < 8
        || input.session_id.len() > 128
        || !input
            .session_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-')
    {
        return Err(ApiError::bad_request("invalid session_id"));
    }
    let cart = db::get_cart_model(&state.db, id)
        .await?
        .ok_or_else(|| ApiError::not_found("cart not found"))?;
    let session_key = sha256_hex(&format!("session:{}", input.session_id));
    let viewer_key = sha256_hex(&match user {
        Some(ref u) => format!("user:{}", u.id),
        None => format!("session:{}", input.session_id),
    });
    let exists = play_events::Entity::find()
        .filter(play_events::Column::CartId.eq(id))
        .filter(play_events::Column::SessionKey.eq(&session_key))
        .one(&state.db)
        .await?
        .is_some();
    if exists {
        return Ok(Json(PlayResult {
            counted: false,
            plays: cart.plays,
        }));
    }
    let txn = state.db.begin().await?;
    play_events::ActiveModel {
        id: Set(Uuid::new_v4().to_string()),
        cart_id: Set(id.to_string()),
        session_key: Set(session_key),
        viewer_key: Set(viewer_key),
        played_at: Set(now()),
    }
    .insert(&txn)
    .await?;
    use sea_orm::sea_query::Expr;
    crate::entities::carts::Entity::update_many()
        .col_expr(
            crate::entities::carts::Column::Plays,
            Expr::col(crate::entities::carts::Column::Plays).add(1),
        )
        .filter(crate::entities::carts::Column::Id.eq(id))
        .exec(&txn)
        .await?;
    txn.commit().await?;
    let plays = db::get_cart_model(&state.db, id)
        .await?
        .map(|c| c.plays)
        .unwrap_or(0);
    Ok(Json(PlayResult {
        counted: true,
        plays,
    }))
}

#[put("/api/v2/users/<username>/follow")]
pub async fn follow_user(
    state: &State<PortState>,
    user: AuthUser,
    username: &str,
) -> Result<(), ApiError> {
    let target = db::get_user_by_username(&state.db, username)
        .await?
        .ok_or_else(|| ApiError::not_found("user not found"))?;
    if target.id == user.id {
        return Err(ApiError::bad_request("cannot follow yourself"));
    }
    if follows::Entity::find_by_id((user.id.clone(), target.id.clone()))
        .one(&state.db)
        .await?
        .is_none()
    {
        follows::ActiveModel {
            follower_id: Set(user.id),
            followed_id: Set(target.id),
            created_at: Set(now()),
        }
        .insert(&state.db)
        .await?;
    }
    Ok(())
}

#[delete("/api/v2/users/<username>/follow")]
pub async fn unfollow_user(
    state: &State<PortState>,
    user: AuthUser,
    username: &str,
) -> Result<(), ApiError> {
    let target = db::get_user_by_username(&state.db, username)
        .await?
        .ok_or_else(|| ApiError::not_found("user not found"))?;
    follows::Entity::delete_by_id((user.id, target.id))
        .exec(&state.db)
        .await?;
    Ok(())
}

#[get("/api/v2/collections?<kind>&<owner>&<page>&<per_page>")]
pub async fn list_collections(
    state: &State<PortState>,
    user: Option<AuthUser>,
    kind: Option<String>,
    owner: Option<String>,
    page: Option<u32>,
    per_page: Option<u32>,
) -> Result<Json<Vec<CollectionInfo>>, ApiError> {
    let mut query = collections::Entity::find();
    if let Some(kind) = kind {
        query = query.filter(collections::Column::Kind.eq(kind));
    }
    if let Some(owner) = owner {
        let owner = db::get_user_by_username(&state.db, &owner)
            .await?
            .ok_or_else(|| ApiError::not_found("user not found"))?;
        query = query.filter(collections::Column::OwnerId.eq(owner.id));
    }
    let mut models = query
        .order_by_asc(collections::Column::FeaturedRank)
        .order_by_desc(collections::Column::UpdatedAt)
        .all(&state.db)
        .await?;
    let page = page.unwrap_or(0) as usize;
    let per_page = per_page.unwrap_or(30).min(100) as usize;
    models = models
        .into_iter()
        .skip(page * per_page)
        .take(per_page)
        .collect();
    let mut out = Vec::with_capacity(models.len());
    for model in models {
        out.push(collection_info(&state.db, model, user.as_ref()).await?);
    }
    Ok(Json(out))
}

#[get("/api/v2/collections/<slug>")]
pub async fn get_collection(
    state: &State<PortState>,
    user: Option<AuthUser>,
    slug: &str,
) -> Result<Json<CollectionInfo>, ApiError> {
    let model = collection_for_slug(&state.db, slug).await?;
    Ok(Json(
        collection_info(&state.db, model, user.as_ref()).await?,
    ))
}

async fn create_collection_impl(
    db: &DatabaseConnection,
    user: &AuthUser,
    input: &CollectionCreate,
    kind: &str,
) -> Result<collections::Model, ApiError> {
    validate_collection(input)?;
    let stamp = now();
    let model = collections::ActiveModel {
        id: Set(Uuid::new_v4().to_string()),
        owner_id: Set(user.id.clone()),
        slug: Set(unique_collection_slug(db, &input.title).await?),
        title: Set(input.title.trim().to_string()),
        description: Set(input.description.trim().to_string()),
        kind: Set(kind.into()),
        featured_rank: Set(if kind == "editorial" {
            input.featured_rank
        } else {
            None
        }),
        created_at: Set(stamp.clone()),
        updated_at: Set(stamp),
    }
    .insert(db)
    .await?;
    Ok(model)
}

#[post("/api/v2/collections", data = "<input>")]
pub async fn create_collection(
    state: &State<PortState>,
    user: AuthUser,
    input: Json<CollectionCreate>,
) -> Result<Json<CollectionInfo>, ApiError> {
    let model = create_collection_impl(&state.db, &user, &input, "player").await?;
    Ok(Json(collection_info(&state.db, model, Some(&user)).await?))
}

#[post("/api/v2/admin/collections", data = "<input>")]
pub async fn create_editorial_collection(
    state: &State<PortState>,
    user: AuthUser,
    input: Json<CollectionCreate>,
) -> Result<Json<CollectionInfo>, ApiError> {
    if !user.is_admin {
        return Err(ApiError::forbidden("admin required"));
    }
    let model = create_collection_impl(&state.db, &user, &input, "editorial").await?;
    Ok(Json(collection_info(&state.db, model, Some(&user)).await?))
}

#[patch("/api/v2/collections/<slug>", data = "<input>")]
pub async fn update_collection(
    state: &State<PortState>,
    user: AuthUser,
    slug: &str,
    input: Json<CollectionPatch>,
) -> Result<Json<CollectionInfo>, ApiError> {
    let model = collection_for_slug(&state.db, slug).await?;
    require_collection_owner(&user, &model)?;
    let mut active: collections::ActiveModel = model.into();
    if let Some(title) = &input.title {
        if title.trim().is_empty() || title.chars().count() > 80 {
            return Err(ApiError::bad_request("collection title must be 1-80 chars"));
        }
        active.title = Set(title.trim().to_string());
    }
    if let Some(description) = &input.description {
        if description.chars().count() > 500 {
            return Err(ApiError::bad_request(
                "collection description max 500 chars",
            ));
        }
        active.description = Set(description.trim().to_string());
    }
    if let Some(rank) = input.featured_rank {
        if !user.is_admin {
            return Err(ApiError::forbidden("admin required for featured rank"));
        }
        active.featured_rank = Set(rank);
    }
    active.updated_at = Set(now());
    let model = active.update(&state.db).await?;
    Ok(Json(collection_info(&state.db, model, Some(&user)).await?))
}

#[delete("/api/v2/collections/<slug>")]
pub async fn delete_collection(
    state: &State<PortState>,
    user: AuthUser,
    slug: &str,
) -> Result<(), ApiError> {
    let model = collection_for_slug(&state.db, slug).await?;
    require_collection_owner(&user, &model)?;
    collections::Entity::delete_by_id(model.id)
        .exec(&state.db)
        .await?;
    Ok(())
}

#[post("/api/v2/collections/<slug>/carts", data = "<input>")]
pub async fn add_collection_cart(
    state: &State<PortState>,
    user: AuthUser,
    slug: &str,
    input: Json<CollectionCartInput>,
) -> Result<Json<CollectionInfo>, ApiError> {
    let model = collection_for_slug(&state.db, slug).await?;
    require_collection_owner(&user, &model)?;
    db::get_cart_model(&state.db, &input.cart_id)
        .await?
        .ok_or_else(|| ApiError::not_found("cart not found"))?;
    let existing = collection_carts::Entity::find_by_id((model.id.clone(), input.cart_id.clone()))
        .one(&state.db)
        .await?;
    if existing.is_none() {
        let position = collection_carts::Entity::find()
            .filter(collection_carts::Column::CollectionId.eq(&model.id))
            .all(&state.db)
            .await?
            .len() as i32;
        collection_carts::ActiveModel {
            collection_id: Set(model.id.clone()),
            cart_id: Set(input.cart_id.clone()),
            position: Set(position),
            added_at: Set(now()),
        }
        .insert(&state.db)
        .await?;
    }
    Ok(Json(collection_info(&state.db, model, Some(&user)).await?))
}

#[delete("/api/v2/collections/<slug>/carts/<cart_id>")]
pub async fn remove_collection_cart(
    state: &State<PortState>,
    user: AuthUser,
    slug: &str,
    cart_id: &str,
) -> Result<Json<CollectionInfo>, ApiError> {
    let model = collection_for_slug(&state.db, slug).await?;
    require_collection_owner(&user, &model)?;
    collection_carts::Entity::delete_by_id((model.id.clone(), cart_id.to_string()))
        .exec(&state.db)
        .await?;
    let rows = collection_carts::Entity::find()
        .filter(collection_carts::Column::CollectionId.eq(&model.id))
        .order_by_asc(collection_carts::Column::Position)
        .all(&state.db)
        .await?;
    for (position, row) in rows.into_iter().enumerate() {
        let mut active: collection_carts::ActiveModel = row.into();
        active.position = Set(position as i32);
        active.update(&state.db).await?;
    }
    Ok(Json(collection_info(&state.db, model, Some(&user)).await?))
}

#[put("/api/v2/collections/<slug>/order", data = "<input>")]
pub async fn reorder_collection(
    state: &State<PortState>,
    user: AuthUser,
    slug: &str,
    input: Json<CollectionOrderInput>,
) -> Result<Json<CollectionInfo>, ApiError> {
    let model = collection_for_slug(&state.db, slug).await?;
    require_collection_owner(&user, &model)?;
    let rows = collection_carts::Entity::find()
        .filter(collection_carts::Column::CollectionId.eq(&model.id))
        .all(&state.db)
        .await?;
    let current: HashSet<_> = rows.iter().map(|r| r.cart_id.as_str()).collect();
    let wanted: HashSet<_> = input.cart_ids.iter().map(String::as_str).collect();
    if current != wanted || input.cart_ids.len() != rows.len() {
        return Err(ApiError::bad_request(
            "cart_ids must exactly match collection membership",
        ));
    }
    let txn = state.db.begin().await?;
    for (position, cart_id) in input.cart_ids.iter().enumerate() {
        let row = rows
            .iter()
            .find(|r| &r.cart_id == cart_id)
            .expect("validated");
        let mut active: collection_carts::ActiveModel = row.clone().into();
        active.position = Set(position as i32);
        active.update(&txn).await?;
    }
    txn.commit().await?;
    Ok(Json(collection_info(&state.db, model, Some(&user)).await?))
}

#[put("/api/v2/collections/<slug>/follow")]
pub async fn follow_collection(
    state: &State<PortState>,
    user: AuthUser,
    slug: &str,
) -> Result<(), ApiError> {
    let model = collection_for_slug(&state.db, slug).await?;
    if collection_follows::Entity::find_by_id((model.id.clone(), user.id.clone()))
        .one(&state.db)
        .await?
        .is_none()
    {
        collection_follows::ActiveModel {
            collection_id: Set(model.id),
            user_id: Set(user.id),
            created_at: Set(now()),
        }
        .insert(&state.db)
        .await?;
    }
    Ok(())
}

#[delete("/api/v2/collections/<slug>/follow")]
pub async fn unfollow_collection(
    state: &State<PortState>,
    user: AuthUser,
    slug: &str,
) -> Result<(), ApiError> {
    let model = collection_for_slug(&state.db, slug).await?;
    collection_follows::Entity::delete_by_id((model.id, user.id))
        .exec(&state.db)
        .await?;
    Ok(())
}

fn parse_time(value: &str, field: &str) -> Result<DateTime<Utc>, ApiError> {
    DateTime::parse_from_rfc3339(value)
        .map(|d| d.with_timezone(&Utc))
        .map_err(|_| ApiError::bad_request(format!("{field} must be RFC3339")))
}

fn jam_status(model: &jams::Model) -> String {
    let stamp = Utc::now();
    let starts = parse_time(&model.starts_at, "starts_at").ok();
    let closes = parse_time(&model.submissions_close_at, "submissions_close_at").ok();
    match (starts, closes) {
        (Some(s), _) if stamp < s => "upcoming",
        (_, Some(c)) if stamp < c => "open",
        _ => "closed",
    }
    .into()
}

fn validate_jam(input: &JamCreate) -> Result<(), ApiError> {
    if input.title.trim().is_empty() || input.title.chars().count() > 100 {
        return Err(ApiError::bad_request("jam title must be 1-100 chars"));
    }
    let starts = parse_time(&input.starts_at, "starts_at")?;
    let closes = parse_time(&input.submissions_close_at, "submissions_close_at")?;
    let ends = parse_time(&input.ends_at, "ends_at")?;
    if !(starts < closes && closes <= ends) {
        return Err(ApiError::bad_request(
            "jam timestamps must satisfy starts < submissions_close <= ends",
        ));
    }
    Ok(())
}

async fn jam_info(db: &DatabaseConnection, model: jams::Model) -> Result<JamInfo, ApiError> {
    let status = jam_status(&model);
    let entries = jam_entries::Entity::find()
        .filter(jam_entries::Column::JamId.eq(&model.id))
        .order_by_desc(jam_entries::Column::SubmittedAt)
        .all(db)
        .await?;
    let creators: HashSet<_> = entries.iter().map(|e| e.user_id.as_str()).collect();
    let mut carts = Vec::with_capacity(entries.len());
    for entry in &entries {
        if let Some(cart) = db::get(db, &entry.cart_id).await? {
            carts.push(cart);
        }
    }
    Ok(JamInfo {
        slug: model.slug,
        title: model.title,
        description: model.description,
        rules: model.rules,
        starts_at: model.starts_at,
        submissions_close_at: model.submissions_close_at,
        ends_at: model.ends_at,
        status,
        entry_count: entries.len() as u64,
        creator_count: creators.len() as u64,
        carts,
    })
}

async fn jam_for_slug(db: &DatabaseConnection, slug: &str) -> Result<jams::Model, ApiError> {
    jams::Entity::find()
        .filter(jams::Column::Slug.eq(slug))
        .one(db)
        .await?
        .ok_or_else(|| ApiError::not_found("jam not found"))
}

#[get("/api/v2/jams")]
pub async fn list_jams(state: &State<PortState>) -> Result<Json<Vec<JamInfo>>, ApiError> {
    let models = jams::Entity::find()
        .order_by_desc(jams::Column::StartsAt)
        .all(&state.db)
        .await?;
    let mut out = Vec::with_capacity(models.len());
    for model in models {
        out.push(jam_info(&state.db, model).await?);
    }
    Ok(Json(out))
}

#[get("/api/v2/jams/<slug>")]
pub async fn get_jam(state: &State<PortState>, slug: &str) -> Result<Json<JamInfo>, ApiError> {
    let model = jam_for_slug(&state.db, slug).await?;
    Ok(Json(jam_info(&state.db, model).await?))
}

#[post("/api/v2/admin/jams", data = "<input>")]
pub async fn create_jam(
    state: &State<PortState>,
    user: AuthUser,
    input: Json<JamCreate>,
) -> Result<Json<JamInfo>, ApiError> {
    if !user.is_admin {
        return Err(ApiError::forbidden("admin required"));
    }
    validate_jam(&input)?;
    let slug = input
        .slug
        .clone()
        .unwrap_or_else(|| slug_base(&input.title));
    if jams::Entity::find()
        .filter(jams::Column::Slug.eq(&slug))
        .one(&state.db)
        .await?
        .is_some()
    {
        return Err(ApiError::bad_request("jam slug already exists"));
    }
    let stamp = now();
    let model = jams::ActiveModel {
        id: Set(Uuid::new_v4().to_string()),
        slug: Set(slug),
        title: Set(input.title.trim().into()),
        description: Set(input.description.trim().into()),
        rules: Set(input.rules.trim().into()),
        starts_at: Set(input.starts_at.clone()),
        submissions_close_at: Set(input.submissions_close_at.clone()),
        ends_at: Set(input.ends_at.clone()),
        created_at: Set(stamp.clone()),
        updated_at: Set(stamp),
    }
    .insert(&state.db)
    .await?;
    Ok(Json(jam_info(&state.db, model).await?))
}

#[patch("/api/v2/admin/jams/<slug>", data = "<input>")]
pub async fn update_jam(
    state: &State<PortState>,
    user: AuthUser,
    slug: &str,
    input: Json<JamPatch>,
) -> Result<Json<JamInfo>, ApiError> {
    if !user.is_admin {
        return Err(ApiError::forbidden("admin required"));
    }
    let model = jam_for_slug(&state.db, slug).await?;
    let prospective_starts = input.starts_at.as_deref().unwrap_or(&model.starts_at);
    let prospective_closes = input
        .submissions_close_at
        .as_deref()
        .unwrap_or(&model.submissions_close_at);
    let prospective_ends = input.ends_at.as_deref().unwrap_or(&model.ends_at);
    let starts = parse_time(prospective_starts, "starts_at")?;
    let closes = parse_time(prospective_closes, "submissions_close_at")?;
    let ends = parse_time(prospective_ends, "ends_at")?;
    if !(starts < closes && closes <= ends) {
        return Err(ApiError::bad_request(
            "jam timestamps must satisfy starts < submissions_close <= ends",
        ));
    }
    let mut active: jams::ActiveModel = model.into();
    if let Some(v) = &input.title {
        active.title = Set(v.trim().into());
    }
    if let Some(v) = &input.description {
        active.description = Set(v.trim().into());
    }
    if let Some(v) = &input.rules {
        active.rules = Set(v.trim().into());
    }
    if let Some(v) = &input.starts_at {
        parse_time(v, "starts_at")?;
        active.starts_at = Set(v.clone());
    }
    if let Some(v) = &input.submissions_close_at {
        parse_time(v, "submissions_close_at")?;
        active.submissions_close_at = Set(v.clone());
    }
    if let Some(v) = &input.ends_at {
        parse_time(v, "ends_at")?;
        active.ends_at = Set(v.clone());
    }
    active.updated_at = Set(now());
    let model = active.update(&state.db).await?;
    Ok(Json(jam_info(&state.db, model).await?))
}

#[post("/api/v2/jams/<slug>/entries", data = "<input>")]
pub async fn enter_jam(
    state: &State<PortState>,
    user: AuthUser,
    slug: &str,
    input: Json<JamEntryInput>,
) -> Result<Json<JamInfo>, ApiError> {
    let jam = jam_for_slug(&state.db, slug).await?;
    if jam_status(&jam) != "open" {
        return Err(ApiError::bad_request("jam submissions are not open"));
    }
    let cart = db::get_cart_model(&state.db, &input.cart_id)
        .await?
        .ok_or_else(|| ApiError::not_found("cart not found"))?;
    if cart.owner_id.as_deref() != Some(user.id.as_str()) && !user.is_admin {
        return Err(ApiError::forbidden("not the owner of this cart"));
    }
    let existing = jam_entries::Entity::find()
        .filter(jam_entries::Column::CartId.eq(&input.cart_id))
        .all(&state.db)
        .await?;
    for entry in existing {
        let other = jams::Entity::find_by_id(&entry.jam_id)
            .one(&state.db)
            .await?;
        if let Some(other) = other
            && jam_status(&other) == "open"
            && other.id != jam.id
        {
            return Err(ApiError::bad_request(
                "cart already entered in another open jam",
            ));
        }
        if entry.jam_id == jam.id {
            return Ok(Json(jam_info(&state.db, jam).await?));
        }
    }
    jam_entries::ActiveModel {
        id: Set(Uuid::new_v4().to_string()),
        jam_id: Set(jam.id.clone()),
        cart_id: Set(input.cart_id.clone()),
        user_id: Set(user.id),
        submitted_at: Set(now()),
    }
    .insert(&state.db)
    .await?;
    Ok(Json(jam_info(&state.db, jam).await?))
}

#[delete("/api/v2/jams/<slug>/entries/<cart_id>")]
pub async fn withdraw_jam_entry(
    state: &State<PortState>,
    user: AuthUser,
    slug: &str,
    cart_id: &str,
) -> Result<Json<JamInfo>, ApiError> {
    let jam = jam_for_slug(&state.db, slug).await?;
    if jam_status(&jam) != "open" {
        return Err(ApiError::bad_request("jam submissions are not open"));
    }
    let entry = jam_entries::Entity::find()
        .filter(jam_entries::Column::JamId.eq(&jam.id))
        .filter(jam_entries::Column::CartId.eq(cart_id))
        .one(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("jam entry not found"))?;
    if entry.user_id != user.id && !user.is_admin {
        return Err(ApiError::forbidden("not the owner of this jam entry"));
    }
    jam_entries::Entity::delete_by_id(entry.id)
        .exec(&state.db)
        .await?;
    Ok(Json(jam_info(&state.db, jam).await?))
}

#[get("/api/v2/feed?<page>&<per_page>")]
pub async fn feed(
    state: &State<PortState>,
    user: AuthUser,
    page: Option<u32>,
    per_page: Option<u32>,
) -> Result<Json<FeedPage>, ApiError> {
    let followed = follows::Entity::find()
        .filter(follows::Column::FollowerId.eq(&user.id))
        .all(&state.db)
        .await?;
    let followed_ids: HashSet<_> = followed.into_iter().map(|f| f.followed_id).collect();
    let followed_collections: HashSet<_> = collection_follows::Entity::find()
        .filter(collection_follows::Column::UserId.eq(&user.id))
        .all(&state.db)
        .await?
        .into_iter()
        .map(|f| f.collection_id)
        .collect();
    let all_users: HashMap<_, _> = users::Entity::find()
        .all(&state.db)
        .await?
        .into_iter()
        .map(|u| (u.id, u.username))
        .collect();
    let mut events = Vec::new();
    let carts = crate::entities::carts::Entity::find()
        .all(&state.db)
        .await?;
    for cart_model in carts {
        let Some(owner_id) = cart_model.owner_id.as_ref() else {
            continue;
        };
        if followed_ids.contains(owner_id) {
            let actor = all_users
                .get(owner_id)
                .cloned()
                .unwrap_or_else(|| cart_model.author.clone());
            let Some(cart) = db::get(&state.db, &cart_model.id).await? else {
                continue;
            };
            events.push(FeedEvent {
                kind: "cart_published".into(),
                actor: actor.clone(),
                occurred_at: cart_model.uploaded_at.clone(),
                cart: cart.clone(),
                version: None,
                collection_slug: None,
                collection_title: None,
                jam_slug: None,
                jam_title: None,
            });
            let versions = cart_versions::Entity::find()
                .filter(cart_versions::Column::CartId.eq(&cart_model.id))
                .filter(cart_versions::Column::Version.gt(1))
                .all(&state.db)
                .await?;
            for version in versions {
                events.push(FeedEvent {
                    kind: "version_published".into(),
                    actor: actor.clone(),
                    occurred_at: version.created_at,
                    cart: cart.clone(),
                    version: Some(version.version),
                    collection_slug: None,
                    collection_title: None,
                    jam_slug: None,
                    jam_title: None,
                });
            }
        }
    }
    for collection_id in followed_collections {
        let Some(collection) = collections::Entity::find_by_id(&collection_id)
            .one(&state.db)
            .await?
        else {
            continue;
        };
        let actor = all_users
            .get(&collection.owner_id)
            .cloned()
            .unwrap_or_else(|| "unknown".into());
        let rows = collection_carts::Entity::find()
            .filter(collection_carts::Column::CollectionId.eq(&collection_id))
            .all(&state.db)
            .await?;
        for row in rows {
            if let Some(cart) = db::get(&state.db, &row.cart_id).await? {
                events.push(FeedEvent {
                    kind: "collection_addition".into(),
                    actor: actor.clone(),
                    occurred_at: row.added_at,
                    cart,
                    version: None,
                    collection_slug: Some(collection.slug.clone()),
                    collection_title: Some(collection.title.clone()),
                    jam_slug: None,
                    jam_title: None,
                });
            }
        }
    }
    let entries = jam_entries::Entity::find().all(&state.db).await?;
    for entry in entries {
        if !followed_ids.contains(&entry.user_id) {
            continue;
        }
        let Some(jam) = jams::Entity::find_by_id(&entry.jam_id)
            .one(&state.db)
            .await?
        else {
            continue;
        };
        let Some(cart) = db::get(&state.db, &entry.cart_id).await? else {
            continue;
        };
        events.push(FeedEvent {
            kind: "jam_entry".into(),
            actor: all_users
                .get(&entry.user_id)
                .cloned()
                .unwrap_or_else(|| "unknown".into()),
            occurred_at: entry.submitted_at,
            cart,
            version: None,
            collection_slug: None,
            collection_title: None,
            jam_slug: Some(jam.slug),
            jam_title: Some(jam.title),
        });
    }
    events.sort_by(|a, b| b.occurred_at.cmp(&a.occurred_at));
    let total = events.len() as u64;
    let page = page.unwrap_or(0);
    let per_page = per_page.unwrap_or(20).min(100);
    let events = events
        .into_iter()
        .skip(page as usize * per_page as usize)
        .take(per_page as usize)
        .collect();
    Ok(Json(FeedPage {
        events,
        page,
        per_page,
        total,
    }))
}

#[get("/api/v2/dashboard")]
pub async fn dashboard(
    state: &State<PortState>,
    user: AuthUser,
) -> Result<Json<DashboardInfo>, ApiError> {
    let cart_models = crate::entities::carts::Entity::find()
        .filter(crate::entities::carts::Column::OwnerId.eq(&user.id))
        .all(&state.db)
        .await?;
    let cart_ids: HashSet<_> = cart_models.iter().map(|c| c.id.as_str()).collect();
    let mut carts = Vec::with_capacity(cart_models.len());
    for model in &cart_models {
        if let Some(cart) = db::get(&state.db, &model.id).await? {
            carts.push(cart);
        }
    }
    let events: Vec<_> = play_events::Entity::find()
        .all(&state.db)
        .await?
        .into_iter()
        .filter(|e| cart_ids.contains(e.cart_id.as_str()))
        .collect();
    let now = Utc::now();
    let current_start = now - Duration::days(30);
    let previous_start = now - Duration::days(60);
    let mut current = Vec::new();
    let mut previous = Vec::new();
    for event in &events {
        if let Ok(stamp) =
            DateTime::parse_from_rfc3339(&event.played_at).map(|d| d.with_timezone(&Utc))
        {
            if stamp >= current_start {
                current.push(event);
            } else if stamp >= previous_start {
                previous.push(event);
            }
        }
    }
    let current_viewers: HashSet<_> = current.iter().map(|e| e.viewer_key.as_str()).collect();
    let previous_viewers: HashSet<_> = previous.iter().map(|e| e.viewer_key.as_str()).collect();
    let mut daily = Vec::new();
    for offset in (0..30).rev() {
        let date = (now - Duration::days(offset)).date_naive();
        let day: Vec<_> = current
            .iter()
            .filter(|e| {
                DateTime::parse_from_rfc3339(&e.played_at)
                    .ok()
                    .map(|d| d.date_naive())
                    == Some(date)
            })
            .collect();
        let viewers: HashSet<_> = day.iter().map(|e| e.viewer_key.as_str()).collect();
        daily.push(DailyMetric {
            date: date.to_string(),
            plays: day.len() as i64,
            unique_players: viewers.len() as i64,
        });
    }
    let rating_count: i64 = cart_models.iter().map(|c| c.rating_count).sum();
    let rating_sum: i64 = cart_models.iter().map(|c| c.rating_sum).sum();
    let follower_rows = follows::Entity::find()
        .filter(follows::Column::FollowedId.eq(&user.id))
        .all(&state.db)
        .await?;
    let new_followers = follower_rows
        .iter()
        .filter(|f| {
            DateTime::parse_from_rfc3339(&f.created_at)
                .ok()
                .map(|d| d.with_timezone(&Utc) >= current_start)
                .unwrap_or(false)
        })
        .count() as i64;
    Ok(Json(DashboardInfo {
        plays: MetricWindow {
            current: current.len() as i64,
            previous: previous.len() as i64,
        },
        unique_players: MetricWindow {
            current: current_viewers.len() as i64,
            previous: previous_viewers.len() as i64,
        },
        rating_avg: if rating_count > 0 {
            rating_sum as f64 / rating_count as f64
        } else {
            0.0
        },
        followers: follower_rows.len() as i64,
        new_followers,
        daily,
        carts,
    }))
}
