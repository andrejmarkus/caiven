use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "api_tokens")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub user_id: String,
    pub token_hash: String,
    pub name: String,
    pub created_at: String,
    pub last_used_at: Option<String>,
    /// `"full"` (self-service tokens, `/auth/tokens`) or `"publish"` (minted
    /// by the Studio link flow) — see `auth::AuthUser::require_full_scope`.
    pub scope: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
