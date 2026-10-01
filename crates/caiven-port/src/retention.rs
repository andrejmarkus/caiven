//! Storage limitation (GDPR Art. 5(1)(e)): expired auth rows are purged and
//! old personal data is deleted or anonymized. The periods here are the ones
//! the privacy policy promises — change both together.

use chrono::{Duration, Utc};
use sea_orm::{
    ColumnTrait, DatabaseConnection, DbErr, EntityTrait, ExprTrait, QueryFilter, sea_query::Expr,
};

use crate::entities::{
    audit_log, email_tokens, funnel_events, mfa_challenges, moderation_actions, play_events,
    sessions, studio_link_requests, webauthn_challenges,
};

pub const AUDIT_LOG_DAYS: i64 = 365;
pub const VIEWER_KEY_DAYS: i64 = 90;
pub const MODERATION_LOG_DAYS: i64 = 3 * 365;

fn ago(days: i64) -> String {
    (Utc::now() - Duration::days(days)).to_rfc3339()
}

async fn purge<E: EntityTrait>(
    db: &DatabaseConnection,
    col: E::Column,
    before: &str,
) -> Result<(), DbErr> {
    E::delete_many().filter(col.lt(before)).exec(db).await?;
    Ok(())
}

/// Viewer keys are hashes of an IP or user id; swapping in the row id keeps
/// per-cart counts and uniqueness while cutting the link to a person.
async fn anonymize<E: EntityTrait>(
    db: &DatabaseConnection,
    [id, key, time]: [E::Column; 3],
) -> Result<(), DbErr> {
    E::update_many()
        .col_expr(key, Expr::col(id))
        .filter(time.lt(ago(VIEWER_KEY_DAYS)))
        .filter(Expr::col(key).ne(Expr::col(id)))
        .exec(db)
        .await?;
    Ok(())
}

pub async fn sweep(db: &DatabaseConnection) -> Result<(), DbErr> {
    let now = Utc::now().to_rfc3339();
    purge::<sessions::Entity>(db, sessions::Column::ExpiresAt, &now).await?;
    purge::<email_tokens::Entity>(db, email_tokens::Column::ExpiresAt, &now).await?;
    purge::<mfa_challenges::Entity>(db, mfa_challenges::Column::ExpiresAt, &now).await?;
    purge::<webauthn_challenges::Entity>(db, webauthn_challenges::Column::ExpiresAt, &now).await?;
    // A day of grace so a late Studio poll still sees "expired", not "unknown".
    purge::<studio_link_requests::Entity>(db, studio_link_requests::Column::ExpiresAt, &ago(1))
        .await?;
    purge::<audit_log::Entity>(db, audit_log::Column::CreatedAt, &ago(AUDIT_LOG_DAYS)).await?;
    purge::<moderation_actions::Entity>(
        db,
        moderation_actions::Column::CreatedAt,
        &ago(MODERATION_LOG_DAYS),
    )
    .await?;
    use play_events::Column as P;
    anonymize::<play_events::Entity>(db, [P::Id, P::ViewerKey, P::PlayedAt]).await?;
    use funnel_events::Column as F;
    anonymize::<funnel_events::Entity>(db, [F::Id, F::ViewerKey, F::CreatedAt]).await
}
