pub use sea_orm_migration::prelude::*;

mod m20240101_000001_create_carts;
mod m20260715_000002_create_auth;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20240101_000001_create_carts::Migration),
            Box::new(m20260715_000002_create_auth::Migration),
        ]
    }
}
