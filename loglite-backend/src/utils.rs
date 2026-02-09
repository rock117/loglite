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
