use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "follows")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub follower_id: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub followed_id: String,
    pub created_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
