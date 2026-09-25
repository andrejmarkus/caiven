//! First-party remix-loop measurement. Stores one row per (cart, step,
//! hashed viewer) — no timings, no paths, no raw IPs — so the table answers
//! "how many people reached this step" and nothing finer. Removing it means
//! dropping these two routes and the `funnel_events` table; nothing else
//! reads it. See `docs/product/quick-remix.md`.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Duration, Utc};
use rocket::{State, get, http::Status, post, serde::json::Json};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QuerySelect, Set,
};
use uuid::Uuid;

use super::valid_id;
use crate::{
    PortState,
    auth::{AdminUser, AuthUser, ClientIp, sha256_hex},
    db,
    entities::{carts, funnel_events, play_events, users},
    error::ApiError,
    models::{CartFunnel, FunnelConversion, FunnelInput, RemixFunnel},
};

/// Plenty for every seed and remix in a small test; totals cover the rest.
const MAX_FUNNEL_ROWS: usize = 200;

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

fn ratio(n: u64, d: u64) -> Option<f64> {
    (d > 0).then(|| (n as f64 / d as f64 * 1000.0).round() / 1000.0)
}

fn conversion(row: &CartFunnel) -> FunnelConversion {
    FunnelConversion {
        qualified_play_to_remix_opened: ratio(row.remix_opened, row.qualified_plays),
        remix_opened_to_remix_ran: ratio(row.remix_ran, row.remix_opened),
        remix_ran_to_publish_started: ratio(row.publish_started, row.remix_ran),
        publish_started_to_remix_published: ratio(row.remixes_published, row.publish_started),
        remix_published_to_external_play: ratio(
            row.remixes_with_external_play,
            row.remixes_published,
        ),
        remix_published_to_remixed: ratio(row.remixes_remixed, row.remixes_published),
    }
}

/// `since` (RFC 3339) pins the window to an experiment's start; otherwise the
/// last `days`. Staff accounts are left out by default: whoever runs a test
/// also plays, remixes and publishes while checking on it.
#[get("/api/v2/admin/metrics/remix-funnel?<days>&<since>&<include_staff>")]
pub async fn remix_funnel(
    state: &State<PortState>,
    _admin: AdminUser,
    days: Option<i64>,
    since: Option<&str>,
    include_staff: Option<bool>,
) -> Result<Json<RemixFunnel>, ApiError> {
    let include_staff = include_staff.unwrap_or(false);
    // RFC 3339 UTC strings from `to_rfc3339` sort chronologically.
    let (days, since) = match since {
        Some(raw) => {
            let at = DateTime::parse_from_rfc3339(raw)
                .map_err(|_| ApiError::bad_request("since must be an RFC 3339 timestamp"))?;
            (None, at.with_timezone(&Utc).to_rfc3339())
        }
        None => {
            let days = days.unwrap_or(7).clamp(1, 90);
            (Some(days), (Utc::now() - Duration::days(days)).to_rfc3339())
        }
    };

    let staff_ids: HashSet<String> = if include_staff {
        HashSet::new()
    } else {
        users::Entity::find()
            .filter(users::Column::IsAdmin.eq(true))
            .all(&state.db)
            .await?
            .into_iter()
            .map(|u| u.id)
            .collect()
    };
    let staff_keys: HashSet<String> = staff_ids
        .iter()
        .map(|id| sha256_hex(&format!("user:{id}")))
        .collect();
    let by_staff = |owner: &Option<String>| owner.as_ref().is_some_and(|o| staff_ids.contains(o));

    let mut rows: HashMap<String, CartFunnel> = HashMap::new();
    let events: Vec<(String, String, String)> = funnel_events::Entity::find()
        .select_only()
        .column(funnel_events::Column::CartId)
        .column(funnel_events::Column::Event)
        .column(funnel_events::Column::ViewerKey)
        .filter(funnel_events::Column::CreatedAt.gte(&since))
        .into_tuple()
        .all(&state.db)
        .await?;
    for (cart_id, event, viewer) in events {
        if staff_keys.contains(&viewer) {
            continue;
        }
        let row = rows.entry(cart_id).or_default();
        match event.as_str() {
            "qualified_play" => row.qualified_plays += 1,
            "remix_opened" => row.remix_opened += 1,
            "remix_ran" => row.remix_ran += 1,
            "publish_started" => row.publish_started += 1,
            _ => {}
        }
    }
    let plays: Vec<(String, String)> = play_events::Entity::find()
        .select_only()
        .column(play_events::Column::CartId)
        .column(play_events::Column::ViewerKey)
        .filter(play_events::Column::PlayedAt.gte(&since))
        .into_tuple()
        .all(&state.db)
        .await?;
    for (cart_id, viewer) in plays {
        if !staff_keys.contains(&viewer) {
            rows.entry(cart_id).or_default().plays += 1;
        }
    }

    let published: Vec<carts::Model> = carts::Entity::find()
        .filter(carts::Column::UploadedAt.gte(&since))
        .all(&state.db)
        .await?
        .into_iter()
        .filter(|c| !by_staff(&c.owner_id))
        .collect();
    let published_ids: Vec<&str> = published.iter().map(|c| c.id.as_str()).collect();
    let mut external_viewers: HashMap<&str, HashSet<String>> = HashMap::new();
    let mut remixed: HashSet<String> = HashSet::new();
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
                && !staff_keys.contains(&e.viewer_key)
            {
                external_viewers
                    .entry(cart_id)
                    .or_default()
                    .insert(e.viewer_key);
            }
        }
        remixed = carts::Entity::find()
            .filter(carts::Column::ParentCartId.is_in(published_ids.iter().copied()))
            .all(&state.db)
            .await?
            .into_iter()
            .filter(|c| !by_staff(&c.owner_id))
            .filter_map(|c| c.parent_cart_id)
            .collect();
    }

    for c in &published {
        let Some(parent) = &c.parent_cart_id else {
            continue;
        };
        let row = rows.entry(parent.clone()).or_default();
        row.remixes_published += 1;
        row.remixes_with_external_play += u64::from(external_viewers.contains_key(c.id.as_str()));
        row.remixes_remixed += u64::from(remixed.contains(&c.id));
    }

    // Empty lookups skip the query rather than send an empty `IN ()`.
    let mut carts_by_id: HashMap<String, carts::Model> = HashMap::new();
    let mut usernames: HashMap<String, String> = HashMap::new();
    if !rows.is_empty() {
        carts_by_id = carts::Entity::find()
            .filter(carts::Column::Id.is_in(rows.keys().cloned()))
            .all(&state.db)
            .await?
            .into_iter()
            .map(|c| (c.id.clone(), c))
            .collect();
    }
    let owner_ids: HashSet<String> = carts_by_id
        .values()
        .filter_map(|c| c.owner_id.clone())
        .collect();
    if !owner_ids.is_empty() {
        usernames = users::Entity::find()
            .filter(users::Column::Id.is_in(owner_ids))
            .all(&state.db)
            .await?
            .into_iter()
            .map(|u| (u.id, u.username))
            .collect();
    }

    let mut total = CartFunnel::default();
    let mut by_cart: Vec<CartFunnel> = rows
        .into_iter()
        .map(|(cart_id, mut row)| {
            if let Some(cart) = carts_by_id.get(&cart_id) {
                row.title = cart.title.clone();
                row.owner = cart
                    .owner_id
                    .as_ref()
                    .and_then(|o| usernames.get(o))
                    .cloned();
                row.parent_cart_id = cart.parent_cart_id.clone();
            }
            row.cart_id = cart_id;
            row.conversion = conversion(&row);
            total.plays += row.plays;
            total.qualified_plays += row.qualified_plays;
            total.remix_opened += row.remix_opened;
            total.remix_ran += row.remix_ran;
            total.publish_started += row.publish_started;
            total.remixes_published += row.remixes_published;
            total.remixes_with_external_play += row.remixes_with_external_play;
            total.remixes_remixed += row.remixes_remixed;
            row
        })
        .collect();
    by_cart.sort_by(|a, b| {
        (b.qualified_plays, b.plays, b.remix_opened)
            .cmp(&(a.qualified_plays, a.plays, a.remix_opened))
            .then_with(|| a.cart_id.cmp(&b.cart_id))
    });
    by_cart.truncate(MAX_FUNNEL_ROWS);

    Ok(Json(RemixFunnel {
        days,
        since,
        include_staff,
        plays: total.plays,
        qualified_plays: total.qualified_plays,
        remix_opened: total.remix_opened,
        remix_ran: total.remix_ran,
        publish_started: total.publish_started,
        carts_published: published.len() as u64,
        remixes_published: total.remixes_published,
        social_creations: external_viewers.len() as u64,
        remixes_with_external_play: total.remixes_with_external_play,
        remixes_remixed: total.remixes_remixed,
        conversion: conversion(&total),
        by_cart,
    }))
}
