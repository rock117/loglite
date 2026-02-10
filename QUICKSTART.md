# Loglite Quick Start Guide

## Prerequisites

- Rust 1.70+ (install from https://rustup.rs/)
- PostgreSQL 14+ (running on localhost:5432)
- Node.js 18+ (for frontend)

## 1. Database Setup

```bash
# Create database
psql -U postgres -c "CREATE DATABASE loglite;"

# The backend will automatically create tables on first run
```

## 2. Backend Setup

```bash
cd loglite-backend

# Copy environment configuration
cp .env.example .env

# Edit .env if needed (default values should work for local development)

# Build and run
cargo run --release
```

The API server will start on `http://localhost:8000`

## 3. Test the API

### Create an Application

```bash
curl -X POST http://localhost:8000/api/apps \
  -H "Content-Type: application/json" \
  -d '{"name": "my-app"}'
```

Response:
```json
{
  "app_id": "my-app-a1b2c3d4",
  "name": "my-app",
  "created_at": "2024-02-09T14:30:15Z"
}
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

### Auto-Detect Format (Recommended)

```bash
curl -X POST http://localhost:8000/api/ingest/auto \
  -H "Content-Type: text/plain" \
  -d "2024-02-09 22:30:15.123 ERROR [main] com.example.App - Error occurred
java.lang.NullPointerException
    at com.example.Service.process(Service.java:42)"
```

### Search Logs

```bash
curl -X POST http://localhost:8000/api/search \
  -H "Content-Type: application/json" \
  -d '{
    "app_id": "default",
    "q": "ERROR",
    "limit": 100
  }'
```

## 4. Frontend Setup (Optional)

```bash
cd loglite-frontend
npm install
npm run dev
```

The UI will be available at `http://localhost:5173`

## Supported Log Formats

### Java (Log4j/Logback)
```
2024-02-09 22:30:15.123 ERROR [main] com.example.App - Message
2024-02-09 22:30:15,123 WARN [thread] Logger - Message
```

### Rust (env_logger/tracing)
```
[2024-02-09T14:30:15Z ERROR my_app] Message
2024-02-09T14:30:15Z ERROR module: Message
```

### Go (standard/zap/logrus)
```
2024/02/09 22:30:15 [ERROR] main.go:42: Message
{"level":"error","ts":1707502215.123,"msg":"Message"}
```

## Features

✅ Multi-language log parsing (Java, Rust, Go)
✅ Multi-line log merging (exception stack traces)
✅ Format auto-detection
✅ Full-text search with Tantivy
✅ Multi-application support
✅ TTL-based log retention
✅ Structured field extraction

## Troubleshooting

### Database Connection Failed
- Ensure PostgreSQL is running: `pg_isready`
- Check database exists: `psql -U postgres -l | grep loglite`
- Verify connection string in `.env`

### Port Already in Use
- Change Rocket port: `ROCKET_PORT=8001` in `.env`
- Or kill existing process: `netstat -ano | findstr :8000`

### Index Directory Permission Error
- Ensure `LOGLITE_INDEX_DIR` path is writable
- Default: `./loglite-index` (created automatically)

## Next Steps

- Read full documentation in `README.md`
- Check test examples in `test_logs.md`
- Explore API endpoints at `http://localhost:8000/api/health`
- Configure log retention and TTL settings
- Set up file/directory tailing for automatic ingestion

## Support

For issues and questions, please check:
- Project README: `README.md`
- Test examples: `test_logs.md`
- Architecture documentation in README
