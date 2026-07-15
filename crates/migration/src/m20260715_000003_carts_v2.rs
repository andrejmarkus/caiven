use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260715_000003_carts_v2"
    }
}

#[derive(Iden)]
enum Carts {
    Table,
    Id,
    HasScreenshot,
    RomSize,
    OwnerId,
    RatingCount,
    RatingSum,
}

#[derive(Iden)]
enum CartVersions {
    Table,
    Id,
    CartId,
    Version,
    RomPath,
    RomSize,
    Changelog,
    HasScreenshot,
    CreatedAt,
}

const LEGACY_USER_ID: &str = "00000000-0000-0000-0000-000000000001";

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(CartVersions::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(CartVersions::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(CartVersions::CartId).string().not_null())
                    .col(ColumnDef::new(CartVersions::Version).integer().not_null())
                    .col(ColumnDef::new(CartVersions::RomPath).string().not_null())
                    .col(
                        ColumnDef::new(CartVersions::RomSize)
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(CartVersions::Changelog)
                            .string()
                            .not_null()
                            .default(""),
                    )
                    .col(
                        ColumnDef::new(CartVersions::HasScreenshot)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(ColumnDef::new(CartVersions::CreatedAt).string().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_cart_versions_cart")
                            .from(CartVersions::Table, CartVersions::CartId)
                            .to(Carts::Table, Carts::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_cart_versions_cart")
                    .table(CartVersions::Table)
                    .col(CartVersions::CartId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_cart_versions_cart_version")
                    .table(CartVersions::Table)
                    .col(CartVersions::CartId)
                    .col(CartVersions::Version)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Carts::Table)
                    .add_column(ColumnDef::new(Carts::OwnerId).string().null())
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Carts::Table)
                    .add_column(
                        ColumnDef::new(Carts::RatingCount)
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Carts::Table)
                    .add_column(
                        ColumnDef::new(Carts::RatingSum)
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .to_owned(),
            )
            .await?;

        let conn = manager.get_connection();

        // Bootstrap a `legacy` owner for any carts uploaded before accounts
        // existed, then give each of them a v1 cart_versions row built from
        // their existing rom_size/has_screenshot columns (read before those
        // columns are dropped below).
        conn.execute_unprepared(&format!(
            "INSERT INTO users (id, username, password_hash, is_admin, created_at) \
             SELECT '{LEGACY_USER_ID}', 'legacy', '!', 0, strftime('%Y-%m-%dT%H:%M:%fZ', 'now') \
             WHERE EXISTS (SELECT 1 FROM carts) \
               AND NOT EXISTS (SELECT 1 FROM users WHERE id = '{LEGACY_USER_ID}')"
        ))
        .await?;

        conn.execute_unprepared(&format!(
            "UPDATE carts SET owner_id = '{LEGACY_USER_ID}' WHERE owner_id IS NULL"
        ))
        .await?;

        conn.execute_unprepared(
            "INSERT INTO cart_versions \
                (id, cart_id, version, rom_path, rom_size, changelog, has_screenshot, created_at) \
             SELECT lower(hex(randomblob(16))), id, 1, 'roms/' || id || '.rom', rom_size, '', \
                has_screenshot, uploaded_at \
             FROM carts \
             WHERE NOT EXISTS (SELECT 1 FROM cart_versions WHERE cart_versions.cart_id = carts.id)",
        )
        .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Carts::Table)
                    .drop_column(Carts::HasScreenshot)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Carts::Table)
                    .drop_column(Carts::RomSize)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Carts::Table)
                    .add_column(
                        ColumnDef::new(Carts::RomSize)
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Carts::Table)
                    .add_column(
                        ColumnDef::new(Carts::HasScreenshot)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .to_owned(),
            )
            .await?;

        let conn = manager.get_connection();
        conn.execute_unprepared(
            "UPDATE carts SET rom_size = COALESCE((SELECT rom_size FROM cart_versions \
                WHERE cart_versions.cart_id = carts.id ORDER BY version DESC LIMIT 1), 0), \
                has_screenshot = COALESCE((SELECT has_screenshot FROM cart_versions \
                WHERE cart_versions.cart_id = carts.id ORDER BY version DESC LIMIT 1), 0)",
        )
        .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Carts::Table)
                    .drop_column(Carts::RatingSum)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Carts::Table)
                    .drop_column(Carts::RatingCount)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Carts::Table)
                    .drop_column(Carts::OwnerId)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(Table::drop().table(CartVersions::Table).to_owned())
            .await
    }
}
