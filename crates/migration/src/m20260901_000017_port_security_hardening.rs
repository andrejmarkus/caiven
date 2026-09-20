use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260901_000017_port_security_hardening"
    }
}

#[derive(Iden)]
enum ApiTokens {
    Table,
    Scope,
}

#[derive(Iden)]
enum StudioLinkRequests {
    Table,
    UserCodeHash,
    CodeAttempts,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Tokens minted for a specific purpose (e.g. the Studio link flow)
        // carry a scope so a leaked/phished token can't reach the account
        // management surface — existing tokens default to "full" to match
        // their current unrestricted behavior.
        manager
            .alter_table(
                Table::alter()
                    .table(ApiTokens::Table)
                    .add_column(
                        ColumnDef::new(ApiTokens::Scope)
                            .string()
                            .not_null()
                            .default("full"),
                    )
                    .to_owned(),
            )
            .await?;

        // Anti-phishing confirmation code for the Studio link flow: Studio
        // displays it locally, the approving browser must type it back.
        manager
            .alter_table(
                Table::alter()
                    .table(StudioLinkRequests::Table)
                    .add_column(
                        ColumnDef::new(StudioLinkRequests::UserCodeHash)
                            .string()
                            .not_null()
                            .default(""),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(StudioLinkRequests::Table)
                    .add_column(
                        ColumnDef::new(StudioLinkRequests::CodeAttempts)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(StudioLinkRequests::Table)
                    .drop_column(StudioLinkRequests::CodeAttempts)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(StudioLinkRequests::Table)
                    .drop_column(StudioLinkRequests::UserCodeHash)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(ApiTokens::Table)
                    .drop_column(ApiTokens::Scope)
                    .to_owned(),
            )
            .await
    }
}
