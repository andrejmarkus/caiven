use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260726_000009_auth_modern"
    }
}

#[derive(Iden)]
enum Users {
    Table,
    Id,
    Email,
    EmailVerified,
    EmailNormalized,
}

#[derive(Iden)]
enum EmailTokens {
    Table,
    Id,
    UserId,
    Kind,
    TokenHash,
    CreatedAt,
    ExpiresAt,
    UsedAt,
}

#[derive(Iden)]
enum OauthIdentities {
    Table,
    Id,
    UserId,
    Provider,
    Subject,
    Email,
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
                    .add_column(ColumnDef::new(Users::Email).string().null())
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .add_column(
                        ColumnDef::new(Users::EmailVerified)
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
                    .add_column(ColumnDef::new(Users::EmailNormalized).string().null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_users_email_normalized")
                    .table(Users::Table)
                    .col(Users::EmailNormalized)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(EmailTokens::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(EmailTokens::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(EmailTokens::UserId).string().not_null())
                    .col(ColumnDef::new(EmailTokens::Kind).string().not_null())
                    .col(ColumnDef::new(EmailTokens::TokenHash).string().not_null())
                    .col(ColumnDef::new(EmailTokens::CreatedAt).string().not_null())
                    .col(ColumnDef::new(EmailTokens::ExpiresAt).string().not_null())
                    .col(ColumnDef::new(EmailTokens::UsedAt).string().null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_email_tokens_user")
                            .from(EmailTokens::Table, EmailTokens::UserId)
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
                    .name("idx_email_tokens_hash")
                    .table(EmailTokens::Table)
                    .col(EmailTokens::TokenHash)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(OauthIdentities::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(OauthIdentities::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(OauthIdentities::UserId)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(OauthIdentities::Provider)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(OauthIdentities::Subject)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(OauthIdentities::Email).string().null())
                    .col(
                        ColumnDef::new(OauthIdentities::CreatedAt)
                            .string()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_oauth_identities_user")
                            .from(OauthIdentities::Table, OauthIdentities::UserId)
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
                    .name("idx_oauth_identities_provider_subject")
                    .table(OauthIdentities::Table)
                    .col(OauthIdentities::Provider)
                    .col(OauthIdentities::Subject)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(OauthIdentities::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(EmailTokens::Table).to_owned())
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .drop_column(Users::Email)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .drop_column(Users::EmailVerified)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .drop_column(Users::EmailNormalized)
                    .to_owned(),
            )
            .await
    }
}
