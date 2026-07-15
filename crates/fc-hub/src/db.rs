use anyhow::Result;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseConnection, EntityTrait, PaginatorTrait,
    QueryFilter, QueryOrder, Set,
    sea_query::{Expr, Order},
};

use crate::entities::{
    cart_versions::{self, Entity as CartVersionEntity},
    carts::{self, Entity as CartEntity},
    users::{self, Entity as UserEntity},
};
use crate::models::{Cart, CartMeta, CartPatch, TagCount};

fn normalize_tags(tags: &[String]) -> String {
    tags.iter()
        .map(|t| t.replace(',', " ").trim().to_lowercase())
        .filter(|t| !t.is_empty())
        .collect::<Vec<_>>()
        .join(",")
}

pub fn rom_rel_path(cart_id: &str, version: i32) -> String {
    if version <= 1 {
        format!("roms/{cart_id}.rom")
    } else {
        format!("roms/{cart_id}-v{version}.rom")
    }
}

pub fn screenshot_rel_path(cart_id: &str, version: i32) -> String {
    if version <= 1 {
        format!("screenshots/{cart_id}.png")
    } else {
        format!("screenshots/{cart_id}-v{version}.png")
    }
}

/// Create a new cart owned by `owner_id`, plus its version-1 row. Returns the
/// new cart id.
pub async fn insert_cart(
    db: &DatabaseConnection,
    owner_id: &str,
    id: &str,
    meta: &CartMeta,
    rom_size: usize,
) -> Result<()> {
    let now = chrono::Utc::now().to_rfc3339();
    carts::ActiveModel {
        id: Set(id.to_string()),
        title: Set(meta.title.clone()),
        author: Set(meta.author.clone()),
        description: Set(meta.description.clone()),
        tags: Set(normalize_tags(&meta.tags)),
        uploaded_at: Set(now.clone()),
        downloads: Set(0),
        owner_id: Set(Some(owner_id.to_string())),
        rating_count: Set(0),
        rating_sum: Set(0),
    }
    .insert(db)
    .await?;

    cart_versions::ActiveModel {
        id: Set(uuid::Uuid::new_v4().to_string()),
        cart_id: Set(id.to_string()),
        version: Set(1),
        rom_path: Set(rom_rel_path(id, 1)),
        rom_size: Set(rom_size as i64),
        changelog: Set(String::new()),
        has_screenshot: Set(false),
        created_at: Set(now),
    }
    .insert(db)
    .await?;
    Ok(())
}

/// Add a new version to an existing cart. Returns the new version number.
pub async fn insert_version(
    db: &DatabaseConnection,
    cart_id: &str,
    changelog: &str,
    rom_size: usize,
) -> Result<i32> {
    let next = latest_version(db, cart_id)
        .await?
        .map(|v| v.version + 1)
        .unwrap_or(1);
    cart_versions::ActiveModel {
        id: Set(uuid::Uuid::new_v4().to_string()),
        cart_id: Set(cart_id.to_string()),
        version: Set(next),
        rom_path: Set(rom_rel_path(cart_id, next)),
        rom_size: Set(rom_size as i64),
        changelog: Set(changelog.to_string()),
        has_screenshot: Set(false),
        created_at: Set(chrono::Utc::now().to_rfc3339()),
    }
    .insert(db)
    .await?;
    Ok(next)
}

pub async fn get_cart_model(db: &DatabaseConnection, id: &str) -> Result<Option<carts::Model>> {
    Ok(CartEntity::find_by_id(id).one(db).await?)
}

pub async fn owner_username(db: &DatabaseConnection, owner_id: Option<&str>) -> Result<Option<String>> {
    let Some(owner_id) = owner_id else {
        return Ok(None);
    };
    Ok(UserEntity::find_by_id(owner_id)
        .one(db)
        .await?
        .map(|u| u.username))
}

pub async fn latest_version(
    db: &DatabaseConnection,
    cart_id: &str,
) -> Result<Option<cart_versions::Model>> {
    Ok(CartVersionEntity::find()
        .filter(cart_versions::Column::CartId.eq(cart_id))
        .order_by_desc(cart_versions::Column::Version)
        .one(db)
        .await?)
}

pub async fn get_version(
    db: &DatabaseConnection,
    cart_id: &str,
    version: i32,
) -> Result<Option<cart_versions::Model>> {
    Ok(CartVersionEntity::find()
        .filter(cart_versions::Column::CartId.eq(cart_id))
        .filter(cart_versions::Column::Version.eq(version))
        .one(db)
        .await?)
}

pub async fn list_versions(
    db: &DatabaseConnection,
    cart_id: &str,
) -> Result<Vec<cart_versions::Model>> {
    Ok(CartVersionEntity::find()
        .filter(cart_versions::Column::CartId.eq(cart_id))
        .order_by_asc(cart_versions::Column::Version)
        .all(db)
        .await?)
}

async fn to_cart(db: &DatabaseConnection, m: carts::Model) -> Result<Cart> {
    let latest = latest_version(db, &m.id).await?;
    let owner = owner_username(db, m.owner_id.as_deref()).await?;
    Ok(Cart::from_model(m, owner, latest.as_ref()))
}

pub async fn get(db: &DatabaseConnection, id: &str) -> Result<Option<Cart>> {
    let Some(m) = get_cart_model(db, id).await? else {
        return Ok(None);
    };
    Ok(Some(to_cart(db, m).await?))
}

pub enum Sort {
    New,
    Popular,
    Top,
}

impl Sort {
    pub fn parse(s: Option<&str>) -> Self {
        match s {
            Some("popular") => Sort::Popular,
            Some("top") => Sort::Top,
            _ => Sort::New,
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn list(
    db: &DatabaseConnection,
    page: u32,
    per_page: u32,
    query: Option<&str>,
    tag: Option<&str>,
    author: Option<&str>,
    sort: Sort,
) -> Result<(Vec<Cart>, u64)> {
    let mut select = CartEntity::find();

    if let Some(q) = query {
        select = select.filter(
            Condition::any()
                .add(carts::Column::Title.contains(q))
                .add(carts::Column::Author.contains(q))
                .add(carts::Column::Description.contains(q)),
        );
    }
    if let Some(tag) = tag {
        let needle = format!("%,{},%", tag.trim().to_lowercase());
        select = select.filter(Expr::cust_with_values(
            "(',' || tags || ',') LIKE ?",
            [needle],
        ));
    }
    if let Some(author) = author {
        select = select.filter(carts::Column::Author.eq(author));
    }

    select = match sort {
        Sort::New => select.order_by_desc(carts::Column::UploadedAt),
        Sort::Popular => select.order_by_desc(carts::Column::Downloads),
        Sort::Top => select
            .order_by(
                Expr::cust("CAST(rating_sum AS REAL) / MAX(rating_count, 1)"),
                Order::Desc,
            )
            .order_by_desc(carts::Column::RatingCount),
    };

    let pager = select.paginate(db, per_page as u64);
    let total = pager.num_items().await?;
    let items = pager.fetch_page(page as u64).await?;

    let mut carts = Vec::with_capacity(items.len());
    for m in items {
        carts.push(to_cart(db, m).await?);
    }
    Ok((carts, total))
}

pub async fn increment_downloads(db: &DatabaseConnection, id: &str) -> Result<()> {
    CartEntity::update_many()
        .col_expr(
            carts::Column::Downloads,
            Expr::col(carts::Column::Downloads).add(1),
        )
        .filter(carts::Column::Id.eq(id))
        .exec(db)
        .await?;
    Ok(())
}

pub async fn set_version_has_screenshot(
    db: &DatabaseConnection,
    cart_id: &str,
    version: i32,
) -> Result<()> {
    CartVersionEntity::update_many()
        .col_expr(cart_versions::Column::HasScreenshot, Expr::value(true))
        .filter(cart_versions::Column::CartId.eq(cart_id))
        .filter(cart_versions::Column::Version.eq(version))
        .exec(db)
        .await?;
    Ok(())
}

pub async fn update_cart(db: &DatabaseConnection, id: &str, patch: &CartPatch) -> Result<()> {
    let Some(m) = get_cart_model(db, id).await? else {
        return Ok(());
    };
    let mut active: carts::ActiveModel = m.into();
    if let Some(title) = &patch.title {
        active.title = Set(title.clone());
    }
    if let Some(description) = &patch.description {
        active.description = Set(description.clone());
    }
    if let Some(tags) = &patch.tags {
        active.tags = Set(normalize_tags(tags));
    }
    active.update(db).await?;
    Ok(())
}

/// Delete a cart, its versions, and return the relative file paths (rom +
/// screenshot, if present) of every version so the caller can remove them
/// from disk. SQLite doesn't enforce the `ON DELETE CASCADE` on
/// `cart_versions` unless foreign keys are pragma-enabled, so versions are
/// deleted explicitly here rather than relied upon.
pub async fn delete_cart(db: &DatabaseConnection, id: &str) -> Result<Vec<(String, Option<String>)>> {
    let versions = list_versions(db, id).await?;
    let paths = versions
        .iter()
        .map(|v| {
            let screenshot = v.has_screenshot.then(|| screenshot_rel_path(id, v.version));
            (v.rom_path.clone(), screenshot)
        })
        .collect();

    CartVersionEntity::delete_many()
        .filter(cart_versions::Column::CartId.eq(id))
        .exec(db)
        .await?;
    CartEntity::delete_by_id(id).exec(db).await?;
    Ok(paths)
}

pub async fn list_tags(db: &DatabaseConnection) -> Result<Vec<TagCount>> {
    let carts = CartEntity::find().all(db).await?;
    let mut counts: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    for c in carts {
        for tag in c.tags.split(',') {
            let tag = tag.trim();
            if !tag.is_empty() {
                *counts.entry(tag.to_string()).or_insert(0) += 1;
            }
        }
    }
    let mut out: Vec<TagCount> = counts
        .into_iter()
        .map(|(tag, count)| TagCount { tag, count })
        .collect();
    out.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.tag.cmp(&b.tag)));
    Ok(out)
}

pub async fn get_user_by_username(
    db: &DatabaseConnection,
    username: &str,
) -> Result<Option<users::Model>> {
    Ok(UserEntity::find()
        .filter(users::Column::Username.eq(username))
        .one(db)
        .await?)
}

pub async fn list_by_owner(
    db: &DatabaseConnection,
    owner_id: &str,
    page: u32,
    per_page: u32,
) -> Result<(Vec<Cart>, u64)> {
    let select = CartEntity::find()
        .filter(carts::Column::OwnerId.eq(owner_id))
        .order_by_desc(carts::Column::UploadedAt);
    let pager = select.paginate(db, per_page as u64);
    let total = pager.num_items().await?;
    let items = pager.fetch_page(page as u64).await?;
    let mut carts = Vec::with_capacity(items.len());
    for m in items {
        carts.push(to_cart(db, m).await?);
    }
    Ok((carts, total))
}
