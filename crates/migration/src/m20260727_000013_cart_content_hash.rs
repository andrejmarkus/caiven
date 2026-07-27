use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::{ConnectionTrait, Statement};

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260727_000013_cart_content_hash"
    }
}

#[derive(Iden)]
enum CartVersions {
    Table,
    ContentHash,
}

async fn backfill_blob_hashes(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    let connection = manager.get_connection();
    let backend = manager.get_database_backend();
    let rows = connection
        .query_all(Statement::from_string(
            backend,
            "SELECT cart_versions.id, cart_blobs.cart_data \
             FROM cart_versions \
             INNER JOIN cart_blobs ON cart_blobs.version_id = cart_versions.id",
        ))
        .await?;

    for row in rows {
        let id: String = row.try_get("", "id")?;
        let cart_data: Vec<u8> = row.try_get("", "cart_data")?;
        let content_hash = caiven_cart::content_hash(&cart_data).map_err(|error| {
            DbErr::Custom(format!("cannot hash existing cart version {id}: {error}"))
        })?;
        let sql = match backend {
            sea_orm_migration::sea_orm::DatabaseBackend::Postgres => {
                "UPDATE cart_versions SET content_hash = $1 WHERE id = $2"
            }
            _ => "UPDATE cart_versions SET content_hash = ? WHERE id = ?",
        };
        connection
            .execute(Statement::from_sql_and_values(
                backend,
                sql,
                [content_hash.into(), id.into()],
            ))
            .await?;
    }
    Ok(())
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(CartVersions::Table)
                    .add_column(ColumnDef::new(CartVersions::ContentHash).string().null())
                    .to_owned(),
            )
            .await?;
        backfill_blob_hashes(manager).await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_cart_versions_content_hash")
                    .table(CartVersions::Table)
                    .col(CartVersions::ContentHash)
                    .to_owned(),
            )
            .await
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("idx_cart_versions_content_hash")
                    .table(CartVersions::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(CartVersions::Table)
                    .drop_column(CartVersions::ContentHash)
                    .to_owned(),
            )
            .await
    }
}
