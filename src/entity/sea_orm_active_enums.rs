//! SeaORM Entity. Generated by sea-orm-codegen 0.9.2

use sea_orm::entity::prelude::*;

#[derive(Debug, Clone, PartialEq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "store")]
pub enum Store {
    #[sea_orm(string_value = "QINIU")]
    Qiniu,
    #[sea_orm(string_value = "DISK")]
    Disk,
}
#[derive(Debug, Clone, PartialEq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "verified_wallet")]
pub enum VerifiedWallet {
    #[sea_orm(string_value = "TRUE")]
    True,
    #[sea_orm(string_value = "FALSE")]
    False,
}