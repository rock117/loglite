use anyhow::Result;
use sea_orm::{
    sea_query::TableCreateStatement, ConnectionTrait, Database, DatabaseConnection, Schema,
};

use crate::entities::prelude::*;

/// Initialize database connection and create tables if needed.
pub async fn init_db(db_url: &str) -> Result<DatabaseConnection> {
    let db = Database::connect(db_url).await?;

    let backend = db.get_database_backend();
    let schema = Schema::new(backend);
    let stmts: Vec<TableCreateStatement> = vec![
        schema
            .create_table_from_entity(App)
            .if_not_exists()
            .to_owned(),
        schema
            .create_table_from_entity(AppSource)
            .if_not_exists()
            .to_owned(),
        schema
            .create_table_from_entity(TailOffset)
            .if_not_exists()
            .to_owned(),
        schema
            .create_table_from_entity(Event)
            .if_not_exists()
            .to_owned(),
    ];

    for stmt in stmts {
        db.execute(backend.build(&stmt)).await?;
    }

    Ok(db)
}
