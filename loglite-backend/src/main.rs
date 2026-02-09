#[macro_use]
extern crate rocket;

use std::sync::Arc;

use anyhow::Result;
use chrono::{DateTime, FixedOffset, Utc};
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use rocket_cors::CorsOptions;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::{Field, SchemaBuilder, TEXT, STORED};
use tantivy::schema::OwnedValue;
use tantivy::{doc, Index, IndexReader, IndexWriter};
use sea_orm::entity::prelude::*;
use sea_orm::prelude::DateTimeWithTimeZone;
use sea_orm::{
    sea_query::TableCreateStatement, ActiveModelTrait, ActiveValue::NotSet, ColumnTrait, Condition, Database,
    DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, QuerySelect, Schema, Set,
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
    search: Arc<SearchState>,
}

struct SearchState {
    reader: IndexReader,
    writer: parking_lot::Mutex<IndexWriter>,
    field_event_id: Field,
    field_ts_epoch_ms: Field,
    field_host: Field,
    field_source: Field,
    field_message: Field,
    query_parser: QueryParser,
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

async fn ingest_events(state: &AppState, events: &[IngestEvent]) -> Result<usize, Status> {
    if events.is_empty() {
        return Ok(0);
    }

    let db = &state.db;
    let mut writer = state.search.writer.lock();

    for e in events {
        let inserted = entities::ActiveModel {
            id: NotSet,
            ts: Set(e.ts),
            host: Set(e.host.clone()),
            source: Set(e.source.clone()),
            sourcetype: Set(e.sourcetype.clone()),
            severity: Set(e.severity),
            message: Set(e.message.clone()),
            fields: Set(e.fields.clone()),
        }
        .insert(db.as_ref())
        .await
        .map_err(|_| Status::InternalServerError)?;

        let ts_epoch_ms = inserted.ts.timestamp_millis();

        writer
            .add_document(doc!(
                state.search.field_event_id => inserted.id,
                state.search.field_ts_epoch_ms => ts_epoch_ms,
                state.search.field_host => inserted.host,
                state.search.field_source => inserted.source,
                state.search.field_message => inserted.message
            ))
            .map_err(|_| Status::InternalServerError)?;
    }

    writer.commit().map_err(|_| Status::InternalServerError)?;
    state
        .search
        .reader
        .reload()
        .map_err(|_| Status::InternalServerError)?;

    Ok(events.len())
}

#[post("/ingest", data = "<payload>")]
async fn ingest(state: &State<AppState>, payload: Json<IngestRequest>) -> Result<Json<IngestResponse>, Status> {
    let accepted = ingest_events(state.inner(), &payload.events).await?;

    Ok(Json(IngestResponse {
        accepted,
    }))
}

fn parse_nginx_access_line(line: &str) -> Option<(String, serde_json::Value)> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }

    let mut parts = line.splitn(2, ' ');
    let remote_addr = parts.next()?.to_string();

    Some((
        line.to_string(),
        serde_json::json!({
            "remote_addr": remote_addr
        }),
    ))
}

#[post("/ingest/nginx", data = "<body>")]
async fn ingest_nginx(state: &State<AppState>, body: String) -> Result<Json<IngestResponse>, Status> {
    let mut events: Vec<IngestEvent> = Vec::new();
    for line in body.lines() {
        if let Some((msg, fields)) = parse_nginx_access_line(line) {
            events.push(IngestEvent {
                ts: default_ts(),
                host: String::new(),
                source: "nginx".to_string(),
                sourcetype: Some("nginx_access".to_string()),
                severity: None,
                message: msg,
                fields,
            });
        }
    }

    let accepted = ingest_events(state.inner(), &events).await?;
    Ok(Json(IngestResponse { accepted }))
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
    let q = query.q.clone().unwrap_or_default();
    let q = q.trim().to_string();

    if !q.is_empty() {
        let searcher = state.search.reader.searcher();
        let tantivy_query = state
            .search
            .query_parser
            .parse_query(&q)
            .map_err(|_| Status::BadRequest)?;

        let top_docs = searcher
            .search(&tantivy_query, &TopDocs::with_limit(query.limit.min(1000) as usize))
            .map_err(|_| Status::InternalServerError)?;

        let mut ids: Vec<i64> = Vec::with_capacity(top_docs.len());
        for (_score, addr) in top_docs {
            let retrieved = searcher
                .doc(addr)
                .map_err(|_| Status::InternalServerError)?;
            if let Some(v) = retrieved.get_first(state.search.field_event_id) {
                if let OwnedValue::I64(id) = v {
                    ids.push(*id);
                }
            }
        }

        if ids.is_empty() {
            return Ok(Json(SearchResponse { total: 0, items: vec![] }));
        }

        cond = cond.add(entities::Column::Id.is_in(ids.clone()));
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

fn init_search(index_dir: &str) -> Result<SearchState> {
    let mut schema_builder = SchemaBuilder::default();
    let field_event_id = schema_builder.add_i64_field("event_id", STORED);
    let field_ts_epoch_ms = schema_builder.add_i64_field("ts_epoch_ms", STORED);
    let field_host = schema_builder.add_text_field("host", TEXT | STORED);
    let field_source = schema_builder.add_text_field("source", TEXT | STORED);
    let field_message = schema_builder.add_text_field("message", TEXT | STORED);
    let schema = schema_builder.build();

    let path = std::path::Path::new(index_dir);
    let index = if let Ok(idx) = Index::open_in_dir(path) {
        idx
    } else {
        Index::create_in_dir(path, schema.clone())?
    };

    let reader = index
        .reader_builder()
        .reload_policy(tantivy::ReloadPolicy::Manual)
        .try_into()?;
    let writer = index.writer(50_000_000)?;

    let query_parser = QueryParser::for_index(&index, vec![field_message, field_host, field_source]);

    Ok(SearchState {
        reader,
        writer: parking_lot::Mutex::new(writer),
        field_event_id,
        field_ts_epoch_ms,
        field_host,
        field_source,
        field_message,
        query_parser,
    })
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

    let index_dir = std::env::var("LOGLITE_INDEX_DIR").unwrap_or_else(|_| "loglite-index".to_string());
    let search = init_search(&index_dir).expect("failed to init search index");

    let state = AppState {
        db: Arc::new(db),
        search: Arc::new(search),
    };

    let cors = CorsOptions::default()
        .to_cors()
        .expect("failed to build CORS");

    let _ = rocket::build()
        .manage(state)
        .mount("/api", routes![health, ingest, ingest_nginx, search])
        .attach(cors)
        .launch()
        .await?;

    Ok(())
}
