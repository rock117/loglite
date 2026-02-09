#[macro_use]
extern crate rocket;

use std::sync::atomic::{AtomicI64, AtomicU16, Ordering};
use std::sync::Arc;

use anyhow::Result;
use chrono::{DateTime, FixedOffset, Utc};
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use rocket_cors::CorsOptions;
use tantivy::collector::TopDocs;
use tantivy::query::{BooleanQuery, Occur, Query, TermQuery};
use tantivy::schema::{Field, IndexRecordOption, SchemaBuilder, Value, STORED, STRING, TEXT};
use tantivy::query::QueryParser;
use tantivy::schema::Term;
use tantivy::{doc, Index, IndexReader, IndexWriter, TantivyDocument};
use sha2::{Digest, Sha256};
use sea_orm::entity::prelude::*;
use sea_orm::prelude::DateTimeWithTimeZone;
use sea_orm::{
    sea_query::TableCreateStatement, ActiveModelTrait, ColumnTrait, Condition, Database, DatabaseConnection,
    EntityTrait, QueryFilter, QueryOrder, QuerySelect, Schema, Set,
};
use serde::{Deserialize, Serialize};
use tokio::time::{sleep, Duration};
use tracing_subscriber::EnvFilter;

mod entities {
    pub mod apps {
        use sea_orm::entity::prelude::*;

        /// Application registry.
        #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
        #[sea_orm(table_name = "apps")]
        pub struct Model {
            #[sea_orm(primary_key, auto_increment = false)]
            pub app_id: String,
            pub name: String,
            pub created_at: DateTimeWithTimeZone,
        }

        #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
        pub enum Relation {}

        impl ActiveModelBehavior for ActiveModel {}
    }

    pub mod app_sources {
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
    }

    pub mod tail_offsets {
        use sea_orm::entity::prelude::*;

        /// Tail offsets per source+file.
        #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
        #[sea_orm(table_name = "tail_offsets")]
        pub struct Model {
            #[sea_orm(primary_key, auto_increment = true)]
            pub id: i64,
            pub source_id: i64,
            pub file_path: String,
            pub offset_bytes: i64,
            pub updated_at: DateTimeWithTimeZone,
        }

        #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
        pub enum Relation {}

        impl ActiveModelBehavior for ActiveModel {}
    }

    pub mod events {
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
    }

    pub mod prelude {
        pub use super::app_sources::Entity as AppSource;
        pub use super::apps::Entity as App;
        pub use super::events::Entity as Event;
        pub use super::tail_offsets::Entity as TailOffset;
    }
}

use entities::prelude::*;

#[derive(Clone)]
struct AppState {
    db: Arc<DatabaseConnection>,
    search: Arc<SearchState>,
    ids: Arc<Snowflake>,
}

struct SearchState {
    index: Index,
    reader: IndexReader,
    writer: parking_lot::Mutex<IndexWriter>,
    field_app_id: Field,
    field_event_id: Field,
    field_ts_epoch_ms: Field,
    field_host: Field,
    field_source: Field,
    field_message: Field,
}

/// A minimal Snowflake-style ID generator.
///
/// This generator produces roughly time-ordered 64-bit integers suitable as primary keys.
/// It is configured by `LOGLITE_NODE_ID` to avoid collisions when multiple instances exist.
struct Snowflake {
    node_id: i64,
    last_ms: AtomicI64,
    seq: AtomicU16,
}

impl Snowflake {
    /// Create a new generator.
    fn new(node_id: i64) -> Self {
        Self {
            node_id,
            last_ms: AtomicI64::new(0),
            seq: AtomicU16::new(0),
        }
    }

    /// Generate the next unique id.
    fn next_id(&self) -> i64 {
        // Layout (high -> low): timestamp_ms (41) | node_id (10) | sequence (12)
        // This is a simplified variant and assumes node_id fits in 10 bits.
        let mut now_ms = Utc::now().timestamp_millis();
        loop {
            let last = self.last_ms.load(Ordering::SeqCst);
            if now_ms < last {
                now_ms = last;
            }

            if now_ms == last {
                let seq = self.seq.fetch_add(1, Ordering::SeqCst) & 0x0fff;
                if seq == 0 {
                    // Sequence overflow within the same millisecond.
                    while Utc::now().timestamp_millis() <= now_ms {}
                    now_ms = Utc::now().timestamp_millis();
                    continue;
                }
                return ((now_ms & 0x1ffffffffff) << 22) | ((self.node_id & 0x03ff) << 12) | (seq as i64);
            }

            self.last_ms.store(now_ms, Ordering::SeqCst);
            self.seq.store(0, Ordering::SeqCst);
            return ((now_ms & 0x1ffffffffff) << 22) | ((self.node_id & 0x03ff) << 12);
        }
    }
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
    app_id: String,
    events: Vec<IngestEvent>,
}

/// Request payload for creating a new application.
#[derive(Debug, Deserialize)]
struct CreateAppRequest {
    name: String,
}

/// Application summary for UI usage.
#[derive(Debug, Serialize)]
struct AppInfo {
    app_id: String,
    name: String,
    created_at: DateTimeWithTimeZone,
}

#[derive(Debug, Serialize)]
struct IngestResponse {
    accepted: usize,
}

#[derive(Debug, Deserialize)]
struct SearchRequest {
    app_id: String,
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
    app_id: String,
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

/// Generate a stable application id from a display name.
///
/// The generated `app_id` is deterministic and human-friendly: `<slug>-<hash8>`.
fn generate_app_id(name: &str) -> String {
    let slug = slugify(name);
    let mut hasher = Sha256::new();
    hasher.update(name.as_bytes());
    let digest = hasher.finalize();
    let hash8 = hex::encode(&digest[..4]);
    if slug.is_empty() {
        format!("app-{}", hash8)
    } else {
        format!("{}-{}", slug, hash8)
    }
}

/// Convert an application name into a URL-safe slug.
fn slugify(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut last_dash = false;
    for ch in s.chars() {
        let ch = ch.to_ascii_lowercase();
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    out.trim_matches('-').to_string()
}

/// Create an application.
#[post("/apps", data = "<payload>")]
async fn create_app(state: &State<AppState>, payload: Json<CreateAppRequest>) -> Result<Json<AppInfo>, Status> {
    let name = payload.name.trim();
    if name.is_empty() {
        return Err(Status::BadRequest);
    }

    let app_id = generate_app_id(name);
    let created_at = default_ts();

    let model = entities::apps::ActiveModel {
        app_id: Set(app_id.clone()),
        name: Set(name.to_string()),
        created_at: Set(created_at),
    }
    .insert(state.db.as_ref())
    .await
    .map_err(|_| Status::InternalServerError)?;

    Ok(Json(AppInfo {
        app_id: model.app_id,
        name: model.name,
        created_at: model.created_at,
    }))
}

/// List all applications.
#[get("/apps")]
async fn list_apps(state: &State<AppState>) -> Result<Json<Vec<AppInfo>>, Status> {
    let apps = App::find()
        .order_by_desc(entities::apps::Column::CreatedAt)
        .all(state.db.as_ref())
        .await
        .map_err(|_| Status::InternalServerError)?;

    Ok(Json(
        apps.into_iter()
            .map(|a| AppInfo {
                app_id: a.app_id,
                name: a.name,
                created_at: a.created_at,
            })
            .collect(),
    ))
}

/// Ingest events for a single application.
///
/// This function writes events to Postgres and mirrors the searchable fields into Tantivy.
async fn ingest_events_for_app(state: &AppState, app_id: &str, events: &[IngestEvent]) -> Result<usize, Status> {
    if events.is_empty() {
        return Ok(0);
    }

    let db = &state.db;
    let mut docs: Vec<tantivy::TantivyDocument> = Vec::with_capacity(events.len());

    for e in events {
        let inserted = entities::events::ActiveModel {
            id: Set(state.ids.next_id()),
            app_id: Set(app_id.to_string()),
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

        docs.push(doc!(
            state.search.field_app_id => inserted.app_id.clone(),
            state.search.field_event_id => inserted.id,
            state.search.field_ts_epoch_ms => ts_epoch_ms,
            state.search.field_host => inserted.host,
            state.search.field_source => inserted.source,
            state.search.field_message => inserted.message
        ));
    }

    {
        let mut writer = state.search.writer.lock();
        for d in docs {
            writer.add_document(d).map_err(|_| Status::InternalServerError)?;
        }
        writer.commit().map_err(|_| Status::InternalServerError)?;
    }

    state
        .search
        .reader
        .reload()
        .map_err(|_| Status::InternalServerError)?;

    Ok(events.len())
}

#[post("/ingest", data = "<payload>")]
async fn ingest(state: &State<AppState>, payload: Json<IngestRequest>) -> Result<Json<IngestResponse>, Status> {
    let accepted = ingest_events_for_app(state.inner(), &payload.app_id, &payload.events).await?;

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

    let accepted = ingest_events_for_app(state.inner(), "default", &events).await?;
    Ok(Json(IngestResponse { accepted }))
}

#[post("/search", data = "<query>")]
async fn search(state: &State<AppState>, query: Json<SearchRequest>) -> Result<Json<SearchResponse>, Status> {
    let db = &state.db;
    let mut cond = Condition::all();

    // App scoping is mandatory.
    cond = cond.add(entities::events::Column::AppId.eq(query.app_id.clone()));

    if let Some(start) = query.start_ts {
        cond = cond.add(entities::events::Column::Ts.gte(start));
    }
    if let Some(end) = query.end_ts {
        cond = cond.add(entities::events::Column::Ts.lte(end));
    }
    if let Some(ref sources) = query.sources {
        if !sources.is_empty() {
            cond = cond.add(entities::events::Column::Source.is_in(sources.clone()));
        }
    }
    if let Some(ref hosts) = query.hosts {
        if !hosts.is_empty() {
            cond = cond.add(entities::events::Column::Host.is_in(hosts.clone()));
        }
    }
    if let Some(ref severities) = query.severities {
        if !severities.is_empty() {
            cond = cond.add(entities::events::Column::Severity.is_in(severities.clone()));
        }
    }
    let q = query.q.clone().unwrap_or_default();
    let q = q.trim().to_string();

    if !q.is_empty() {
        let query_parser = QueryParser::for_index(
            &state.search.index,
            vec![
                state.search.field_message,
                state.search.field_host,
                state.search.field_source,
            ],
        );
        let searcher = state.search.reader.searcher();

        let app_term = Term::from_field_text(state.search.field_app_id, &query.app_id);
        let app_filter = TermQuery::new(app_term, IndexRecordOption::Basic);

        let user_q = query_parser.parse_query(&q).map_err(|_| Status::BadRequest)?;
        let combined: BooleanQuery = BooleanQuery::from(vec![
            (Occur::Must, Box::new(app_filter) as Box<dyn Query>),
            (Occur::Must, user_q),
        ]);

        let top_docs = searcher
            .search(&combined, &TopDocs::with_limit(query.limit.min(1000) as usize))
            .map_err(|_| Status::InternalServerError)?;

        let mut ids: Vec<i64> = Vec::with_capacity(top_docs.len());
        for (_score, addr) in top_docs {
            let retrieved: TantivyDocument = searcher
                .doc(addr)
                .map_err(|_| Status::InternalServerError)?;
            if let Some(v) = retrieved.get_first(state.search.field_event_id) {
                if let Some(id) = v.as_i64() {
                    ids.push(id);
                }
            }
        }

        if ids.is_empty() {
            return Ok(Json(SearchResponse { total: 0, items: vec![] }));
        }

        cond = cond.add(entities::events::Column::Id.is_in(ids.clone()));
    }

    let finder = Event::find().filter(cond).order_by_desc(entities::events::Column::Ts);

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
            app_id: m.app_id,
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
    let field_app_id = schema_builder.add_text_field("app_id", STRING | STORED);
    let field_event_id = schema_builder.add_i64_field("event_id", tantivy::schema::INDEXED | STORED);
    let field_ts_epoch_ms = schema_builder.add_i64_field("ts_epoch_ms", STORED);
    let field_host = schema_builder.add_text_field("host", TEXT | STORED);
    let field_source = schema_builder.add_text_field("source", TEXT | STORED);
    let field_message = schema_builder.add_text_field("message", TEXT | STORED);
    let schema = schema_builder.build();

    let path = std::path::Path::new(index_dir);
    std::fs::create_dir_all(path)?;
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

    Ok(SearchState {
        index,
        reader,
        writer: parking_lot::Mutex::new(writer),
        field_app_id,
        field_event_id,
        field_ts_epoch_ms,
        field_host,
        field_source,
        field_message,
    })
}

async fn ttl_cleanup_loop(state: Arc<AppState>) {
    let retention_days: i64 = std::env::var("LOGLITE_RETENTION_DAYS")
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(7);
    let interval_seconds: u64 = std::env::var("LOGLITE_TTL_INTERVAL_SECS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(300);

    loop {
        if retention_days > 0 {
            let cutoff = Utc::now() - chrono::Duration::days(retention_days);
            let cutoff = cutoff.with_timezone(&FixedOffset::east_opt(0).unwrap());

            let expired: Vec<i64> = match Event::find()
                .select_only()
                .column(entities::events::Column::Id)
                .filter(entities::events::Column::Ts.lt(cutoff))
                .limit(10_000)
                .into_tuple()
                .all(state.db.as_ref())
                .await
            {
                Ok(v) => v,
                Err(_) => {
                    sleep(Duration::from_secs(interval_seconds)).await;
                    continue;
                }
            };

            if !expired.is_empty() {
                let _ = Event::delete_many()
                    .filter(entities::events::Column::Id.is_in(expired.clone()))
                    .exec(state.db.as_ref())
                    .await;

                {
                    let mut writer = state.search.writer.lock();
                    for id in expired {
                        let term = Term::from_field_i64(state.search.field_event_id, id);
                        let q = TermQuery::new(term, IndexRecordOption::Basic);
                        let _ = writer.delete_query(Box::new(q));
                    }
                    let _ = writer.commit();
                }

                let _ = state.search.reader.reload();
            }
        }

        sleep(Duration::from_secs(interval_seconds)).await;
    }
}

async fn init_db(db_url: &str) -> Result<DatabaseConnection> {
    let db = Database::connect(db_url).await?;

    let backend = db.get_database_backend();
    let schema = Schema::new(backend);
    let stmts: Vec<TableCreateStatement> = vec![
        schema.create_table_from_entity(App).if_not_exists().to_owned(),
        schema.create_table_from_entity(AppSource).if_not_exists().to_owned(),
        schema.create_table_from_entity(TailOffset).if_not_exists().to_owned(),
        schema.create_table_from_entity(Event).if_not_exists().to_owned(),
    ];

    for stmt in stmts {
        db.execute(backend.build(&stmt)).await?;
    }

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

    let node_id: i64 = std::env::var("LOGLITE_NODE_ID")
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(1);
    let ids = Arc::new(Snowflake::new(node_id));

    let state = AppState {
        db: Arc::new(db),
        search: Arc::new(search),
        ids,
    };

    tokio::spawn(ttl_cleanup_loop(Arc::new(state.clone())));

    let cors = CorsOptions::default()
        .to_cors()
        .expect("failed to build CORS");

    let _ = rocket::build()
        .manage(state)
        .mount("/api", routes![health, create_app, list_apps, ingest, ingest_nginx, search])
        .attach(cors)
        .launch()
        .await?;

    Ok(())
}
