use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// A short-lived server-held WebAuthn ceremony state, between the
/// `start`/`finish` steps of registration or authentication. `user_id` is
/// `None` for login challenges (identity isn't confirmed until the
/// credential itself is verified).
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "webauthn_challenges")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub user_id: Option<String>,
    /// `register` or `authenticate`.
    pub kind: String,
    /// Serialized `PasskeyRegistration` or `PasskeyAuthentication` (JSON).
    pub state_json: String,
    pub expires_at: String,
    pub created_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
