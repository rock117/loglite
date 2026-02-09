use chrono::{DateTime, FixedOffset, Utc};
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use sea_orm::prelude::DateTimeWithTimeZone;
use sea_orm::{ActiveModelTrait, Set};
use tantivy::doc;

use crate::models::{IngestEvent, IngestRequest, IngestResponse};
use crate::state::AppState;
use crate::utils::parse_nginx_access_line;

fn default_ts() -> DateTimeWithTimeZone {
    let utc: DateTime<Utc> = Utc::now();
    utc.with_timezone(&FixedOffset::east_opt(0).unwrap())
}

/// Ingest events for a single application.
///
/// This function writes events to Postgres and mirrors the searchable fields into Tantivy.
async fn ingest_events_for_app(
    state: &AppState,
    app_id: &str,
    events: &[IngestEvent],
) -> Result<usize, Status> {
    if events.is_empty() {
        return Ok(0);
    }

    let db = &state.db;
    let mut docs: Vec<tantivy::TantivyDocument> = Vec::with_capacity(events.len());

    for e in events {
        let inserted = crate::entities::events::ActiveModel {
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
            writer
                .add_document(d)
                .map_err(|_| Status::InternalServerError)?;
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

/// Ingest events endpoint.
#[post("/ingest", data = "<payload>")]
pub async fn ingest(
    state: &State<AppState>,
    payload: Json<IngestRequest>,
) -> Result<Json<IngestResponse>, Status> {
    let accepted = ingest_events_for_app(state.inner(), &payload.app_id, &payload.events).await?;

    Ok(Json(IngestResponse { accepted }))
}

/// Ingest nginx access logs endpoint.
#[post("/ingest/nginx", data = "<body>")]
pub async fn ingest_nginx(
    state: &State<AppState>,
    body: String,
) -> Result<Json<IngestResponse>, Status> {
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
