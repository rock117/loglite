use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use sea_orm::{ColumnTrait, Condition, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect};
use tantivy::collector::TopDocs;
use tantivy::query::{BooleanQuery, Occur, Query, QueryParser, TermQuery};
use tantivy::schema::{IndexRecordOption, Term, Value};
use tantivy::TantivyDocument;

use crate::entities::prelude::*;
use crate::models::{SearchItem, SearchRequest, SearchResponse};
use crate::state::AppState;

/// Search logs endpoint.
#[post("/search", data = "<query>")]
pub async fn search(
    state: &State<AppState>,
    query: Json<SearchRequest>,
) -> Result<Json<SearchResponse>, Status> {
    let db = &state.db;
    let mut cond = Condition::all();

    // App scoping is mandatory.
    cond = cond.add(crate::entities::events::Column::AppId.eq(query.app_id.clone()));

    if let Some(start) = query.start_ts {
        cond = cond.add(crate::entities::events::Column::Ts.gte(start));
    }
    if let Some(end) = query.end_ts {
        cond = cond.add(crate::entities::events::Column::Ts.lte(end));
    }
    if let Some(ref sources) = query.sources {
        if !sources.is_empty() {
            cond = cond.add(crate::entities::events::Column::Source.is_in(sources.clone()));
        }
    }
    if let Some(ref hosts) = query.hosts {
        if !hosts.is_empty() {
            cond = cond.add(crate::entities::events::Column::Host.is_in(hosts.clone()));
        }
    }
    if let Some(ref severities) = query.severities {
        if !severities.is_empty() {
            cond = cond.add(crate::entities::events::Column::Severity.is_in(severities.clone()));
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

        let user_q = query_parser
            .parse_query(&q)
            .map_err(|_| Status::BadRequest)?;
        let combined: BooleanQuery = BooleanQuery::from(vec![
            (Occur::Must, Box::new(app_filter) as Box<dyn Query>),
            (Occur::Must, user_q),
        ]);

        let top_docs = searcher
            .search(
                &combined,
                &TopDocs::with_limit(query.limit.min(1000) as usize),
            )
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
            return Ok(Json(SearchResponse {
                total: 0,
                items: vec![],
            }));
        }

        cond = cond.add(crate::entities::events::Column::Id.is_in(ids.clone()));
    }

    let finder = Event::find()
        .filter(cond)
        .order_by_desc(crate::entities::events::Column::Ts);

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
