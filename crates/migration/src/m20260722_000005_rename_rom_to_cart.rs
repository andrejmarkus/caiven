use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260722_000005_rename_rom_to_cart"
    }
}

#[derive(Iden)]
enum CartVersions {
    Table,
    RomPath,
    RomSize,
    CartPath,
    CartSize,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(CartVersions::Table)
                    .rename_column(CartVersions::RomPath, CartVersions::CartPath)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(CartVersions::Table)
                    .rename_column(CartVersions::RomSize, CartVersions::CartSize)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(CartVersions::Table)
                    .rename_column(CartVersions::CartPath, CartVersions::RomPath)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(CartVersions::Table)
                    .rename_column(CartVersions::CartSize, CartVersions::RomSize)
                    .to_owned(),
            )
            .await
    }
}
