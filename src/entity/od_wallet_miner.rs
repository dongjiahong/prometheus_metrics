//! SeaORM Entity. Generated by sea-orm-codegen 0.9.2

use super::sea_orm_active_enums::VerifiedWallet;
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "od_wallet_miner")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: u32,
    pub wallet: String,
    pub verified_wallet: VerifiedWallet,
    pub miner_id: String,
    pub deleted: i32,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        panic!("No RelationDef")
    }
}

impl ActiveModelBehavior for ActiveModel {}
