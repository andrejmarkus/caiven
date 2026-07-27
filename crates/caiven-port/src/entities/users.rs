use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub username: String,
    pub password_hash: String,
    pub is_admin: bool,
    pub created_at: String,
    pub email: Option<String>,
    #[sea_orm(default_value = false)]
    pub email_verified: bool,
    pub email_normalized: Option<String>,
    pub mfa_totp_secret: Option<String>,
    #[sea_orm(default_value = false)]
    pub mfa_enabled: bool,
    #[sea_orm(default_value = true)]
    pub password_set: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
