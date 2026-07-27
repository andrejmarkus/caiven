use sea_orm_migration::prelude::*;

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
