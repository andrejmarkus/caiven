pub use sea_orm_migration::prelude::*;

mod m20240101_000001_create_carts;
mod m20260715_000002_create_auth;
mod m20260715_000003_carts_v2;
mod m20260715_000004_social;
mod m20260722_000005_rename_rom_to_cart;
mod m20260723_000006_blob_storage;
mod m20260725_000007_port_redesign;
mod m20260725_000008_harden_auth;
mod m20260726_000009_auth_modern;
mod m20260726_000010_security_hardening;
mod m20260726_000011_security_round3;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20240101_000001_create_carts::Migration),
            Box::new(m20260715_000002_create_auth::Migration),
            Box::new(m20260715_000003_carts_v2::Migration),
            Box::new(m20260715_000004_social::Migration),
            Box::new(m20260722_000005_rename_rom_to_cart::Migration),
            Box::new(m20260723_000006_blob_storage::Migration),
            Box::new(m20260725_000007_port_redesign::Migration),
            Box::new(m20260725_000008_harden_auth::Migration),
            Box::new(m20260726_000009_auth_modern::Migration),
            Box::new(m20260726_000010_security_hardening::Migration),
            Box::new(m20260726_000011_security_round3::Migration),
        ]
    }
}
