use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260726_000010_security_hardening"
    }
}

#[derive(Iden)]
enum Users {
    Table,
    Id,
    MfaTotpSecret,
    MfaEnabled,
    PasswordSet,
}

#[derive(Iden)]
enum Sessions {
    Table,
    UserAgent,
    Ip,
    LastSeenAt,
    CreatedAt,
}

#[derive(Iden)]
enum MfaBackupCodes {
    Table,
    Id,
    UserId,
    CodeHash,
    UsedAt,
    CreatedAt,
}

#[derive(Iden)]
enum MfaChallenges {
    Table,
    Id,
    UserId,
    ExpiresAt,
    CreatedAt,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // SQLite only supports one ALTER TABLE ADD COLUMN per statement.
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .add_column(ColumnDef::new(Users::MfaTotpSecret).string().null())
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .add_column(
                        ColumnDef::new(Users::MfaEnabled)
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
                    .add_column(
                        ColumnDef::new(Users::PasswordSet)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Sessions::Table)
                    .add_column(ColumnDef::new(Sessions::UserAgent).string().null())
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Sessions::Table)
                    .add_column(ColumnDef::new(Sessions::Ip).string().null())
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Sessions::Table)
                    .add_column(
                        ColumnDef::new(Sessions::LastSeenAt)
                            .string()
                            .not_null()
                            .default(""),
                    )
                    .to_owned(),
            )
            .await?;
        // Backfill last_seen_at for pre-existing rows from created_at, since
        // the column default above can't reference another column.
        manager
            .get_connection()
            .execute(sea_orm::Statement::from_string(
                manager.get_database_backend(),
                format!(
                    "UPDATE {} SET {} = {} WHERE {} = ''",
                    Sessions::Table.to_string(),
                    Sessions::LastSeenAt.to_string(),
                    Sessions::CreatedAt.to_string(),
                    Sessions::LastSeenAt.to_string(),
                ),
            ))
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(MfaBackupCodes::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(MfaBackupCodes::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(MfaBackupCodes::UserId).string().not_null())
                    .col(ColumnDef::new(MfaBackupCodes::CodeHash).string().not_null())
                    .col(ColumnDef::new(MfaBackupCodes::UsedAt).string().null())
                    .col(
                        ColumnDef::new(MfaBackupCodes::CreatedAt)
                            .string()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_mfa_backup_codes_user")
                            .from(MfaBackupCodes::Table, MfaBackupCodes::UserId)
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
                    .name("idx_mfa_backup_codes_user")
                    .table(MfaBackupCodes::Table)
                    .col(MfaBackupCodes::UserId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(MfaChallenges::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(MfaChallenges::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(MfaChallenges::UserId).string().not_null())
                    .col(ColumnDef::new(MfaChallenges::ExpiresAt).string().not_null())
                    .col(ColumnDef::new(MfaChallenges::CreatedAt).string().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_mfa_challenges_user")
                            .from(MfaChallenges::Table, MfaChallenges::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(MfaChallenges::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(MfaBackupCodes::Table).to_owned())
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Sessions::Table)
                    .drop_column(Sessions::LastSeenAt)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Sessions::Table)
                    .drop_column(Sessions::Ip)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Sessions::Table)
                    .drop_column(Sessions::UserAgent)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .drop_column(Users::PasswordSet)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .drop_column(Users::MfaEnabled)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .drop_column(Users::MfaTotpSecret)
                    .to_owned(),
            )
            .await
    }
}
