use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "studio_link_requests")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub poll_secret_hash: String,
    pub approved_user_id: Option<String>,
    pub expires_at: String,
    pub consumed_at: Option<String>,
    pub cancelled_at: Option<String>,
    pub created_at: String,
    /// SHA-256 of the normalized confirmation code shown in Studio — the
    /// approving browser must type it back, so a phished `browser_url`
    /// alone (without also seeing the requester's own Studio window) can't
    /// approve the link.
    pub user_code_hash: String,
    pub code_attempts: i32,
}
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
