//! Admin-only user management: search/list, ban/unban, promote/demote.
//! Every route here takes [`AdminUser`] so admin-gating is one consistent
//! guard instead of scattered inline `if !user.is_admin` checks.

use chrono::Utc;
use rocket::{State, get, post, serde::json::Json};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, ConnectionTrait, DatabaseBackend, EntityTrait,
    PaginatorTrait, QueryFilter, QueryOrder, Set, TransactionTrait, sea_query::Expr,
};
use uuid::Uuid;

use crate::{
    PortState,
    auth::{AdminUser, delete_all_sessions},
    entities::{carts, moderation_actions, users},
    error::ApiError,
    models::{AdminUserInfo, AdminUserList, BanInput},
};

fn now() -> String {
    Utc::now().to_rfc3339()
}

async fn log_action(
    db: &sea_orm::DatabaseConnection,
    actor_id: &str,
    action: &str,
    target_type: &str,
    target_id: &str,
    reason: Option<&str>,
) {
    let result = moderation_actions::ActiveModel {
        id: Set(Uuid::new_v4().to_string()),
        actor_id: Set(actor_id.to_string()),
        action: Set(action.to_string()),
        target_type: Set(target_type.to_string()),
        target_id: Set(target_id.to_string()),
        reason: Set(reason.map(str::to_string)),
        created_at: Set(now()),
    }
    .insert(db)
    .await;
    if let Err(e) = result {
        log::error!("failed to write moderation action ({action}) on {target_id}: {e}");
    }
}

async fn to_admin_user_info(
    db: &sea_orm::DatabaseConnection,
    m: users::Model,
) -> Result<AdminUserInfo, ApiError> {
    let cart_count = carts::Entity::find()
        .filter(carts::Column::OwnerId.eq(m.id.clone()))
        .count(db)
        .await?;
    Ok(AdminUserInfo {
        id: m.id,
        username: m.username,
        email: m.email,
        is_admin: m.is_admin,
        is_banned: m.is_banned,
        banned_reason: m.banned_reason,
        created_at: m.created_at,
        cart_count,
    })
}

#[get("/api/v2/admin/users?<page>&<per_page>&<q>&<filter>")]
pub async fn list_users(
    state: &State<PortState>,
    _admin: AdminUser,
    page: Option<u32>,
    per_page: Option<u32>,
    q: Option<String>,
    filter: Option<String>,
) -> Result<Json<AdminUserList>, ApiError> {
    let page = page.unwrap_or(0);
    let per_page = per_page.unwrap_or(20).min(100);

    let mut select = users::Entity::find();
    if let Some(q) = q.as_deref().filter(|s| !s.trim().is_empty()) {
        // `LOWER(...) LIKE` rather than `.contains()` so the match is
        // case-insensitive on both backends — plain `LIKE` is
        // case-insensitive on SQLite but case-sensitive on Postgres, which
        // would make this search behave differently between dev and prod.
        let needle = format!("%{}%", q.to_lowercase());
        select = select.filter(
            Condition::any()
                .add(Expr::cust_with_values(
                    "LOWER(username) LIKE ?",
                    [needle.clone()],
                ))
                .add(Expr::cust_with_values("LOWER(email) LIKE ?", [needle])),
        );
    }
    select = match filter.as_deref() {
        Some("banned") => select.filter(users::Column::IsBanned.eq(true)),
        Some("admin") => select.filter(users::Column::IsAdmin.eq(true)),
        _ => select,
    };
    select = select.order_by_asc(users::Column::CreatedAt);

    let pager = select.paginate(&state.db, per_page as u64);
    let total = pager.num_items().await?;
    let items = pager.fetch_page(page as u64).await?;

    let mut out = Vec::with_capacity(items.len());
    for m in items {
        out.push(to_admin_user_info(&state.db, m).await?);
    }
    Ok(Json(AdminUserList {
        users: out,
        total,
        page,
        per_page,
    }))
}

async fn find_user(db: &sea_orm::DatabaseConnection, id: &str) -> Result<users::Model, ApiError> {
    users::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| ApiError::not_found("user not found"))
}

#[post("/api/v2/admin/users/<id>/ban", data = "<input>")]
pub async fn ban_user(
    state: &State<PortState>,
    admin: AdminUser,
    id: &str,
    input: Json<BanInput>,
) -> Result<Json<AdminUserInfo>, ApiError> {
    let admin = admin.0;
    if id == admin.id {
        return Err(ApiError::bad_request("cannot ban your own account"));
    }
    let reason = input.reason.trim();
    if reason.is_empty() || reason.chars().count() > 500 {
        return Err(ApiError::bad_request("ban reason must be 1-500 chars"));
    }
    let model = find_user(&state.db, id).await?;
    let mut active: users::ActiveModel = model.into();
    active.is_banned = Set(true);
    active.banned_at = Set(Some(now()));
    active.banned_reason = Set(Some(reason.to_string()));
    active.banned_by = Set(Some(admin.id.clone()));
    let model = active.update(&state.db).await?;

    delete_all_sessions(&state.db, id).await?;
    log_action(&state.db, &admin.id, "ban_user", "user", id, Some(reason)).await;

    Ok(Json(to_admin_user_info(&state.db, model).await?))
}

#[post("/api/v2/admin/users/<id>/unban")]
pub async fn unban_user(
    state: &State<PortState>,
    admin: AdminUser,
    id: &str,
) -> Result<Json<AdminUserInfo>, ApiError> {
    let admin = admin.0;
    let model = find_user(&state.db, id).await?;
    if !model.is_banned {
        // Already unbanned: no state change, so no audit-log noise either.
        return Ok(Json(to_admin_user_info(&state.db, model).await?));
    }
    let mut active: users::ActiveModel = model.into();
    active.is_banned = Set(false);
    active.banned_at = Set(None);
    active.banned_reason = Set(None);
    active.banned_by = Set(None);
    let model = active.update(&state.db).await?;

    log_action(&state.db, &admin.id, "unban_user", "user", id, None).await;

    Ok(Json(to_admin_user_info(&state.db, model).await?))
}

#[post("/api/v2/admin/users/<id>/promote")]
pub async fn promote_user(
    state: &State<PortState>,
    admin: AdminUser,
    id: &str,
) -> Result<Json<AdminUserInfo>, ApiError> {
    let admin = admin.0;
    let model = find_user(&state.db, id).await?;
    if model.is_admin {
        // Already an admin: no state change, so no audit-log noise either.
        return Ok(Json(to_admin_user_info(&state.db, model).await?));
    }
    let mut active: users::ActiveModel = model.into();
    active.is_admin = Set(true);
    let model = active.update(&state.db).await?;

    log_action(&state.db, &admin.id, "promote_admin", "user", id, None).await;

    Ok(Json(to_admin_user_info(&state.db, model).await?))
}

#[post("/api/v2/admin/users/<id>/demote")]
pub async fn demote_user(
    state: &State<PortState>,
    admin: AdminUser,
    id: &str,
) -> Result<Json<AdminUserInfo>, ApiError> {
    let admin = admin.0;

    // The last-remaining-admin check below only holds if concurrent
    // demotes can't both read a stale count and both proceed. On Postgres,
    // lock every functional-admin row for the transaction's duration so a
    // second concurrent demote blocks until this one commits or rolls
    // back; SQLite has no `FOR UPDATE` and serializes writers at the
    // connection level regardless, so the lock is skipped there.
    let txn = state.db.begin().await?;
    if txn.get_database_backend() == DatabaseBackend::Postgres {
        txn.execute_unprepared(
            "SELECT id FROM users WHERE is_admin = true AND is_banned = false FOR UPDATE",
        )
        .await?;
    }

    let model = users::Entity::find_by_id(id)
        .one(&txn)
        .await?
        .ok_or_else(|| ApiError::not_found("user not found"))?;
    if !model.is_admin {
        return Err(ApiError::bad_request("user is not an admin"));
    }
    // Count only *functional* admins — one who is `is_admin` but banned
    // can never authenticate, so counting them here would let an admin ban
    // a co-admin and then demote themselves, leaving the port with zero
    // admins anyone can actually reach.
    let functional_admin_count = users::Entity::find()
        .filter(users::Column::IsAdmin.eq(true))
        .filter(users::Column::IsBanned.eq(false))
        .count(&txn)
        .await?;
    if functional_admin_count <= 1 {
        return Err(ApiError::conflict("cannot demote the last remaining admin"));
    }
    let mut active: users::ActiveModel = model.into();
    active.is_admin = Set(false);
    let model = active.update(&txn).await?;
    txn.commit().await?;

    log_action(&state.db, &admin.id, "demote_admin", "user", id, None).await;

    Ok(Json(to_admin_user_info(&state.db, model).await?))
}
