# Multi-Language Log Parsing Test Examples

## Java Log Example

```bash
curl -X POST http://localhost:8000/api/ingest/java \
  -H "Content-Type: text/plain" \
  -d "2024-02-09 22:30:15.123 ERROR [main] com.example.App - Connection failed
java.lang.NullPointerException: Cannot invoke method
    at com.example.Service.process(Service.java:42)
    at com.example.App.main(App.java:15)
2024-02-09 22:30:16.456 INFO [worker-1] com.example.Service - Processing request"
```

## Rust Log Example

```bash
curl -X POST http://localhost:8000/api/ingest/rust \
  -H "Content-Type: text/plain" \
  -d "[2024-02-09T14:30:15Z ERROR my_app] Database connection lost
[2024-02-09T14:30:16Z INFO  my_app::handler] Request completed
[2024-02-09T14:30:17Z WARN  my_app::db] Retrying connection"
```

## Go Log Example

### Standard Format
```bash
curl -X POST http://localhost:8000/api/ingest/go \
  -H "Content-Type: text/plain" \
  -d "2024/02/09 22:30:15 [ERROR] main.go:42: Failed to connect
2024/02/09 22:30:16 [INFO] handler.go:28: Request processed
2024/02/09 22:30:17 [WARN] db.go:15: Connection slow"
```

### JSON Format (zap)
```bash
curl -X POST http://localhost:8000/api/ingest/go \
  -H "Content-Type: text/plain" \
  -d '{"level":"error","ts":1707502215.123,"caller":"main.go:42","msg":"Failed to connect"}
{"level":"info","ts":1707502216.456,"caller":"handler.go:28","msg":"Request processed"}'
```

## Auto-Detection Example (Recommended)

```bash
curl -X POST http://localhost:8000/api/ingest/auto \
  -H "Content-Type: text/plain" \
  -d "2024-02-09 22:30:15.123 ERROR [main] com.example.App - Connection failed
java.lang.NullPointerException: Cannot invoke method
    at com.example.Service.process(Service.java:42)
    at com.example.App.main(App.java:15)
2024-02-09 22:30:16.456 INFO [worker-1] com.example.Service - Processing request"
```

## Search Logs

```bash
curl -X POST http://localhost:8000/api/search \
  -H "Content-Type: application/json" \
  -d '{
    "app_id": "default",
    "q": "ERROR",
    "limit": 100
  }'
```

## Features

### ✅ Implemented
- **Java Log Parsing**: Log4j/Logback format with multi-line exception stack traces
- **Rust Log Parsing**: env_logger and tracing formats
- **Go Log Parsing**: Standard library and JSON formats (zap, logrus)
- **Multi-line Merging**: Automatic detection and merging of stack traces
- **Format Auto-Detection**: Automatically detect log format from content
- **Structured Fields**: Extract thread, logger, module, caller information
- **Severity Mapping**: Map log levels to syslog severity codes

### 🎯 Supported Log Formats

#### Java
- `2024-02-09 22:30:15.123 ERROR [main] com.example.App - Message`
- `2024-02-09 22:30:15,123 WARN [thread] Logger - Message` (comma separator)
- Multi-line exception stack traces with `at`, `Caused by:`, `... N more`

#### Rust
- `[2024-02-09T14:30:15Z ERROR my_app] Message` (env_logger)
- `2024-02-09T14:30:15Z ERROR module: Message` (tracing)

#### Go
- `2024/02/09 22:30:15 [ERROR] main.go:42: Message` (standard)
- `{"level":"error","ts":1707502215.123,"msg":"Message"}` (zap JSON)
- `time="2024-02-09T22:30:15Z" level=error msg="Message"` (logrus)

### 📊 Extracted Fields

**Java:**
- `thread`: Thread name
- `logger`: Logger class name
- `stacktrace`: Exception stack trace (if present)

**Rust:**
- `module`: Module path
- `stacktrace`: Panic backtrace (if present)

**Go:**
- `caller`: File and line number
- All JSON fields preserved for JSON format logs

### 🔍 API Endpoints

- `POST /api/ingest/java` - Java logs
- `POST /api/ingest/rust` - Rust logs
- `POST /api/ingest/go` - Go logs
- `POST /api/ingest/auto` - Auto-detect format (recommended)
- `POST /api/ingest/nginx` - Nginx access logs (existing)
- `POST /api/ingest` - Generic JSON format (existing)
