use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260725_000008_harden_auth"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Previous rows used the plaintext bearer token as their primary key.
        // New rows store SHA-256(token), so invalidate old browser sessions once.
        manager
            .get_connection()
            .execute_unprepared("DELETE FROM sessions")
            .await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // Revoked bearer sessions cannot and should not be restored.
        Ok(())
    }
}
