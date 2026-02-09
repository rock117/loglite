#[macro_use]
extern crate rocket;

use std::sync::Arc;

use anyhow::Result;
use chrono::{DateTime, FixedOffset, Utc};
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use rocket_cors::CorsOptions;
use sea_orm::entity::prelude::*;
use sea_orm::prelude::DateTimeWithTimeZone;
use sea_orm::{
    sea_query::TableCreateStatement, ActiveValue::NotSet, ColumnTrait, Condition, Database, DatabaseConnection,
    EntityTrait, QueryFilter, QueryOrder, QuerySelect, Schema, Set,
};
use serde::{Deserialize, Serialize};
use tracing_subscriber::EnvFilter;

mod entities {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
    #[sea_orm(table_name = "events")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = true)]
        pub id: i64,
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

    pub mod prelude {
        pub use super::Entity as Event;
    }
}

use entities::prelude::*;

#[derive(Clone)]
struct AppState {
    db: Arc<DatabaseConnection>,
}

#[derive(Debug, Deserialize)]
struct IngestEvent {
    #[serde(default = "default_ts")]
    ts: DateTimeWithTimeZone,
    #[serde(default)]
    host: String,
    #[serde(default)]
    source: String,
    #[serde(default)]
    sourcetype: Option<String>,
    #[serde(default)]
    severity: Option<i32>,
    message: String,
    #[serde(default)]
    fields: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct IngestRequest {
    events: Vec<IngestEvent>,
}

#[derive(Debug, Serialize)]
struct IngestResponse {
    accepted: usize,
}

#[derive(Debug, Deserialize)]
struct SearchRequest {
    #[serde(default)]
    q: Option<String>,
    #[serde(default)]
    sources: Option<Vec<String>>,
    #[serde(default)]
    hosts: Option<Vec<String>>,
    #[serde(default)]
    severities: Option<Vec<i32>>,
    #[serde(default)]
    start_ts: Option<DateTimeWithTimeZone>,
    #[serde(default)]
    end_ts: Option<DateTimeWithTimeZone>,
    #[serde(default = "default_limit")]
    limit: u64,
}

fn default_limit() -> u64 {
    100
}

#[derive(Debug, Serialize)]
struct SearchItem {
    id: i64,
    ts: DateTimeWithTimeZone,
    host: String,
    source: String,
    sourcetype: Option<String>,
    severity: Option<i32>,
    message: String,
    fields: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct SearchResponse {
    total: u64,
    items: Vec<SearchItem>,
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
}

fn default_ts() -> DateTimeWithTimeZone {
    let utc: DateTime<Utc> = Utc::now();
    utc.with_timezone(&FixedOffset::east_opt(0).unwrap())
}

#[get("/health")]
async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

#[post("/ingest", data = "<payload>")]
async fn ingest(state: &State<AppState>, payload: Json<IngestRequest>) -> Result<Json<IngestResponse>, Status> {
    let db = &state.db;

    let models: Vec<entities::ActiveModel> = payload
        .events
        .iter()
        .map(|e| entities::ActiveModel {
            id: NotSet,
            ts: Set(e.ts),
            host: Set(e.host.clone()),
            source: Set(e.source.clone()),
            sourcetype: Set(e.sourcetype.clone()),
            severity: Set(e.severity),
            message: Set(e.message.clone()),
            fields: Set(e.fields.clone()),
        })
        .collect();

    if models.is_empty() {
        return Ok(Json(IngestResponse { accepted: 0 }));
    }

    Event::insert_many(models)
        .exec(db.as_ref())
        .await
        .map_err(|_| Status::InternalServerError)?;

    Ok(Json(IngestResponse {
        accepted: payload.events.len(),
    }))
}

#[post("/search", data = "<query>")]
async fn search(state: &State<AppState>, query: Json<SearchRequest>) -> Result<Json<SearchResponse>, Status> {
    let db = &state.db;
    let mut cond = Condition::all();

    if let Some(start) = query.start_ts {
        cond = cond.add(entities::Column::Ts.gte(start));
    }
    if let Some(end) = query.end_ts {
        cond = cond.add(entities::Column::Ts.lte(end));
    }
    if let Some(ref sources) = query.sources {
        if !sources.is_empty() {
            cond = cond.add(entities::Column::Source.is_in(sources.clone()));
        }
    }
    if let Some(ref hosts) = query.hosts {
        if !hosts.is_empty() {
            cond = cond.add(entities::Column::Host.is_in(hosts.clone()));
        }
    }
    if let Some(ref severities) = query.severities {
        if !severities.is_empty() {
            cond = cond.add(entities::Column::Severity.is_in(severities.clone()));
        }
    }
    if let Some(ref text) = query.q {
        if !text.is_empty() {
            let pattern = format!("%{}%", text);
            cond = cond.add(entities::Column::Message.like(pattern));
        }
    }

    let finder = Event::find().filter(cond).order_by_desc(entities::Column::Ts);

    let total = finder
        .clone()
        .count(db.as_ref())
        .await
        .map_err(|_| Status::InternalServerError)?;

    let limit = query.limit.min(1000); // safety cap

    let items = finder
        .limit(limit)
        .all(db.as_ref())
        .await
        .map_err(|_| Status::InternalServerError)?
        .into_iter()
        .map(|m| SearchItem {
            id: m.id,
            ts: m.ts,
            host: m.host,
            source: m.source,
            sourcetype: m.sourcetype,
            severity: m.severity,
            message: m.message,
            fields: m.fields,
        })
        .collect();

    Ok(Json(SearchResponse { total, items }))
}

async fn init_db(db_url: &str) -> Result<DatabaseConnection> {
    let db = Database::connect(db_url).await?;

    let backend = db.get_database_backend();
    let schema = Schema::new(backend);
    let stmt: TableCreateStatement = schema
        .create_table_from_entity(Event)
        .if_not_exists()
        .to_owned();

    db.execute(backend.build(&stmt)).await?;

    Ok(db)
}

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let db_url = std::env::var("LOGLITE_DB_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost/loglite".to_string());
    let db = init_db(&db_url)
        .await
        .expect("failed to init database");

    let state = AppState {
        db: Arc::new(db),
    };

    let cors = CorsOptions::default()
        .to_cors()
        .expect("failed to build CORS");

    let _ = rocket::build()
        .manage(state)
        .mount("/api", routes![health, ingest, search])
        .attach(cors)
        .launch()
        .await?;

    Ok(())
}
