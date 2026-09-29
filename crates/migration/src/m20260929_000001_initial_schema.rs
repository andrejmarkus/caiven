use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260929_000001_initial_schema"
    }
}

fn col(name: &str) -> ColumnDef {
    ColumnDef::new(Alias::new(name))
}

fn id(name: &str) -> ColumnDef {
    col(name).string().not_null().primary_key().to_owned()
}

fn text(name: &str) -> ColumnDef {
    col(name).string().not_null().to_owned()
}

fn text_or(name: &str, default: &str) -> ColumnDef {
    col(name).string().not_null().default(default).to_owned()
}

fn opt_text(name: &str) -> ColumnDef {
    col(name).string().null().to_owned()
}

fn flag(name: &str, default: bool) -> ColumnDef {
    col(name).boolean().not_null().default(default).to_owned()
}

fn counter(name: &str) -> ColumnDef {
    col(name).big_integer().not_null().default(0).to_owned()
}

/// `from.column -> to.id`, deleting the row with its parent.
fn cascade(from: &str, column: &str, to: &str) -> ForeignKeyCreateStatement {
    ForeignKey::create()
        .name(format!("fk_{from}_{column}"))
        .from(Alias::new(from), Alias::new(column))
        .to(Alias::new(to), Alias::new("id"))
        .on_delete(ForeignKeyAction::Cascade)
        .to_owned()
}

fn index(name: &str, table: &str, columns: &[&str]) -> IndexCreateStatement {
    let mut index = Index::create();
    index.name(name).table(Alias::new(table));
    for column in columns {
        index.col(Alias::new(*column));
    }
    index
}

fn table(name: &str) -> TableCreateStatement {
    Table::create().table(Alias::new(name)).to_owned()
}

/// Every table, parents before children.
fn tables() -> Vec<TableCreateStatement> {
    vec![
        table("users")
            .col(id("id"))
            .col(text("username"))
            .col(text("password_hash"))
            .col(flag("is_admin", false))
            .col(text("created_at"))
            .col(opt_text("email"))
            .col(flag("email_verified", false))
            .col(opt_text("email_normalized"))
            .col(opt_text("mfa_totp_secret"))
            .col(flag("mfa_enabled", false))
            .col(flag("password_set", true))
            .col(flag("is_banned", false))
            .col(opt_text("banned_at"))
            .col(opt_text("banned_reason"))
            .col(opt_text("banned_by"))
            .to_owned(),
        table("sessions")
            .col(id("id"))
            .col(text("user_id"))
            .col(text("created_at"))
            .col(text("expires_at"))
            .col(opt_text("user_agent"))
            .col(opt_text("ip"))
            .col(text_or("last_seen_at", ""))
            .foreign_key(&mut cascade("sessions", "user_id", "users"))
            .to_owned(),
        table("api_tokens")
            .col(id("id"))
            .col(text("user_id"))
            .col(text("token_hash"))
            .col(text_or("name", ""))
            .col(text("created_at"))
            .col(opt_text("last_used_at"))
            // "full", or "publish" for tokens minted by the Studio link flow.
            .col(text_or("scope", "full"))
            .foreign_key(&mut cascade("api_tokens", "user_id", "users"))
            .to_owned(),
        table("email_tokens")
            .col(id("id"))
            .col(text("user_id"))
            .col(text("kind"))
            .col(text("token_hash"))
            .col(text("created_at"))
            .col(text("expires_at"))
            .col(opt_text("used_at"))
            .foreign_key(&mut cascade("email_tokens", "user_id", "users"))
            .to_owned(),
        table("oauth_identities")
            .col(id("id"))
            .col(text("user_id"))
            .col(text("provider"))
            .col(text("subject"))
            .col(opt_text("email"))
            .col(text("created_at"))
            .foreign_key(&mut cascade("oauth_identities", "user_id", "users"))
            .to_owned(),
        table("mfa_backup_codes")
            .col(id("id"))
            .col(text("user_id"))
            .col(text("code_hash"))
            .col(opt_text("used_at"))
            .col(text("created_at"))
            .foreign_key(&mut cascade("mfa_backup_codes", "user_id", "users"))
            .to_owned(),
        table("mfa_challenges")
            .col(id("id"))
            .col(text("user_id"))
            .col(text("expires_at"))
            .col(text("created_at"))
            .foreign_key(&mut cascade("mfa_challenges", "user_id", "users"))
            .to_owned(),
        table("webauthn_credentials")
            .col(id("id"))
            .col(text("user_id"))
            .col(text("label"))
            .col(col("passkey_json").text().not_null())
            .col(text("created_at"))
            .col(opt_text("last_used_at"))
            .foreign_key(&mut cascade("webauthn_credentials", "user_id", "users"))
            .to_owned(),
        table("webauthn_challenges")
            .col(id("id"))
            .col(opt_text("user_id"))
            .col(text("kind"))
            .col(col("state_json").text().not_null())
            .col(text("expires_at"))
            .col(text("created_at"))
            .to_owned(),
        table("audit_log")
            .col(id("id"))
            .col(text("user_id"))
            .col(text("event"))
            .col(opt_text("ip"))
            .col(opt_text("user_agent"))
            .col(col("metadata").text().null())
            .col(text("created_at"))
            .foreign_key(&mut cascade("audit_log", "user_id", "users"))
            .to_owned(),
        table("studio_link_requests")
            .col(id("id"))
            .col(text("poll_secret_hash").unique_key())
            .col(opt_text("approved_user_id"))
            .col(text("expires_at"))
            .col(opt_text("consumed_at"))
            .col(opt_text("cancelled_at"))
            .col(text("created_at"))
            .col(text_or("user_code_hash", ""))
            .col(col("code_attempts").integer().not_null().default(0))
            .foreign_key(&mut cascade(
                "studio_link_requests",
                "approved_user_id",
                "users",
            ))
            .to_owned(),
        table("moderation_actions")
            .col(id("id"))
            .col(text("actor_id"))
            .col(text("action"))
            .col(text("target_type"))
            .col(text("target_id"))
            .col(opt_text("reason"))
            .col(text("created_at"))
            .to_owned(),
        table("carts")
            .col(id("id"))
            .col(text("title"))
            .col(text("author"))
            .col(text_or("description", ""))
            .col(text_or("tags", ""))
            .col(text("uploaded_at"))
            .col(counter("downloads"))
            // NULL once the owning account is deleted; the cart stays public.
            .col(opt_text("owner_id"))
            .col(counter("rating_count"))
            .col(counter("rating_sum"))
            .col(counter("plays"))
            .col(flag("remixable", false))
            // Lineage is written once at create time and never updated, so
            // the root is stored rather than walked on every read.
            .col(opt_text("parent_cart_id"))
            .col(col("parent_version").integer().null())
            .col(opt_text("root_cart_id"))
            .foreign_key(
                ForeignKey::create()
                    .name("fk_carts_owner_id")
                    .from(Alias::new("carts"), Alias::new("owner_id"))
                    .to(Alias::new("users"), Alias::new("id"))
                    .on_delete(ForeignKeyAction::SetNull),
            )
            .to_owned(),
        table("cart_versions")
            .col(id("id"))
            .col(text("cart_id"))
            .col(col("version").integer().not_null())
            .col(counter("cart_size"))
            .col(text_or("changelog", ""))
            .col(flag("has_screenshot", false))
            .col(text("created_at"))
            .col(text_or("editor_username", ""))
            .col(text("content_hash"))
            .foreign_key(&mut cascade("cart_versions", "cart_id", "carts"))
            .to_owned(),
        table("cart_blobs")
            .col(id("version_id"))
            .col(col("cart_data").binary().not_null())
            .col(col("screenshot_data").binary().null())
            .foreign_key(
                ForeignKey::create()
                    .name("fk_cart_blobs_version_id")
                    .from(Alias::new("cart_blobs"), Alias::new("version_id"))
                    .to(Alias::new("cart_versions"), Alias::new("id"))
                    .on_delete(ForeignKeyAction::Cascade),
            )
            .to_owned(),
        table("ratings")
            .col(id("id"))
            .col(text("cart_id"))
            .col(text("user_id"))
            .col(col("score").integer().not_null())
            .col(text("created_at"))
            .col(text("updated_at"))
            .foreign_key(&mut cascade("ratings", "cart_id", "carts"))
            .foreign_key(&mut cascade("ratings", "user_id", "users"))
            .to_owned(),
        table("comments")
            .col(id("id"))
            .col(text("cart_id"))
            .col(text("user_id"))
            .col(text("body"))
            .col(text("created_at"))
            .foreign_key(&mut cascade("comments", "cart_id", "carts"))
            .foreign_key(&mut cascade("comments", "user_id", "users"))
            .to_owned(),
        table("play_events")
            .col(id("id"))
            .col(text("cart_id"))
            .col(text("session_key"))
            .col(text("viewer_key"))
            .col(text("played_at"))
            .foreign_key(&mut cascade("play_events", "cart_id", "carts"))
            .to_owned(),
        table("funnel_events")
            .col(id("id"))
            .col(text("cart_id"))
            .col(text("event"))
            .col(text("viewer_key"))
            .col(text("created_at"))
            .to_owned(),
        table("follows")
            .col(text("follower_id"))
            .col(text("followed_id"))
            .col(text("created_at"))
            .primary_key(
                Index::create()
                    .col(Alias::new("follower_id"))
                    .col(Alias::new("followed_id")),
            )
            .foreign_key(&mut cascade("follows", "follower_id", "users"))
            .foreign_key(&mut cascade("follows", "followed_id", "users"))
            .to_owned(),
        table("collections")
            .col(id("id"))
            .col(text("owner_id"))
            .col(text("slug").unique_key())
            .col(text("title"))
            .col(text_or("description", ""))
            .col(text_or("kind", "player"))
            .col(col("featured_rank").integer().null())
            .col(text("created_at"))
            .col(text("updated_at"))
            .foreign_key(&mut cascade("collections", "owner_id", "users"))
            .to_owned(),
        table("collection_carts")
            .col(text("collection_id"))
            .col(text("cart_id"))
            .col(col("position").integer().not_null())
            .col(text("added_at"))
            .primary_key(
                Index::create()
                    .col(Alias::new("collection_id"))
                    .col(Alias::new("cart_id")),
            )
            .foreign_key(&mut cascade(
                "collection_carts",
                "collection_id",
                "collections",
            ))
            .foreign_key(&mut cascade("collection_carts", "cart_id", "carts"))
            .to_owned(),
        table("collection_follows")
            .col(text("collection_id"))
            .col(text("user_id"))
            .col(text("created_at"))
            .primary_key(
                Index::create()
                    .col(Alias::new("collection_id"))
                    .col(Alias::new("user_id")),
            )
            .foreign_key(&mut cascade(
                "collection_follows",
                "collection_id",
                "collections",
            ))
            .foreign_key(&mut cascade("collection_follows", "user_id", "users"))
            .to_owned(),
        table("jams")
            .col(id("id"))
            .col(text("slug").unique_key())
            .col(text("title"))
            .col(text_or("description", ""))
            .col(text_or("rules", ""))
            .col(text("starts_at"))
            .col(text("submissions_close_at"))
            .col(text("ends_at"))
            .col(text("created_at"))
            .col(text("updated_at"))
            .to_owned(),
        table("jam_entries")
            .col(id("id"))
            .col(text("jam_id"))
            .col(text("cart_id"))
            .col(text("user_id"))
            .col(text("submitted_at"))
            .foreign_key(&mut cascade("jam_entries", "jam_id", "jams"))
            .foreign_key(&mut cascade("jam_entries", "cart_id", "carts"))
            .foreign_key(&mut cascade("jam_entries", "user_id", "users"))
            .to_owned(),
    ]
}

fn indexes() -> Vec<IndexCreateStatement> {
    vec![
        index("idx_users_username", "users", &["username"])
            .unique()
            .to_owned(),
        index("idx_users_email_normalized", "users", &["email_normalized"])
            .unique()
            .to_owned(),
        index("idx_sessions_user", "sessions", &["user_id"]),
        index("idx_api_tokens_hash", "api_tokens", &["token_hash"])
            .unique()
            .to_owned(),
        index("idx_email_tokens_hash", "email_tokens", &["token_hash"])
            .unique()
            .to_owned(),
        index(
            "idx_oauth_identities_provider_subject",
            "oauth_identities",
            &["provider", "subject"],
        )
        .unique()
        .to_owned(),
        index(
            "idx_mfa_backup_codes_user",
            "mfa_backup_codes",
            &["user_id"],
        ),
        index(
            "idx_webauthn_credentials_user",
            "webauthn_credentials",
            &["user_id"],
        ),
        index(
            "idx_audit_log_user_created",
            "audit_log",
            &["user_id", "created_at"],
        ),
        index("idx_carts_uploaded_at", "carts", &["uploaded_at"]),
        index("idx_carts_parent_cart_id", "carts", &["parent_cart_id"]),
        index("idx_carts_root_cart_id", "carts", &["root_cart_id"]),
        index(
            "idx_cart_versions_cart_version",
            "cart_versions",
            &["cart_id", "version"],
        )
        .unique()
        .to_owned(),
        index(
            "idx_cart_versions_content_hash",
            "cart_versions",
            &["content_hash"],
        ),
        index("idx_ratings_cart_user", "ratings", &["cart_id", "user_id"])
            .unique()
            .to_owned(),
        index("idx_comments_cart", "comments", &["cart_id"]),
        index(
            "idx_play_events_cart_session",
            "play_events",
            &["cart_id", "session_key"],
        )
        .unique()
        .to_owned(),
        index("idx_play_events_played_at", "play_events", &["played_at"]),
        index(
            "idx_funnel_events_unique_viewer",
            "funnel_events",
            &["cart_id", "event", "viewer_key"],
        )
        .unique()
        .to_owned(),
        index(
            "idx_funnel_events_created_at",
            "funnel_events",
            &["created_at"],
        ),
        index(
            "idx_collection_carts_position",
            "collection_carts",
            &["collection_id", "position"],
        ),
        index(
            "idx_jam_entries_jam_cart",
            "jam_entries",
            &["jam_id", "cart_id"],
        )
        .unique()
        .to_owned(),
    ]
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for table in tables() {
            manager.create_table(table).await?;
        }
        for index in indexes() {
            manager.create_index(index).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for table in tables().into_iter().rev() {
            let name = table
                .get_table_name()
                .cloned()
                .ok_or_else(|| DbErr::Custom("table statement without a name".to_string()))?;
            manager
                .drop_table(Table::drop().table(name).to_owned())
                .await?;
        }
        Ok(())
    }
}
