use crate::models::SessionRecord;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::path::Path;

/// Reads a session's `workspace.yaml` metadata file. Returns `None` if the file is
/// missing, so a single corrupt session doesn't abort the whole scan.
///
/// The file is a flat, unquoted `key: value` list (one entry per line, no nesting,
/// no lists) written by the Copilot CLI itself, e.g.:
/// ```yaml
/// id: 0084beea-1061-44e5-8c66-bee2addbff2c
/// cwd: D:\Repos\VisionAI\VisionAI.Backend
/// created_at: 2026-06-19T10:09:42.137Z
/// ```
/// so a tiny hand-rolled parser is enough — no general-purpose YAML engine needed.
/// Splitting on the *first* `": "` keeps this safe even though values like Windows
/// paths (`cwd: C:\Users\...`) contain further colons of their own.
pub fn try_read(session_dir: &Path) -> Option<SessionRecord> {
    let yaml_path = session_dir.join("workspace.yaml");
    let contents = std::fs::read_to_string(&yaml_path).ok()?;

    let mut map: HashMap<String, String> = HashMap::new();
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once(": ") {
            map.insert(key.trim().to_string(), value.trim().to_string());
        } else if let Some(key) = line.strip_suffix(':') {
            map.insert(key.trim().to_string(), String::new());
        }
    }

    let get = |k: &str| map.get(k).filter(|v| !v.is_empty()).cloned();

    // The folder name is the session's identity: the indexer matches sessions to folders by it,
    // so trusting a `workspace.yaml` whose `id` disagrees would make the session vanish from the
    // index and get re-read on every scan. The yaml `id` is only a fallback.
    let id = session_dir
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .filter(|n| !n.is_empty())
        .or_else(|| get("id"))?;

    Some(SessionRecord {
        id,
        name: get("name"),
        user_named: map.get("user_named").map(|v| v == "true").unwrap_or(false),
        cwd: get("cwd"),
        git_root: get("git_root"),
        repository: get("repository"),
        branch: get("branch"),
        client_name: get("client_name"),
        created_at: parse_date(map.get("created_at").map(String::as_str)),
        updated_at: parse_date(map.get("updated_at").map(String::as_str)),
        message_count: 0,
        first_prompt: None,
        path: session_dir.to_string_lossy().into_owned(),
    })
}

fn parse_date(value: Option<&str>) -> Option<DateTime<Utc>> {
    let value = value?;
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|d| d.with_timezone(&Utc))
}
