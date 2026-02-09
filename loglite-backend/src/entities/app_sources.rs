use sea_orm::entity::prelude::*;

/// Log ingestion sources configured per application.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "app_sources")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i64,
    pub app_id: String,
    pub kind: String,
    pub path: String,
    pub recursive: bool,
    pub encoding: String,
    pub include_glob: Option<String>,
    pub exclude_glob: Option<String>,
    pub enabled: bool,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
