//! First-party remix-loop measurement. Stores one row per (cart, step,
//! hashed viewer) — no timings, no paths, no raw IPs — so the table answers
//! "how many people reached this step" and nothing finer. Removing it means
//! dropping these two routes and the `funnel_events` table; nothing else
//! reads it. See `docs/product/quick-remix.md`.

use std::collections::{HashMap, HashSet};

use chrono::{Duration, Utc};
use rocket::{State, get, http::Status, post, serde::json::Json};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, Set};
use uuid::Uuid;

use super::valid_id;
use crate::{
    PortState,
    auth::{AdminUser, AuthUser, ClientIp, sha256_hex},
    db,
    entities::{carts, funnel_events, play_events},
    error::ApiError,
    models::{FunnelInput, RemixFunnel},
};

/// Steps the client may report. Publishing and external plays are derived
/// from `carts`/`play_events`, so they are never client-claimed.
pub const FUNNEL_EVENTS: &[&str] = &[
    "qualified_play",
    "remix_opened",
    "remix_ran",
    "publish_started",
];

/// Same keying as `record_play`, so a viewer is one key across both tables.
pub(crate) fn viewer_key(user: Option<&AuthUser>, ip: &ClientIp) -> String {
    sha256_hex(&match user {
        Some(u) => format!("user:{}", u.id),
        None => format!("ip:{}", ip.0),
    })
}

#[post("/api/v2/carts/<id>/funnel", data = "<input>")]
pub async fn record_funnel_event(
    state: &State<PortState>,
    user: Option<AuthUser>,
    ip: ClientIp,
    id: &str,
    input: Json<FunnelInput>,
) -> Result<Status, ApiError> {
    if !valid_id(id) {
        return Err(ApiError::bad_request("invalid id"));
    }
    let Some(event) = FUNNEL_EVENTS.iter().find(|e| **e == input.event) else {
        return Err(ApiError::bad_request("unknown event"));
    };
    if db::get_cart_model(&state.db, id).await?.is_none() {
        return Err(ApiError::not_found("cart not found"));
    }
    let viewer_key = viewer_key(user.as_ref(), &ip);
    let exists = funnel_events::Entity::find()
        .filter(funnel_events::Column::CartId.eq(id))
        .filter(funnel_events::Column::Event.eq(*event))
        .filter(funnel_events::Column::ViewerKey.eq(&viewer_key))
        .count(&state.db)
        .await?
        > 0;
    if !exists {
        // A concurrent duplicate loses on the unique index; that's the dedup
        // working, not a failure worth surfacing.
        let _ = funnel_events::ActiveModel {
            id: Set(Uuid::new_v4().to_string()),
            cart_id: Set(id.to_string()),
            event: Set((*event).to_string()),
            viewer_key: Set(viewer_key),
            created_at: Set(Utc::now().to_rfc3339()),
        }
        .insert(&state.db)
        .await;
    }
    Ok(Status::NoContent)
}

async fn count_event(state: &PortState, event: &str, since: &str) -> Result<u64, ApiError> {
    Ok(funnel_events::Entity::find()
        .filter(funnel_events::Column::Event.eq(event))
        .filter(funnel_events::Column::CreatedAt.gte(since))
        .count(&state.db)
        .await?)
}

#[get("/api/v2/admin/metrics/remix-funnel?<days>")]
pub async fn remix_funnel(
    state: &State<PortState>,
    _admin: AdminUser,
    days: Option<i64>,
) -> Result<Json<RemixFunnel>, ApiError> {
    let days = days.unwrap_or(7).clamp(1, 90);
    // RFC 3339 UTC strings from `to_rfc3339` sort chronologically.
    let since = (Utc::now() - Duration::days(days)).to_rfc3339();

    let plays = play_events::Entity::find()
        .filter(play_events::Column::PlayedAt.gte(&since))
        .count(&state.db)
        .await?;

    let published = carts::Entity::find()
        .filter(carts::Column::UploadedAt.gte(&since))
        .all(&state.db)
        .await?;
    let published_ids: Vec<&str> = published.iter().map(|c| c.id.as_str()).collect();
    let mut external_viewers: HashMap<&str, HashSet<String>> = HashMap::new();
    if !published_ids.is_empty() {
        let owners: HashMap<&str, String> = published
            .iter()
            .map(|c| {
                let owner = c.owner_id.as_deref().unwrap_or_default();
                (c.id.as_str(), sha256_hex(&format!("user:{owner}")))
            })
            .collect();
        let qualified = funnel_events::Entity::find()
            .filter(funnel_events::Column::Event.eq("qualified_play"))
            .filter(funnel_events::Column::CartId.is_in(published_ids.iter().copied()))
            .all(&state.db)
            .await?;
        for e in qualified {
            if let Some((cart_id, owner_key)) = owners.get_key_value(e.cart_id.as_str())
                && *owner_key != e.viewer_key
            {
                external_viewers
                    .entry(cart_id)
                    .or_default()
                    .insert(e.viewer_key);
            }
        }
    }
    let remixes: Vec<_> = published
        .iter()
        .filter(|c| c.parent_cart_id.is_some())
        .collect();
    let mut remixes_remixed = 0;
    for r in &remixes {
        let children = carts::Entity::find()
            .filter(carts::Column::ParentCartId.eq(&r.id))
            .count(&state.db)
            .await?;
        if children > 0 {
            remixes_remixed += 1;
        }
    }

    Ok(Json(RemixFunnel {
        days,
        plays,
        qualified_plays: count_event(state, "qualified_play", &since).await?,
        remix_opened: count_event(state, "remix_opened", &since).await?,
        remix_ran: count_event(state, "remix_ran", &since).await?,
        publish_started: count_event(state, "publish_started", &since).await?,
        carts_published: published.len() as u64,
        remixes_published: remixes.len() as u64,
        social_creations: external_viewers.len() as u64,
        remixes_with_external_play: remixes
            .iter()
            .filter(|r| external_viewers.contains_key(r.id.as_str()))
            .count() as u64,
        remixes_remixed,
    }))
}
