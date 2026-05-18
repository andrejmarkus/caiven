use anyhow::Result;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseConnection, EntityTrait,
    PaginatorTrait, QueryFilter, QueryOrder, Set,
    sea_query::Expr,
};

use crate::entities::carts::{self, ActiveModel, Entity as CartEntity};
use crate::models::{Cart, CartMeta};

pub async fn insert(db: &DatabaseConnection, id: &str, meta: &CartMeta, rom_size: usize) -> Result<()> {
    let now = chrono::Utc::now().to_rfc3339();
    ActiveModel {
        id: Set(id.to_string()),
        title: Set(meta.title.clone()),
        author: Set(meta.author.clone()),
        description: Set(meta.description.clone()),
        tags: Set(meta.tags.iter().map(|t| t.replace(',', " ")).collect::<Vec<_>>().join(",")),
        uploaded_at: Set(now),
        downloads: Set(0),
        has_screenshot: Set(false),
        rom_size: Set(rom_size as i64),
    }
    .insert(db)
    .await?;
    Ok(())
}

pub async fn get(db: &DatabaseConnection, id: &str) -> Result<Option<Cart>> {
    Ok(CartEntity::find_by_id(id).one(db).await?.map(Cart::from))
}

pub async fn list(
    db: &DatabaseConnection,
    page: u32,
    per_page: u32,
    query: Option<&str>,
) -> Result<(Vec<Cart>, u64)> {
    let select = if let Some(q) = query {
        CartEntity::find().filter(
            Condition::any()
                .add(carts::Column::Title.contains(q))
                .add(carts::Column::Author.contains(q))
                .add(carts::Column::Description.contains(q)),
        )
    } else {
        CartEntity::find()
    }
    .order_by_desc(carts::Column::UploadedAt);

    let pager = select.paginate(db, per_page as u64);
    let total = pager.num_items().await?;
    let items = pager.fetch_page(page as u64).await?;
    Ok((items.into_iter().map(Cart::from).collect(), total))
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

pub async fn set_has_screenshot(db: &DatabaseConnection, id: &str) -> Result<()> {
    CartEntity::update_many()
        .col_expr(carts::Column::HasScreenshot, Expr::value(true))
        .filter(carts::Column::Id.eq(id))
        .exec(db)
        .await?;
    Ok(())
}
