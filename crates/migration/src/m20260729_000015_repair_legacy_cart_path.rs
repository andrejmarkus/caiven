use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260729_000015_repair_legacy_cart_path"
    }
}

#[derive(Iden)]
enum CartVersions {
    Table,
    LegacyCartPath,
}

// m20260723_000006_blob_storage adds this column, but some installs ended up
// missing it despite that migration being recorded as applied (schema drift
// from an out-of-band DB restore). Re-add it defensively so later migrations
// that reference it don't fail on `column "legacy_cart_path" does not exist`.
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if !manager
            .has_column("cart_versions", "legacy_cart_path")
            .await?
        {
            manager
                .alter_table(
                    Table::alter()
                        .table(CartVersions::Table)
                        .add_column(ColumnDef::new(CartVersions::LegacyCartPath).string().null())
                        .to_owned(),
                )
                .await?;
        }
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // No-op: this migration only repairs drift, it doesn't own the column.
        Ok(())
    }
}
