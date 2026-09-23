use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260923_000018_remix_lineage"
    }
}

#[derive(Iden)]
enum Carts {
    Table,
    Remixable,
    ParentCartId,
    ParentVersion,
    RootCartId,
}

#[derive(Iden)]
enum FunnelEvents {
    Table,
    Id,
    CartId,
    Event,
    ViewerKey,
    CreatedAt,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Opt-in: existing carts never become remixable without their owner
        // saying so, since creators were promised source stays theirs.
        manager
            .alter_table(
                Table::alter()
                    .table(Carts::Table)
                    .add_column(
                        ColumnDef::new(Carts::Remixable)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .to_owned(),
            )
            .await?;
        // Lineage is written once at create time and never updated, so the
        // root is stored rather than walked recursively on every read.
        for col in [Carts::ParentCartId, Carts::RootCartId] {
            manager
                .alter_table(
                    Table::alter()
                        .table(Carts::Table)
                        .add_column(ColumnDef::new(col).string().null())
                        .to_owned(),
                )
                .await?;
        }
        manager
            .alter_table(
                Table::alter()
                    .table(Carts::Table)
                    .add_column(ColumnDef::new(Carts::ParentVersion).integer().null())
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_carts_parent_cart_id")
                    .table(Carts::Table)
                    .col(Carts::ParentCartId)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_carts_root_cart_id")
                    .table(Carts::Table)
                    .col(Carts::RootCartId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(FunnelEvents::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(FunnelEvents::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(FunnelEvents::CartId).string().not_null())
                    .col(ColumnDef::new(FunnelEvents::Event).string().not_null())
                    .col(ColumnDef::new(FunnelEvents::ViewerKey).string().not_null())
                    .col(ColumnDef::new(FunnelEvents::CreatedAt).string().not_null())
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_funnel_events_unique_viewer")
                    .table(FunnelEvents::Table)
                    .col(FunnelEvents::CartId)
                    .col(FunnelEvents::Event)
                    .col(FunnelEvents::ViewerKey)
                    .unique()
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_funnel_events_created_at")
                    .table(FunnelEvents::Table)
                    .col(FunnelEvents::CreatedAt)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(FunnelEvents::Table).to_owned())
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_carts_root_cart_id")
                    .table(Carts::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_carts_parent_cart_id")
                    .table(Carts::Table)
                    .to_owned(),
            )
            .await?;
        for col in [
            Carts::ParentVersion,
            Carts::RootCartId,
            Carts::ParentCartId,
            Carts::Remixable,
        ] {
            manager
                .alter_table(
                    Table::alter()
                        .table(Carts::Table)
                        .drop_column(col)
                        .to_owned(),
                )
                .await?;
        }
        Ok(())
    }
}
