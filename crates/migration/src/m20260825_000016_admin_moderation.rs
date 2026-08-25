use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260825_000016_admin_moderation"
    }
}

#[derive(Iden)]
enum Users {
    Table,
    IsBanned,
    BannedAt,
    BannedReason,
    BannedBy,
}

#[derive(Iden)]
enum ModerationActions {
    Table,
    Id,
    ActorId,
    Action,
    TargetType,
    TargetId,
    Reason,
    CreatedAt,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .add_column(
                        ColumnDef::new(Users::IsBanned)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .add_column(ColumnDef::new(Users::BannedAt).string().null())
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .add_column(ColumnDef::new(Users::BannedReason).string().null())
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .add_column(ColumnDef::new(Users::BannedBy).string().null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(ModerationActions::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ModerationActions::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(ModerationActions::ActorId)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ModerationActions::Action)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ModerationActions::TargetType)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ModerationActions::TargetId)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(ModerationActions::Reason).string().null())
                    .col(
                        ColumnDef::new(ModerationActions::CreatedAt)
                            .string()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ModerationActions::Table).to_owned())
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .drop_column(Users::BannedBy)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .drop_column(Users::BannedReason)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .drop_column(Users::BannedAt)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .drop_column(Users::IsBanned)
                    .to_owned(),
            )
            .await
    }
}
