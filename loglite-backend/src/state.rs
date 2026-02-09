use sea_orm::DatabaseConnection;
use std::sync::Arc;

use crate::id_gen::Snowflake;
use crate::search_engine::SearchState;

/// Global application state shared across handlers.
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<DatabaseConnection>,
    pub search: Arc<SearchState>,
    pub ids: Arc<Snowflake>,
}
