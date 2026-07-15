use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260715_000004_social"
    }
}

#[derive(Iden)]
enum Carts {
    Table,
    Id,
}

#[derive(Iden)]
enum Users {
    Table,
    Id,
}

#[derive(Iden)]
enum Ratings {
    Table,
    Id,
    CartId,
    UserId,
    Score,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum Comments {
    Table,
    Id,
    CartId,
    UserId,
    Body,
    CreatedAt,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Ratings::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Ratings::Id).string().not_null().primary_key())
                    .col(ColumnDef::new(Ratings::CartId).string().not_null())
                    .col(ColumnDef::new(Ratings::UserId).string().not_null())
                    .col(ColumnDef::new(Ratings::Score).integer().not_null())
                    .col(ColumnDef::new(Ratings::CreatedAt).string().not_null())
                    .col(ColumnDef::new(Ratings::UpdatedAt).string().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_ratings_cart")
                            .from(Ratings::Table, Ratings::CartId)
                            .to(Carts::Table, Carts::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_ratings_user")
                            .from(Ratings::Table, Ratings::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_ratings_cart_user")
                    .table(Ratings::Table)
                    .col(Ratings::CartId)
                    .col(Ratings::UserId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Comments::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Comments::Id).string().not_null().primary_key())
                    .col(ColumnDef::new(Comments::CartId).string().not_null())
                    .col(ColumnDef::new(Comments::UserId).string().not_null())
                    .col(ColumnDef::new(Comments::Body).string().not_null())
                    .col(ColumnDef::new(Comments::CreatedAt).string().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_comments_cart")
                            .from(Comments::Table, Comments::CartId)
                            .to(Carts::Table, Carts::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_comments_user")
                            .from(Comments::Table, Comments::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_comments_cart")
                    .table(Comments::Table)
                    .col(Comments::CartId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Comments::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Ratings::Table).to_owned())
            .await
    }
}
