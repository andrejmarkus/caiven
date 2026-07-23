use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260723_000006_blob_storage"
    }
}

#[derive(Iden)]
enum CartVersions {
    Table,
    Id,
    CartPath,
}

#[derive(Iden)]
enum CartBlobs {
    Table,
    VersionId,
    CartData,
    ScreenshotData,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(CartBlobs::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(CartBlobs::VersionId)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(CartBlobs::CartData).binary().not_null())
                    .col(ColumnDef::new(CartBlobs::ScreenshotData).binary().null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_cart_blobs_version")
                            .from(CartBlobs::Table, CartBlobs::VersionId)
                            .to(CartVersions::Table, CartVersions::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(CartVersions::Table)
                    .drop_column(CartVersions::CartPath)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(CartVersions::Table)
                    .add_column(
                        ColumnDef::new(CartVersions::CartPath)
                            .string()
                            .not_null()
                            .default(""),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(Table::drop().table(CartBlobs::Table).to_owned())
            .await
    }
}
