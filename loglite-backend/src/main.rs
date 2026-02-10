#[macro_use]
extern crate rocket;

use std::sync::Arc;
use tracing_subscriber::EnvFilter;

mod db;
mod entities;
mod handlers;
mod id_gen;
mod models;
mod search_engine;
mod state;
mod tail;
mod utils;

use db::init_db;
use handlers::{
    create_app, create_source, delete_source, get_source, health_handler, ingest_auto, ingest_go,
    ingest_handler, ingest_java, ingest_nginx, ingest_rust, list_apps, list_sources,
    search_handler_fn, ttl_cleanup_loop, update_source,
};
use id_gen::Snowflake;
use search_engine::init_search;
use state::AppState;
use tail::tail_ingestion_loop;

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let db_url = std::env::var("LOGLITE_DB_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost/loglite".to_string());
    let db = init_db(&db_url).await.expect("failed to init database");

    let index_dir =
        std::env::var("LOGLITE_INDEX_DIR").unwrap_or_else(|_| "loglite-index".to_string());
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
    tokio::spawn(tail_ingestion_loop(Arc::new(state.clone())));

    let cors = rocket_cors::CorsOptions::default()
        .to_cors()
        .expect("failed to build CORS");

    let _ = rocket::build()
        .manage(state)
        .mount(
            "/api",
            routes![
                health_handler,
                create_app,
                list_apps,
                create_source,
                list_sources,
                get_source,
                update_source,
                delete_source,
                ingest_handler,
                ingest_nginx,
                ingest_java,
                ingest_rust,
                ingest_go,
                ingest_auto,
                search_handler_fn
            ],
        )
        .attach(cors)
        .launch()
        .await?;

    Ok(())
}
