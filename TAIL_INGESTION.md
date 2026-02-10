# Tail Ingestion Guide

## Overview

Loglite supports automatic log file monitoring and ingestion through the tail ingestion feature. This allows you to configure log sources that are continuously monitored for new content.

## Features

- ✅ **File and Directory Monitoring** - Monitor single files or entire directories
- ✅ **Recursive Scanning** - Optionally scan subdirectories
- ✅ **Glob Pattern Filtering** - Include/exclude files using glob patterns
- ✅ **Offset Persistence** - Resume from last read position after restart
- ✅ **Format Auto-Detection** - Automatically detect log format (Java/Rust/Go/Nginx)
- ✅ **Multi-Line Support** - Handle exception stack traces and multi-line logs
- ✅ **Background Processing** - Non-blocking ingestion in background task

## Configuration

### Environment Variables

```bash
# Tail ingestion interval (seconds)
LOGLITE_TAIL_INTERVAL_SECS=10  # Default: 10 seconds
```

## API Endpoints

### Create a Log Source

```bash
POST /api/sources
```

**Request Body:**
```json
{
  "app_id": "my-app-a1b2c3d4",
  "kind": "tail",
  "path": "/var/log/myapp",
  "recursive": true,
  "encoding": "utf-8",
  "include_glob": "*.log",
  "exclude_glob": "*.gz",
  "enabled": true
}
```

**Fields:**
- `app_id` (required): Application ID to associate logs with
- `kind` (required): Source type, use `"tail"` for file monitoring
- `path` (required): File or directory path to monitor
- `recursive` (optional): Scan subdirectories, default: `false`
- `encoding` (optional): File encoding, default: `"utf-8"`
- `include_glob` (optional): Glob pattern for files to include
- `exclude_glob` (optional): Glob pattern for files to exclude
- `enabled` (optional): Enable/disable source, default: `true`

**Response:**
```json
{
  "id": 1,
  "app_id": "my-app-a1b2c3d4",
  "kind": "tail",
  "path": "/var/log/myapp",
  "recursive": true,
  "encoding": "utf-8",
  "include_glob": "*.log",
  "exclude_glob": "*.gz",
  "enabled": true,
  "created_at": "2024-02-09T14:30:15Z"
}
```

### List All Sources

```bash
GET /api/sources?app_id=my-app-a1b2c3d4
```

**Query Parameters:**
- `app_id` (optional): Filter by application ID

**Response:**
```json
[
  {
    "id": 1,
    "app_id": "my-app-a1b2c3d4",
    "kind": "tail",
    "path": "/var/log/myapp",
    "recursive": true,
    "encoding": "utf-8",
    "include_glob": "*.log",
    "exclude_glob": "*.gz",
    "enabled": true,
    "created_at": "2024-02-09T14:30:15Z"
  }
]
```

### Get a Single Source

```bash
GET /api/sources/1
```

### Update a Source

```bash
PUT /api/sources/1
```

**Request Body (all fields optional):**
```json
{
  "path": "/var/log/myapp/new-path",
  "recursive": false,
  "encoding": "utf-8",
  "include_glob": "*.log",
  "exclude_glob": "*.old",
  "enabled": false
}
```

### Delete a Source

```bash
DELETE /api/sources/1
```

## Usage Examples

### Example 1: Monitor a Single Log File

```bash
curl -X POST http://localhost:8000/api/sources \
  -H "Content-Type: application/json" \
  -d '{
    "app_id": "my-app-a1b2c3d4",
    "kind": "tail",
    "path": "/var/log/myapp/application.log",
    "enabled": true
  }'
```

### Example 2: Monitor a Directory with Glob Patterns

```bash
curl -X POST http://localhost:8000/api/sources \
  -H "Content-Type: application/json" \
  -d '{
    "app_id": "my-app-a1b2c3d4",
    "kind": "tail",
    "path": "/var/log/myapp",
    "recursive": true,
    "include_glob": "*.log",
    "exclude_glob": "*.{gz,zip,old}",
    "enabled": true
  }'
```

### Example 3: Monitor Java Application Logs

```bash
curl -X POST http://localhost:8000/api/sources \
  -H "Content-Type: application/json" \
  -d '{
    "app_id": "java-service-a1b2c3d4",
    "kind": "tail",
    "path": "/opt/tomcat/logs",
    "recursive": false,
    "include_glob": "catalina.*.log",
    "enabled": true
  }'
```

### Example 4: Disable a Source Temporarily

```bash
curl -X PUT http://localhost:8000/api/sources/1 \
  -H "Content-Type: application/json" \
  -d '{
    "enabled": false
  }'
```

### Example 5: List All Sources for an App

```bash
curl http://localhost:8000/api/sources?app_id=my-app-a1b2c3d4
```

## How It Works

### 1. Source Registration

When you create a source via the API, it's stored in the `app_sources` table with configuration details.

### 2. Background Monitoring

A background task runs every `LOGLITE_TAIL_INTERVAL_SECS` seconds (default: 10s) and:
- Loads all enabled sources from the database
- Scans configured paths for files matching glob patterns
- Reads new content from each file since last offset

### 3. Offset Tracking

For each file, the system maintains an offset in the `tail_offsets` table:
- Tracks the last byte position read
- Resumes from this position on next scan
- Handles file truncation (resets to 0 if file shrinks)

### 4. Log Parsing

New content is automatically:
- Detected for format (Java/Rust/Go/Nginx)
- Parsed with multi-line merging if needed
- Ingested into database and search index

### 5. Structured Fields

Parsed logs include structured fields:
- **Java**: `thread`, `logger`, `stacktrace`
- **Rust**: `module`
- **Go**: `caller`, all JSON fields
- **All**: `timestamp`, `level`, `message`

## Glob Pattern Examples

### Include Patterns

```
*.log           # All .log files
app-*.log       # Files starting with "app-"
**/*.log        # All .log files recursively
access.log.*    # Rotated access logs
```

### Exclude Patterns

```
*.gz            # Compressed files
*.old           # Old files
*.{gz,zip,bz2}  # Multiple extensions
**/archive/*    # Files in archive directories
```

## Best Practices

### 1. Use Specific Paths

```json
{
  "path": "/var/log/myapp",
  "include_glob": "application-*.log"
}
```

Instead of:
```json
{
  "path": "/var/log",
  "recursive": true
}
```

### 2. Exclude Unnecessary Files

```json
{
  "exclude_glob": "*.{gz,zip,old,bak}"
}
```

### 3. Monitor Application-Specific Directories

Create separate sources for different applications:

```bash
# Web server logs
POST /api/sources {"app_id": "web", "path": "/var/log/nginx"}

# Application logs
POST /api/sources {"app_id": "backend", "path": "/opt/app/logs"}

# Database logs
POST /api/sources {"app_id": "db", "path": "/var/log/postgresql"}
```

### 4. Adjust Scan Interval

For high-volume logs:
```bash
LOGLITE_TAIL_INTERVAL_SECS=5  # Scan every 5 seconds
```

For low-volume logs:
```bash
LOGLITE_TAIL_INTERVAL_SECS=30  # Scan every 30 seconds
```

### 5. Use Encoding Correctly

For non-UTF-8 files:
```json
{
  "encoding": "latin1"
}
```

## Troubleshooting

### Logs Not Appearing

1. **Check source is enabled:**
   ```bash
   curl http://localhost:8000/api/sources/1
   ```

2. **Verify file permissions:**
   - Ensure Loglite process can read the files
   - Check directory permissions

3. **Check glob patterns:**
   - Test patterns match your files
   - Use `include_glob` to be specific

4. **Review logs:**
   ```bash
   RUST_LOG=debug cargo run
   ```

### Offset Issues

If logs are being re-ingested:
- Check `tail_offsets` table for correct offsets
- File might have been truncated or rotated

### Performance Issues

If tail ingestion is slow:
- Reduce number of monitored files
- Use more specific glob patterns
- Increase `LOGLITE_TAIL_INTERVAL_SECS`
- Consider monitoring specific files instead of directories

## Database Schema

### app_sources Table

```sql
CREATE TABLE app_sources (
    id BIGSERIAL PRIMARY KEY,
    app_id VARCHAR NOT NULL,
    kind VARCHAR NOT NULL,
    path VARCHAR NOT NULL,
    recursive BOOLEAN NOT NULL,
    encoding VARCHAR NOT NULL,
    include_glob VARCHAR,
    exclude_glob VARCHAR,
    enabled BOOLEAN NOT NULL,
    created_at TIMESTAMPTZ NOT NULL
);
```

### tail_offsets Table

```sql
CREATE TABLE tail_offsets (
    id BIGSERIAL PRIMARY KEY,
    source_id BIGINT NOT NULL,
    file_path VARCHAR NOT NULL,
    offset_bytes BIGINT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    UNIQUE(source_id, file_path)
);
```

## Advanced Configuration

### Multiple Sources for Same Directory

You can create multiple sources with different patterns:

```bash
# Monitor error logs
curl -X POST http://localhost:8000/api/sources \
  -H "Content-Type: application/json" \
  -d '{
    "app_id": "my-app",
    "kind": "tail",
    "path": "/var/log/myapp",
    "include_glob": "*error*.log"
  }'

# Monitor access logs
curl -X POST http://localhost:8000/api/sources \
  -H "Content-Type: application/json" \
  -d '{
    "app_id": "my-app",
    "kind": "tail",
    "path": "/var/log/myapp",
    "include_glob": "*access*.log"
  }'
```

### Conditional Monitoring

Enable/disable sources based on environment:

```bash
# Development: monitor all logs
PUT /api/sources/1 {"enabled": true}

# Production: monitor only errors
PUT /api/sources/1 {"enabled": false}
PUT /api/sources/2 {"enabled": true}
```

## Next Steps

- Configure sources for your applications
- Monitor ingestion in the logs
- Search ingested logs via `/api/search`
- Set up retention policies with `LOGLITE_RETENTION_DAYS`
- Build frontend UI for source management

## Related Documentation

- [QUICKSTART.md](QUICKSTART.md) - Getting started guide
- [README.md](README.md) - Full documentation
- [FEATURES.md](FEATURES.md) - Complete feature list
- [test_logs.md](test_logs.md) - API usage examples
