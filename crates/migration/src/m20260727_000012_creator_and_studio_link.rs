use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260727_000012_creator_and_studio_link"
    }
}

#[derive(Iden)]
enum CartVersions {
    Table,
    EditorUsername,
}
#[derive(Iden)]
enum StudioLinkRequests {
    Table,
    Id,
    PollSecretHash,
    ApprovedUserId,
    ExpiresAt,
    ConsumedAt,
    CancelledAt,
    CreatedAt,
}
#[derive(Iden)]
enum Users {
    Table,
    Id,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(CartVersions::Table)
                    .add_column(
                        ColumnDef::new(CartVersions::EditorUsername)
                            .string()
                            .not_null()
                            .default(""),
                    )
                    .to_owned(),
            )
            .await?;
        manager.get_connection().execute_unprepared(
            "UPDATE cart_versions SET editor_username = COALESCE((SELECT author FROM carts WHERE carts.id = cart_versions.cart_id), '')"
        ).await?;
        manager
            .create_table(
                Table::create()
                    .table(StudioLinkRequests::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(StudioLinkRequests::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(StudioLinkRequests::PollSecretHash)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(StudioLinkRequests::ApprovedUserId)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(StudioLinkRequests::ExpiresAt)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(StudioLinkRequests::ConsumedAt)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(StudioLinkRequests::CancelledAt)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(StudioLinkRequests::CreatedAt)
                            .string()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_studio_link_user")
                            .from(
                                StudioLinkRequests::Table,
                                StudioLinkRequests::ApprovedUserId,
                            )
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(StudioLinkRequests::Table).to_owned())
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(CartVersions::Table)
                    .drop_column(CartVersions::EditorUsername)
                    .to_owned(),
            )
            .await
    }
}
