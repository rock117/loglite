use chrono::{FixedOffset, Utc};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};
use std::sync::Arc;
use tantivy::query::TermQuery;
use tantivy::schema::{IndexRecordOption, Term};
use tokio::time::{sleep, Duration};

use crate::entities::prelude::*;
use crate::state::AppState;

/// Background task for TTL-based log cleanup.
///
/// Periodically deletes expired events from both PostgreSQL and Tantivy index.
pub async fn ttl_cleanup_loop(state: Arc<AppState>) {
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
                .column(crate::entities::events::Column::Id)
                .filter(crate::entities::events::Column::Ts.lt(cutoff))
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
                    .filter(crate::entities::events::Column::Id.is_in(expired.clone()))
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
