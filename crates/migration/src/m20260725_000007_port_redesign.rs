use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260725_000007_port_redesign"
    }
}

#[derive(Iden)]
enum Carts {
    Table,
    Id,
    Plays,
}

#[derive(Iden)]
enum Users {
    Table,
    Id,
}

#[derive(Iden)]
enum PlayEvents {
    Table,
    Id,
    CartId,
    SessionKey,
    ViewerKey,
    PlayedAt,
}

#[derive(Iden)]
enum Follows {
    Table,
    FollowerId,
    FollowedId,
    CreatedAt,
}

#[derive(Iden)]
enum Collections {
    Table,
    Id,
    OwnerId,
    Slug,
    Title,
    Description,
    Kind,
    FeaturedRank,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum CollectionCarts {
    Table,
    CollectionId,
    CartId,
    Position,
    AddedAt,
}

#[derive(Iden)]
enum CollectionFollows {
    Table,
    CollectionId,
    UserId,
    CreatedAt,
}

#[derive(Iden)]
enum Jams {
    Table,
    Id,
    Slug,
    Title,
    Description,
    Rules,
    StartsAt,
    SubmissionsCloseAt,
    EndsAt,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum JamEntries {
    Table,
    Id,
    JamId,
    CartId,
    UserId,
    SubmittedAt,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Carts::Table)
                    .add_column(
                        ColumnDef::new(Carts::Plays)
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(PlayEvents::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(PlayEvents::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(PlayEvents::CartId).string().not_null())
                    .col(ColumnDef::new(PlayEvents::SessionKey).string().not_null())
                    .col(ColumnDef::new(PlayEvents::ViewerKey).string().not_null())
                    .col(ColumnDef::new(PlayEvents::PlayedAt).string().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_play_events_cart")
                            .from(PlayEvents::Table, PlayEvents::CartId)
                            .to(Carts::Table, Carts::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_play_events_cart_session")
                    .table(PlayEvents::Table)
                    .col(PlayEvents::CartId)
                    .col(PlayEvents::SessionKey)
                    .unique()
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_play_events_played_at")
                    .table(PlayEvents::Table)
                    .col(PlayEvents::PlayedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Follows::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Follows::FollowerId).string().not_null())
                    .col(ColumnDef::new(Follows::FollowedId).string().not_null())
                    .col(ColumnDef::new(Follows::CreatedAt).string().not_null())
                    .primary_key(
                        Index::create()
                            .col(Follows::FollowerId)
                            .col(Follows::FollowedId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_follows_follower")
                            .from(Follows::Table, Follows::FollowerId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_follows_followed")
                            .from(Follows::Table, Follows::FollowedId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Collections::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Collections::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Collections::OwnerId).string().not_null())
                    .col(
                        ColumnDef::new(Collections::Slug)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(Collections::Title).string().not_null())
                    .col(
                        ColumnDef::new(Collections::Description)
                            .string()
                            .not_null()
                            .default(""),
                    )
                    .col(
                        ColumnDef::new(Collections::Kind)
                            .string()
                            .not_null()
                            .default("player"),
                    )
                    .col(ColumnDef::new(Collections::FeaturedRank).integer().null())
                    .col(ColumnDef::new(Collections::CreatedAt).string().not_null())
                    .col(ColumnDef::new(Collections::UpdatedAt).string().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_collections_owner")
                            .from(Collections::Table, Collections::OwnerId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(CollectionCarts::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(CollectionCarts::CollectionId)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(CollectionCarts::CartId).string().not_null())
                    .col(
                        ColumnDef::new(CollectionCarts::Position)
                            .integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(CollectionCarts::AddedAt).string().not_null())
                    .primary_key(
                        Index::create()
                            .col(CollectionCarts::CollectionId)
                            .col(CollectionCarts::CartId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_collection_carts_collection")
                            .from(CollectionCarts::Table, CollectionCarts::CollectionId)
                            .to(Collections::Table, Collections::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_collection_carts_cart")
                            .from(CollectionCarts::Table, CollectionCarts::CartId)
                            .to(Carts::Table, Carts::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_collection_carts_position")
                    .table(CollectionCarts::Table)
                    .col(CollectionCarts::CollectionId)
                    .col(CollectionCarts::Position)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(CollectionFollows::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(CollectionFollows::CollectionId)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CollectionFollows::UserId)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CollectionFollows::CreatedAt)
                            .string()
                            .not_null(),
                    )
                    .primary_key(
                        Index::create()
                            .col(CollectionFollows::CollectionId)
                            .col(CollectionFollows::UserId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_collection_follows_collection")
                            .from(CollectionFollows::Table, CollectionFollows::CollectionId)
                            .to(Collections::Table, Collections::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_collection_follows_user")
                            .from(CollectionFollows::Table, CollectionFollows::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Jams::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Jams::Id).string().not_null().primary_key())
                    .col(ColumnDef::new(Jams::Slug).string().not_null().unique_key())
                    .col(ColumnDef::new(Jams::Title).string().not_null())
                    .col(
                        ColumnDef::new(Jams::Description)
                            .string()
                            .not_null()
                            .default(""),
                    )
                    .col(ColumnDef::new(Jams::Rules).string().not_null().default(""))
                    .col(ColumnDef::new(Jams::StartsAt).string().not_null())
                    .col(ColumnDef::new(Jams::SubmissionsCloseAt).string().not_null())
                    .col(ColumnDef::new(Jams::EndsAt).string().not_null())
                    .col(ColumnDef::new(Jams::CreatedAt).string().not_null())
                    .col(ColumnDef::new(Jams::UpdatedAt).string().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(JamEntries::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(JamEntries::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(JamEntries::JamId).string().not_null())
                    .col(ColumnDef::new(JamEntries::CartId).string().not_null())
                    .col(ColumnDef::new(JamEntries::UserId).string().not_null())
                    .col(ColumnDef::new(JamEntries::SubmittedAt).string().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_jam_entries_jam")
                            .from(JamEntries::Table, JamEntries::JamId)
                            .to(Jams::Table, Jams::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_jam_entries_cart")
                            .from(JamEntries::Table, JamEntries::CartId)
                            .to(Carts::Table, Carts::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_jam_entries_user")
                            .from(JamEntries::Table, JamEntries::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_jam_entries_jam_cart")
                    .table(JamEntries::Table)
                    .col(JamEntries::JamId)
                    .col(JamEntries::CartId)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(JamEntries::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Jams::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(CollectionFollows::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(CollectionCarts::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Collections::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Follows::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(PlayEvents::Table).to_owned())
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Carts::Table)
                    .drop_column(Carts::Plays)
                    .to_owned(),
            )
            .await
    }
}
