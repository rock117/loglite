use sea_orm::entity::prelude::*;

/// The canonical stored event.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "events")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: i64,
    pub app_id: String,
    pub ts: DateTimeWithTimeZone,
    pub host: String,
    pub source: String,
    pub sourcetype: Option<String>,
    pub severity: Option<i32>,
    pub message: String,
    pub fields: serde_json::Value,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
