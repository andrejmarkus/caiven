use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20240101_000001_create_carts"
    }
}

#[derive(Iden)]
enum Carts {
    Table,
    Id,
    Title,
    Author,
    Description,
    Tags,
    UploadedAt,
    Downloads,
    HasScreenshot,
    RomSize,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Carts::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Carts::Id).string().not_null().primary_key())
                    .col(ColumnDef::new(Carts::Title).string().not_null())
                    .col(ColumnDef::new(Carts::Author).string().not_null())
                    .col(
                        ColumnDef::new(Carts::Description)
                            .string()
                            .not_null()
                            .default(""),
                    )
                    .col(ColumnDef::new(Carts::Tags).string().not_null().default(""))
                    .col(ColumnDef::new(Carts::UploadedAt).string().not_null())
                    .col(
                        ColumnDef::new(Carts::Downloads)
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(Carts::HasScreenshot)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Carts::RomSize)
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_carts_uploaded_at")
                    .table(Carts::Table)
                    .col(Carts::UploadedAt)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Carts::Table).to_owned())
            .await
    }
}
