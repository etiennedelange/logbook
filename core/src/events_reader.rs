use crate::models::{TimelineEntry, FIRST_PROMPT_MAX};
use chrono::{DateTime, Utc};
use memchr::memmem;
use serde_json::Value;
use std::io::BufRead;
use std::path::Path;

/// Result of scanning a session's events.jsonl file.
pub struct EventsScanResult {
    pub searchable_text: String,
    pub message_count: i64,
    pub first_prompt: Option<String>,
    /// Empty for an index-only scan (`scan_for_index`).
    pub timeline: Vec<TimelineEntry>,
}

/// Full scan, including the conversation timeline for the detail view.
pub fn scan(events_path: &Path) -> EventsScanResult {
    scan_inner(events_path, true)
}

/// Scan for indexing: same result, but without building the timeline, which would copy every
/// message a second time only to be thrown away.
pub fn scan_for_index(events_path: &Path) -> EventsScanResult {
    scan_inner(events_path, false)
}

/// Streams a session's `events.jsonl` (newline-delimited JSON) and extracts the text
/// used for full-text search plus a lightweight conversation timeline for the detail view.
fn scan_inner(events_path: &Path, want_timeline: bool) -> EventsScanResult {
    let mut text_parts: Vec<String> = Vec::new();
    let mut timeline: Vec<TimelineEntry> = Vec::new();
    let mut message_count: i64 = 0;

    let empty = || EventsScanResult {
        searchable_text: String::new(),
        message_count: 0,
        first_prompt: None,
        timeline: Vec::new(),
    };
    // Stream line by line into one reused buffer: these files run to tens of MB, and with
    // several scanned in parallel, reading each whole would spike memory.
    let Ok(file) = std::fs::File::open(events_path) else {
        return empty();
    };
    let mut reader = std::io::BufReader::with_capacity(1 << 20, file);
    let mut line_buf: Vec<u8> = Vec::new();
    let mut first_prompt_text: Option<String> = None;

    let user = memmem::Finder::new(b"\"user.message\"");
    let assistant = memmem::Finder::new(b"\"assistant.message\"");
    loop {
        line_buf.clear();
        match reader.read_until(b'\n', &mut line_buf) {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }
        let line = line_buf.as_slice();
        // Most lines are tool calls, model turns and the like, often many KB each; a byte search
        // skips them without decoding or parsing them. Lines that pass are still checked below.
        if user.find(line).is_none() && assistant.find(line).is_none() {
            continue;
        }
        let Ok(line) = std::str::from_utf8(line) else { continue };

        let Ok(value) = serde_json::from_str::<Value>(line) else {
            continue; // Skip malformed/truncated lines (e.g. a crash mid-write).
        };

        let Some(event_type) = value.get("type").and_then(Value::as_str) else {
            continue;
        };

        if event_type != "user.message" && event_type != "assistant.message" {
            continue;
        }

        message_count += 1;

        let Some(data) = value.get("data") else { continue };
        let content = data.get("content").and_then(Value::as_str);
        let Some(content) = content else { continue };
        if content.trim().is_empty() {
            continue;
        }

        text_parts.push(content.to_string());

        if event_type == "user.message" && first_prompt_text.is_none() {
            first_prompt_text = Some(content.to_string());
        }
        if !want_timeline {
            continue;
        }

        let timestamp: Option<DateTime<Utc>> = value
            .get("timestamp")
            .and_then(Value::as_str)
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|d| d.with_timezone(&Utc));

        let role = if event_type == "user.message" { "user" } else { "assistant" };
        timeline.push(TimelineEntry { role: role.to_string(), timestamp, text: content.to_string() });
    }

    let first_prompt = first_prompt_text.map(|text| {
        let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
        collapsed.chars().take(FIRST_PROMPT_MAX).collect()
    });

    EventsScanResult { searchable_text: text_parts.join("\n"), message_count, first_prompt, timeline }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_scan_matches_full_scan_apart_from_the_timeline() {
        let path = std::env::temp_dir().join(format!("logbook-events-{}.jsonl", std::process::id()));
        let lines = [
            r#"{"type":"session.start"}"#,
            r#"{"type":"user.message","timestamp":"2026-01-02T00:00:00Z","data":{"content":"  first   ask \n here "}}"#,
            r#"not json but mentions "user.message""#,
            r#"{"type":"assistant.message","data":{"content":"an answer"}}"#,
            r#"{"type":"user.message","data":{"content":"   "}}"#,
            r#"{"type":"user.message","data":{"content":"second"}}"#,
        ];
        std::fs::write(&path, lines.join("\n")).unwrap();

        let full = scan(&path);
        let light = scan_for_index(&path);
        std::fs::remove_file(&path).ok();

        assert_eq!(full.message_count, 4);
        assert_eq!(full.timeline.len(), 3);
        assert!(light.timeline.is_empty());
        assert_eq!(light.message_count, full.message_count);
        assert_eq!(light.searchable_text, full.searchable_text);
        assert_eq!(light.first_prompt, full.first_prompt);
        assert_eq!(full.first_prompt.as_deref(), Some("first ask here"));
    }
}
