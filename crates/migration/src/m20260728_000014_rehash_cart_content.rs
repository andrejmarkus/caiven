use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::{ConnectionTrait, Statement};

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260728_000014_rehash_cart_content"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
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

        let mut hashes = Vec::with_capacity(rows.len());
        for row in rows {
            let id: String = row.try_get("", "id")?;
            let cart_data: Vec<u8> = row.try_get("", "cart_data")?;
            let content_hash = caiven_cart::content_hash(&cart_data).map_err(|error| {
                DbErr::Custom(format!("cannot rehash existing cart version {id}: {error}"))
            })?;
            hashes.push((id, content_hash));
        }

        // Legacy-file bytes are unavailable to migrations. Mark their hashes
        // stale so the startup filesystem pass recomputes them before serving.
        connection
            .execute(Statement::from_string(
                backend,
                "UPDATE cart_versions SET content_hash = NULL \
                 WHERE legacy_cart_path IS NOT NULL",
            ))
            .await?;

        let sql = match backend {
            sea_orm_migration::sea_orm::DatabaseBackend::Postgres => {
                "UPDATE cart_versions SET content_hash = $1 WHERE id = $2"
            }
            _ => "UPDATE cart_versions SET content_hash = ? WHERE id = ?",
        };
        for (id, content_hash) in hashes {
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

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // Old hashes cannot be reconstructed from canonical hashes.
        Ok(())
    }
}
