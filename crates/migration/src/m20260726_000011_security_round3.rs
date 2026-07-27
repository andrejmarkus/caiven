use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260726_000011_security_round3"
    }
}

#[derive(Iden)]
enum Users {
    Table,
    Id,
}

#[derive(Iden)]
enum WebauthnCredentials {
    Table,
    Id,
    UserId,
    Label,
    PasskeyJson,
    CreatedAt,
    LastUsedAt,
}

#[derive(Iden)]
enum WebauthnChallenges {
    Table,
    Id,
    UserId,
    Kind,
    StateJson,
    ExpiresAt,
    CreatedAt,
}

#[derive(Iden)]
enum AuditLog {
    Table,
    Id,
    UserId,
    Event,
    Ip,
    UserAgent,
    Metadata,
    CreatedAt,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(WebauthnCredentials::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(WebauthnCredentials::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(WebauthnCredentials::UserId)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WebauthnCredentials::Label)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WebauthnCredentials::PasskeyJson)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WebauthnCredentials::CreatedAt)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(WebauthnCredentials::LastUsedAt).string().null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_webauthn_credentials_user")
                            .from(WebauthnCredentials::Table, WebauthnCredentials::UserId)
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
                    .name("idx_webauthn_credentials_user")
                    .table(WebauthnCredentials::Table)
                    .col(WebauthnCredentials::UserId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(WebauthnChallenges::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(WebauthnChallenges::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(WebauthnChallenges::UserId).string().null())
                    .col(
                        ColumnDef::new(WebauthnChallenges::Kind)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WebauthnChallenges::StateJson)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WebauthnChallenges::ExpiresAt)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WebauthnChallenges::CreatedAt)
                            .string()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(AuditLog::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(AuditLog::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(AuditLog::UserId).string().not_null())
                    .col(ColumnDef::new(AuditLog::Event).string().not_null())
                    .col(ColumnDef::new(AuditLog::Ip).string().null())
                    .col(ColumnDef::new(AuditLog::UserAgent).string().null())
                    .col(ColumnDef::new(AuditLog::Metadata).text().null())
                    .col(ColumnDef::new(AuditLog::CreatedAt).string().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_audit_log_user")
                            .from(AuditLog::Table, AuditLog::UserId)
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
                    .name("idx_audit_log_user_created")
                    .table(AuditLog::Table)
                    .col(AuditLog::UserId)
                    .col(AuditLog::CreatedAt)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(AuditLog::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(WebauthnChallenges::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(WebauthnCredentials::Table).to_owned())
            .await
    }
}
