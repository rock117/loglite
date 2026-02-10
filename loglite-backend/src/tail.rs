use anyhow::Result;
use chrono::{FixedOffset, Utc};
use globset::{Glob, GlobSetBuilder};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use walkdir::WalkDir;

use crate::entities::prelude::*;
use crate::models::IngestEvent;
use crate::state::AppState;
use crate::utils::{detect_log_format, level_to_severity, merge_multiline_logs, LogFormat};

/// Configuration for a single tail source
#[derive(Debug, Clone)]
pub struct TailSourceConfig {
    pub source_id: i64,
    pub app_id: String,
    pub path: String,
    pub recursive: bool,
    pub encoding: String,
    pub include_glob: Option<String>,
    pub exclude_glob: Option<String>,
}

/// Tail ingestion manager
pub struct TailManager {
    state: Arc<AppState>,
    sources: Vec<TailSourceConfig>,
}

impl TailManager {
    /// Create a new tail manager
    pub fn new(state: Arc<AppState>) -> Self {
        Self {
            state,
            sources: Vec::new(),
        }
    }

    /// Load active sources from database
    pub async fn load_sources(&mut self) -> Result<()> {
        let sources = AppSource::find()
            .filter(crate::entities::app_sources::Column::Enabled.eq(true))
            .filter(crate::entities::app_sources::Column::Kind.eq("tail"))
            .all(self.state.db.as_ref())
            .await?;

        self.sources = sources
            .into_iter()
            .map(|s| TailSourceConfig {
                source_id: s.id,
                app_id: s.app_id,
                path: s.path,
                recursive: s.recursive,
                encoding: s.encoding,
                include_glob: s.include_glob,
                exclude_glob: s.exclude_glob,
            })
            .collect();

        Ok(())
    }

    /// Scan and tail all configured sources
    pub async fn tail_all_sources(&self) -> Result<()> {
        for source in &self.sources {
            if let Err(e) = self.tail_source(source).await {
                tracing::error!("Failed to tail source {}: {}", source.source_id, e);
            }
        }
        Ok(())
    }

    /// Tail a single source
    async fn tail_source(&self, config: &TailSourceConfig) -> Result<()> {
        let path = Path::new(&config.path);

        // Build glob matchers
        let mut include_builder = GlobSetBuilder::new();
        let mut exclude_builder = GlobSetBuilder::new();

        if let Some(ref pattern) = config.include_glob {
            include_builder.add(Glob::new(pattern)?);
        }
        if let Some(ref pattern) = config.exclude_glob {
            exclude_builder.add(Glob::new(pattern)?);
        }

        let include_set = include_builder.build()?;
        let exclude_set = exclude_builder.build()?;

        // Collect files to process
        let mut files_to_process = Vec::new();

        if path.is_file() {
            files_to_process.push(path.to_path_buf());
        } else if path.is_dir() {
            let walker = if config.recursive {
                WalkDir::new(path)
            } else {
                WalkDir::new(path).max_depth(1)
            };

            for entry in walker.into_iter().filter_map(|e| e.ok()) {
                if !entry.file_type().is_file() {
                    continue;
                }

                let file_path = entry.path();

                // Apply glob filters
                if config.include_glob.is_some() && !include_set.is_match(file_path) {
                    continue;
                }
                if config.exclude_glob.is_some() && exclude_set.is_match(file_path) {
                    continue;
                }

                files_to_process.push(file_path.to_path_buf());
            }
        }

        // Process each file
        for file_path in files_to_process {
            if let Err(e) = self.tail_file(config, &file_path).await {
                tracing::error!("Failed to tail file {:?}: {}", file_path, e);
            }
        }

        Ok(())
    }

    /// Tail a single file
    async fn tail_file(&self, config: &TailSourceConfig, file_path: &Path) -> Result<()> {
        let file_path_str = file_path.to_string_lossy().to_string();

        // Get or create offset record
        let offset_record = TailOffset::find()
            .filter(crate::entities::tail_offsets::Column::SourceId.eq(config.source_id))
            .filter(crate::entities::tail_offsets::Column::FilePath.eq(&file_path_str))
            .one(self.state.db.as_ref())
            .await?;

        let mut current_offset = offset_record
            .as_ref()
            .map(|r| r.offset_bytes as u64)
            .unwrap_or(0);

        // Open file and seek to offset
        let mut file = File::open(file_path)?;
        let file_size = file.metadata()?.len();

        // If offset is beyond file size, file might have been truncated
        if current_offset > file_size {
            tracing::warn!(
                "Offset {} beyond file size {} for {:?}, resetting to 0",
                current_offset,
                file_size,
                file_path
            );
            current_offset = 0;
        }

        file.seek(SeekFrom::Start(current_offset))?;

        // Read new lines
        let reader = BufReader::new(file);
        let mut lines = Vec::new();
        let mut new_offset = current_offset;

        for line in reader.lines() {
            match line {
                Ok(line_str) => {
                    new_offset += line_str.len() as u64 + 1; // +1 for newline
                    lines.push(line_str);
                }
                Err(e) => {
                    tracing::error!("Error reading line from {:?}: {}", file_path, e);
                    break;
                }
            }
        }

        // If no new lines, nothing to do
        if lines.is_empty() {
            return Ok(());
        }

        // Detect log format
        let line_refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
        let format = detect_log_format(&line_refs);

        // Parse logs
        let log_entries = if format == LogFormat::Unknown {
            // Treat as plain text
            let utc = Utc::now();
            let ts = utc.with_timezone(&FixedOffset::east_opt(0).unwrap());
            lines
                .into_iter()
                .map(|line| crate::utils::LogEntry {
                    timestamp: ts,
                    level: "INFO".to_string(),
                    message: line,
                    stacktrace: None,
                    fields: serde_json::json!({}),
                })
                .collect()
        } else {
            merge_multiline_logs(line_refs, format)
        };

        // Convert to IngestEvent
        let source_name = file_path.file_name().unwrap_or_default().to_string_lossy();
        let mut events: Vec<IngestEvent> = Vec::new();

        for entry in log_entries {
            events.push(IngestEvent {
                ts: entry.timestamp,
                host: String::new(),
                source: source_name.to_string(),
                sourcetype: Some(format!("{:?}", format).to_lowercase()),
                severity: level_to_severity(&entry.level),
                message: entry.message,
                fields: entry.fields,
            });
        }

        // Ingest events
        if !events.is_empty() {
            self.ingest_events(&config.app_id, events).await?;
        }

        // Update offset
        self.update_offset(config.source_id, &file_path_str, new_offset as i64)
            .await?;

        Ok(())
    }

    /// Ingest events into database and search index
    async fn ingest_events(&self, app_id: &str, events: Vec<IngestEvent>) -> Result<()> {
        use tantivy::doc;

        let db = &self.state.db;
        let mut docs: Vec<tantivy::TantivyDocument> = Vec::with_capacity(events.len());

        for e in events {
            let inserted = crate::entities::events::ActiveModel {
                id: Set(self.state.ids.next_id()),
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
            .await?;

            let ts_epoch_ms = inserted.ts.timestamp_millis();

            docs.push(doc!(
                self.state.search.field_app_id => inserted.app_id.clone(),
                self.state.search.field_event_id => inserted.id,
                self.state.search.field_ts_epoch_ms => ts_epoch_ms,
                self.state.search.field_host => inserted.host,
                self.state.search.field_source => inserted.source,
                self.state.search.field_message => inserted.message
            ));
        }

        {
            let mut writer = self.state.search.writer.lock();
            for d in docs {
                writer.add_document(d)?;
            }
            writer.commit()?;
        }

        self.state.search.reader.reload()?;

        Ok(())
    }

    /// Update offset record in database
    async fn update_offset(&self, source_id: i64, file_path: &str, offset: i64) -> Result<()> {
        let existing = TailOffset::find()
            .filter(crate::entities::tail_offsets::Column::SourceId.eq(source_id))
            .filter(crate::entities::tail_offsets::Column::FilePath.eq(file_path))
            .one(self.state.db.as_ref())
            .await?;

        let utc = Utc::now();
        let updated_at = utc.with_timezone(&FixedOffset::east_opt(0).unwrap());

        if let Some(record) = existing {
            let mut active: crate::entities::tail_offsets::ActiveModel = record.into();
            active.offset_bytes = Set(offset);
            active.updated_at = Set(updated_at);
            active.update(self.state.db.as_ref()).await?;
        } else {
            crate::entities::tail_offsets::ActiveModel {
                id: Set(0), // Auto-increment
                source_id: Set(source_id),
                file_path: Set(file_path.to_string()),
                offset_bytes: Set(offset),
                updated_at: Set(updated_at),
            }
            .insert(self.state.db.as_ref())
            .await?;
        }

        Ok(())
    }
}

/// Background task for tail ingestion
pub async fn tail_ingestion_loop(state: Arc<AppState>) {
    let interval_seconds: u64 = std::env::var("LOGLITE_TAIL_INTERVAL_SECS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(10);

    loop {
        let mut manager = TailManager::new(state.clone());

        // Load sources
        if let Err(e) = manager.load_sources().await {
            tracing::error!("Failed to load tail sources: {}", e);
            sleep(Duration::from_secs(interval_seconds)).await;
            continue;
        }

        // Tail all sources
        if let Err(e) = manager.tail_all_sources().await {
            tracing::error!("Failed to tail sources: {}", e);
        }

        sleep(Duration::from_secs(interval_seconds)).await;
    }
}
