use chrono::{DateTime, FixedOffset, NaiveDateTime, Utc};
use lazy_static::lazy_static;
use regex::Regex;
use sea_orm::prelude::DateTimeWithTimeZone;
use sha2::{Digest, Sha256};

/// Generate a stable application id from a display name.
///
/// The generated `app_id` is deterministic and human-friendly: `<slug>-<hash8>`.
pub fn generate_app_id(name: &str) -> String {
    let slug = slugify(name);
    let mut hasher = Sha256::new();
    hasher.update(name.as_bytes());
    let digest = hasher.finalize();
    let hash8 = hex::encode(&digest[..4]);
    if slug.is_empty() {
        format!("app-{}", hash8)
    } else {
        format!("{}-{}", slug, hash8)
    }
}

/// Convert an application name into a URL-safe slug.
fn slugify(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut last_dash = false;
    for ch in s.chars() {
        let ch = ch.to_ascii_lowercase();
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    out.trim_matches('-').to_string()
}

/// Parse a single nginx access log line.
///
/// Returns (message, fields_json) if successful.
pub fn parse_nginx_access_line(line: &str) -> Option<(String, serde_json::Value)> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }

    let mut parts = line.splitn(2, ' ');
    let remote_addr = parts.next()?.to_string();

    Some((
        line.to_string(),
        serde_json::json!({
            "remote_addr": remote_addr
        }),
    ))
}

/// Log format types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    Java,
    Rust,
    Go,
    Nginx,
    Unknown,
}

/// Parsed log entry with structured fields
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: DateTimeWithTimeZone,
    pub level: String,
    pub message: String,
    pub stacktrace: Option<String>,
    pub fields: serde_json::Value,
}

lazy_static! {
    // Java log patterns: "2024-02-09 22:30:15.123 ERROR [main] com.example.App - Message"
    static ref JAVA_LOG_RE: Regex = Regex::new(
        r"^(\d{4}-\d{2}-\d{2}\s+\d{2}:\d{2}:\d{2}[.,]\d{3})\s+(ERROR|WARN|INFO|DEBUG|TRACE)\s+\[([^\]]+)\]\s+([^\s]+)\s+-\s+(.+)$"
    ).unwrap();

    // Rust env_logger: "[2024-02-09T14:30:15Z ERROR my_app] Message"
    static ref RUST_LOG_RE: Regex = Regex::new(
        r"^\[([^\s]+)\s+(ERROR|WARN|INFO|DEBUG|TRACE)\s+([^\]]+)\]\s+(.+)$"
    ).unwrap();

    // Go standard log: "2024/02/09 22:30:15 [ERROR] main.go:42: Message"
    static ref GO_LOG_RE: Regex = Regex::new(
        r"^(\d{4}/\d{2}/\d{2}\s+\d{2}:\d{2}:\d{2})\s+\[?(ERROR|WARN|INFO|DEBUG|TRACE)?\]?\s*([^:]+)?:?\s*(.+)$"
    ).unwrap();

    // Nginx access log: IP address at start
    static ref NGINX_LOG_RE: Regex = Regex::new(
        r"^\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}"
    ).unwrap();

    // Java exception stack trace patterns
    static ref JAVA_STACKTRACE_RE: Regex = Regex::new(
        r"^\s+(at |Caused by:|\.\.\. \d+ more)"
    ).unwrap();
}

/// Detect log format from sample lines
pub fn detect_log_format(lines: &[&str]) -> LogFormat {
    if lines.is_empty() {
        return LogFormat::Unknown;
    }

    let sample_size = lines.len().min(10);
    let mut java_score = 0;
    let mut rust_score = 0;
    let mut go_score = 0;
    let mut nginx_score = 0;

    for line in lines.iter().take(sample_size) {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if JAVA_LOG_RE.is_match(line) {
            java_score += 1;
        }
        if RUST_LOG_RE.is_match(line) {
            rust_score += 1;
        }
        if GO_LOG_RE.is_match(line) {
            go_score += 1;
        }
        if NGINX_LOG_RE.is_match(line) {
            nginx_score += 1;
        }
    }

    let threshold = (sample_size as f32 * 0.6) as usize;

    if java_score >= threshold {
        LogFormat::Java
    } else if rust_score >= threshold {
        LogFormat::Rust
    } else if go_score >= threshold {
        LogFormat::Go
    } else if nginx_score >= threshold {
        LogFormat::Nginx
    } else {
        LogFormat::Unknown
    }
}

/// Map log level string to syslog severity
pub fn level_to_severity(level: &str) -> Option<i32> {
    match level.to_uppercase().as_str() {
        "FATAL" | "ERROR" => Some(3),
        "WARN" | "WARNING" => Some(4),
        "INFO" => Some(6),
        "DEBUG" | "TRACE" => Some(7),
        _ => None,
    }
}

/// Parse timestamp with multiple format attempts
fn parse_timestamp(ts_str: &str) -> Option<DateTimeWithTimeZone> {
    // Try ISO8601 format first
    if let Ok(dt) = DateTime::parse_from_rfc3339(ts_str) {
        return Some(dt.with_timezone(&FixedOffset::east_opt(0).unwrap()));
    }

    // Try "yyyy-MM-dd HH:mm:ss.SSS" format
    if let Ok(naive) = NaiveDateTime::parse_from_str(ts_str, "%Y-%m-%d %H:%M:%S%.3f") {
        let utc = DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc);
        return Some(utc.with_timezone(&FixedOffset::east_opt(0).unwrap()));
    }

    // Try "yyyy-MM-dd HH:mm:ss,SSS" format (Log4j uses comma)
    let ts_normalized = ts_str.replace(',', ".");
    if let Ok(naive) = NaiveDateTime::parse_from_str(&ts_normalized, "%Y-%m-%d %H:%M:%S%.3f") {
        let utc = DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc);
        return Some(utc.with_timezone(&FixedOffset::east_opt(0).unwrap()));
    }

    // Try "yyyy/MM/dd HH:mm:ss" format (Go standard)
    if let Ok(naive) = NaiveDateTime::parse_from_str(ts_str, "%Y/%m/%d %H:%M:%S") {
        let utc = DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc);
        return Some(utc.with_timezone(&FixedOffset::east_opt(0).unwrap()));
    }

    None
}

/// Parse Java application log line
pub fn parse_java_log_line(line: &str) -> Option<LogEntry> {
    let caps = JAVA_LOG_RE.captures(line)?;

    let timestamp = parse_timestamp(caps.get(1)?.as_str())?;
    let level = caps.get(2)?.as_str().to_string();
    let thread = caps.get(3)?.as_str().to_string();
    let logger = caps.get(4)?.as_str().to_string();
    let message = caps.get(5)?.as_str().to_string();

    Some(LogEntry {
        timestamp,
        level: level.clone(),
        message,
        stacktrace: None,
        fields: serde_json::json!({
            "thread": thread,
            "logger": logger,
        }),
    })
}

/// Parse Rust application log line
pub fn parse_rust_log_line(line: &str) -> Option<LogEntry> {
    let caps = RUST_LOG_RE.captures(line)?;

    let timestamp = parse_timestamp(caps.get(1)?.as_str())?;
    let level = caps.get(2)?.as_str().to_string();
    let module = caps.get(3)?.as_str().to_string();
    let message = caps.get(4)?.as_str().to_string();

    Some(LogEntry {
        timestamp,
        level: level.clone(),
        message,
        stacktrace: None,
        fields: serde_json::json!({
            "module": module,
        }),
    })
}

/// Parse Go application log line
pub fn parse_go_log_line(line: &str) -> Option<LogEntry> {
    // Try JSON format first (zap, logrus JSON)
    if line.trim_start().starts_with('{') {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
            let level = json.get("level")
                .or_else(|| json.get("Level"))
                .and_then(|v| v.as_str())
                .unwrap_or("INFO")
                .to_uppercase();

            let message = json.get("msg")
                .or_else(|| json.get("message"))
                .or_else(|| json.get("Message"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let ts_value = json.get("ts")
                .or_else(|| json.get("time"))
                .or_else(|| json.get("timestamp"));

            let timestamp = if let Some(ts) = ts_value {
                if let Some(ts_str) = ts.as_str() {
                    parse_timestamp(ts_str)
                } else if let Some(ts_f64) = ts.as_f64() {
                    let secs = ts_f64 as i64;
                    let nsecs = ((ts_f64 - secs as f64) * 1_000_000_000.0) as u32;
                    let utc = DateTime::from_timestamp(secs, nsecs)?;
                    Some(utc.with_timezone(&FixedOffset::east_opt(0).unwrap()))
                } else {
                    None
                }
            } else {
                None
            };

            if let Some(ts) = timestamp {
                return Some(LogEntry {
                    timestamp: ts,
                    level,
                    message,
                    stacktrace: None,
                    fields: json,
                });
            }
        }
    }

    // Try standard Go log format
    let caps = GO_LOG_RE.captures(line)?;
    let timestamp = parse_timestamp(caps.get(1)?.as_str())?;
    let level = caps.get(2)
        .map(|m| m.as_str().to_string())
        .unwrap_or_else(|| "INFO".to_string());
    let caller = caps.get(3).map(|m| m.as_str().to_string());
    let message = caps.get(4)?.as_str().to_string();

    let mut fields = serde_json::json!({});
    if let Some(c) = caller {
        fields["caller"] = serde_json::Value::String(c);
    }

    Some(LogEntry {
        timestamp,
        level,
        message,
        stacktrace: None,
        fields,
    })
}

/// Merge multi-line logs (e.g., Java stack traces)
pub fn merge_multiline_logs(lines: Vec<&str>, format: LogFormat) -> Vec<LogEntry> {
    let mut entries = Vec::new();
    let mut current_entry: Option<LogEntry> = None;
    let mut stacktrace_lines: Vec<String> = Vec::new();

    for line in lines {
        let line = line.trim_end();
        if line.is_empty() {
            continue;
        }

        // Check if this is a continuation line (stack trace)
        let is_continuation = match format {
            LogFormat::Java => JAVA_STACKTRACE_RE.is_match(line),
            _ => false,
        };

        if is_continuation {
            // Append to stack trace
            stacktrace_lines.push(line.to_string());
        } else {
            // This is a new log entry, save the previous one
            if let Some(mut entry) = current_entry.take() {
                if !stacktrace_lines.is_empty() {
                    entry.stacktrace = Some(stacktrace_lines.join("\n"));
                    entry.fields["stacktrace"] = serde_json::Value::String(stacktrace_lines.join("\n"));
                    stacktrace_lines.clear();
                }
                entries.push(entry);
            }

            // Try to parse the new entry
            let parsed = match format {
                LogFormat::Java => parse_java_log_line(line),
                LogFormat::Rust => parse_rust_log_line(line),
                LogFormat::Go => parse_go_log_line(line),
                _ => None,
            };

            if let Some(entry) = parsed {
                current_entry = Some(entry);
            } else {
                // If parsing fails, append to previous entry's message
                if let Some(ref mut entry) = current_entry {
                    entry.message.push('\n');
                    entry.message.push_str(line);
                } else {
                    // Create a fallback entry with current timestamp
                    let utc = Utc::now();
                    current_entry = Some(LogEntry {
                        timestamp: utc.with_timezone(&FixedOffset::east_opt(0).unwrap()),
                        level: "INFO".to_string(),
                        message: line.to_string(),
                        stacktrace: None,
                        fields: serde_json::json!({}),
                    });
                }
            }
        }
    }

    // Don't forget the last entry
    if let Some(mut entry) = current_entry {
        if !stacktrace_lines.is_empty() {
            entry.stacktrace = Some(stacktrace_lines.join("\n"));
            entry.fields["stacktrace"] = serde_json::Value::String(stacktrace_lines.join("\n"));
        }
        entries.push(entry);
    }

    entries
}
