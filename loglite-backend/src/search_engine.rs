use anyhow::Result;
use tantivy::schema::{Field, SchemaBuilder, STORED, STRING, TEXT};
use tantivy::{Index, IndexReader, IndexWriter};

/// Tantivy search state with index, reader, writer, and field handles.
pub struct SearchState {
    pub index: Index,
    pub reader: IndexReader,
    pub writer: parking_lot::Mutex<IndexWriter>,
    pub field_app_id: Field,
    pub field_event_id: Field,
    pub field_ts_epoch_ms: Field,
    pub field_host: Field,
    pub field_source: Field,
    pub field_message: Field,
}

/// Initialize the Tantivy search index.
pub fn init_search(index_dir: &str) -> Result<SearchState> {
    let mut schema_builder = SchemaBuilder::default();
    let field_app_id = schema_builder.add_text_field("app_id", STRING | STORED);
    let field_event_id =
        schema_builder.add_i64_field("event_id", tantivy::schema::INDEXED | STORED);
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
