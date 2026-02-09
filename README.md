# Loglite

A lightweight, high-performance log ingestion and search system inspired by Splunk, designed for single-machine deployment with multi-application support.

## 🚧 Project Status

**This project is currently under active development.** Core features are being implemented and the API may change.

### Current Progress
- ✅ Backend scaffolding (Rust + Rocket + SeaORM + Tantivy)
- ✅ Multi-application (multi-tenant) support with app-scoped indexing
- ✅ Snowflake-based distributed ID generation
- ✅ Full-text search with Tantivy
- ✅ TTL-based log retention and cleanup
- ✅ Basic ingestion and search APIs
- ✅ **Multi-language log parsing (Java, Rust, Go) with auto-detection**
- ✅ **Multi-line log merging (exception stack traces)**
- 🚧 File/directory tail ingestion with offset persistence
- 🚧 Frontend UI (Vue3)
- ⏳ Per-app source configuration (include/exclude patterns, recursive scanning)
- ⏳ Authentication and rate limiting

## 📖 Overview

Loglite provides a simplified alternative to enterprise log management systems, optimized for:
- **Single-machine deployment** with minimal operational overhead
- **Multi-application support** with logical isolation per app
- **Fast full-text search** powered by Tantivy
- **Flexible ingestion** from files, directories, and HTTP endpoints
- **Configurable retention** with automatic TTL cleanup

## 🏗️ Architecture

### Technology Stack

#### Backend
- **Language**: Rust
- **Web Framework**: [Rocket](https://rocket.rs/) - Type-safe, async web framework
- **Database**: PostgreSQL via [SeaORM](https://www.sea-ql.org/SeaORM/) - Async ORM with compile-time query validation
- **Search Engine**: [Tantivy](https://github.com/quickwit-oss/tantivy) - Full-text search library (Lucene-like)
- **ID Generation**: Snowflake algorithm for distributed, time-ordered IDs

#### Frontend
- **Framework**: Vue 3 with TypeScript
- **Build Tool**: Vite
- **HTTP Client**: Axios

### System Components

```
┌─────────────────────────────────────────────────────────────┐
│                         Frontend (Vue3)                      │
│                    Search UI + App Selector                  │
└──────────────────────────┬──────────────────────────────────┘
                           │ HTTP/JSON
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                    Backend (Rocket API)                      │
├─────────────────────────────────────────────────────────────┤
│  • /api/apps          - App management                       │
│  • /api/ingest        - Generic JSON ingestion               │
│  • /api/ingest/java   - Java log parsing                     │
│  • /api/ingest/rust   - Rust log parsing                     │
│  • /api/ingest/go     - Go log parsing                       │
│  • /api/ingest/auto   - Auto-detect format                   │
│  • /api/ingest/nginx  - Nginx access logs                    │
│  • /api/search        - Full-text search                     │
│  • /api/health        - Health check                         │
└──────────────┬──────────────────────────┬───────────────────┘
               │                          │
               ▼                          ▼
    ┌──────────────────┐      ┌──────────────────────┐
    │   PostgreSQL     │      │   Tantivy Index      │
    │                  │      │                      │
    │  • apps          │      │  • app_id (filter)   │
    │  • app_sources   │      │  • event_id          │
    │  • tail_offsets  │      │  • message (FTS)     │
    │  • events        │      │  • host, source      │
    └──────────────────┘      └──────────────────────┘
               ▲
               │
    ┌──────────────────┐
    │  Tail Ingestion  │
    │   (Background)   │
    │                  │
    │  • File scanning │
    │  • Offset track  │
    │  • Glob filters  │
    └──────────────────┘
```

### Data Model

#### Core Tables
- **`apps`**: Application registry with stable `app_id` generation (slug + hash)
- **`app_sources`**: Per-app ingestion sources (file/directory paths, glob patterns, encoding)
- **`tail_offsets`**: File read offsets for resumable tailing
- **`events`**: Canonical log events with snowflake IDs and app-scoped partitioning

#### Indexing Strategy
- **App-scoped indexing**: All events share a single Tantivy index with `app_id` as a mandatory filter
- **Searchable fields**: `message` (full-text), `host`, `source`, `app_id`
- **Stored fields**: `event_id`, `ts_epoch_ms` for result retrieval

### Key Features

#### 1. Multi-Application Support
Each application has:
- Unique `app_id` generated from name (e.g., `order-service-a1b2c3d4`)
- Isolated ingestion sources
- App-scoped search queries
- Independent TTL policies (planned)

#### 2. Snowflake ID Generation
- **64-bit IDs**: `timestamp(41 bits) | node_id(10 bits) | sequence(12 bits)`
- **Configurable node ID**: Set via `LOGLITE_NODE_ID` environment variable
- **Time-ordered**: Natural chronological sorting
- **Collision-free**: Up to 4096 IDs per millisecond per node

#### 3. TTL-Based Retention
- Configurable retention period (`LOGLITE_RETENTION_DAYS`)
- Periodic cleanup task (`LOGLITE_TTL_INTERVAL_SECS`)
- Synchronized deletion from both PostgreSQL and Tantivy index

#### 4. Tail Ingestion (In Development)
- Recursive directory scanning with glob patterns
- UTF-8 encoding support
- Offset persistence in database for crash recovery
- Include/exclude pattern matching

## 🚀 Getting Started

### Prerequisites
- Rust 1.70+ (with Cargo)
- PostgreSQL 14+
- Node.js 18+ (for frontend)

### Environment Variables

```bash
# Database connection
LOGLITE_DB_URL=postgres://user:password@localhost/loglite

# Tantivy index directory
LOGLITE_INDEX_DIR=./loglite-index

# Retention settings
LOGLITE_RETENTION_DAYS=7
LOGLITE_TTL_INTERVAL_SECS=300

# Snowflake ID generation
LOGLITE_NODE_ID=1
```

### Running the Backend

```bash
cd loglite-backend
cargo build --release
cargo run
```

The API server will start on `http://localhost:8000`.

### Running the Frontend

```bash
cd loglite-frontend
npm install
npm run dev
```

The UI will be available at `http://localhost:5173`.

## 📡 API Examples

### Create an Application
```bash
curl -X POST http://localhost:8000/api/apps \
  -H "Content-Type: application/json" \
  -d '{"name": "order-service"}'
```

### Ingest Events (Generic JSON)
```bash
curl -X POST http://localhost:8000/api/ingest \
  -H "Content-Type: application/json" \
  -d '{
    "app_id": "order-service-a1b2c3d4",
    "events": [
      {
        "message": "Order #12345 created",
        "host": "web-01",
        "source": "/var/log/app.log",
        "severity": 6
      }
    ]
  }'
```

### Search Logs
```bash
curl -X POST http://localhost:8000/api/search \
  -H "Content-Type: application/json" \
  -d '{
    "app_id": "order-service-a1b2c3d4",
    "q": "error",
    "limit": 100
  }'
```

### Ingest Java Logs (with Stack Traces)
```bash
curl -X POST http://localhost:8000/api/ingest/java \
  -H "Content-Type: text/plain" \
  -d "2024-02-09 22:30:15.123 ERROR [main] com.example.App - Connection failed
java.lang.NullPointerException: Cannot invoke method
    at com.example.Service.process(Service.java:42)
    at com.example.App.main(App.java:15)
2024-02-09 22:30:16.456 INFO [worker-1] com.example.Service - Processing request"
```

### Ingest Rust Logs
```bash
curl -X POST http://localhost:8000/api/ingest/rust \
  -H "Content-Type: text/plain" \
  -d "[2024-02-09T14:30:15Z ERROR my_app] Database connection lost
[2024-02-09T14:30:16Z INFO  my_app::handler] Request completed"
```

### Ingest Go Logs
```bash
curl -X POST http://localhost:8000/api/ingest/go \
  -H "Content-Type: text/plain" \
  -d "2024/02/09 22:30:15 [ERROR] main.go:42: Failed to connect
2024/02/09 22:30:16 [INFO] handler.go:28: Request processed"
```

### Auto-Detect Log Format (Recommended)
```bash
curl -X POST http://localhost:8000/api/ingest/auto \
  -H "Content-Type: text/plain" \
  -d "2024-02-09 22:30:15.123 ERROR [main] com.example.App - Error occurred
java.lang.NullPointerException
    at com.example.Service.process(Service.java:42)"
```

## 🛣️ Roadmap

- [ ] Complete tail ingestion implementation
- [ ] Frontend app selector with localStorage persistence
- [ ] Per-app source management UI
- [ ] Real-time log streaming (WebSocket)
- [ ] Authentication and authorization
- [ ] Rate limiting and quota management
- [ ] Metrics and monitoring dashboard
- [ ] Docker deployment support
- [ ] Backup and restore utilities

## 📝 License

This project is currently under development. License to be determined.

## 🤝 Contributing

As this project is in early development, contribution guidelines will be established soon.
