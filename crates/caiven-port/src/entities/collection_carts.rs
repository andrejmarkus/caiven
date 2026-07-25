use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "collection_carts")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub collection_id: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub cart_id: String,
    pub position: i32,
    pub added_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
