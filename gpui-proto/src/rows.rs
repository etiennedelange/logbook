//! What the list shows, as plain data. No GPUI in here, so it is unit-tested on any platform.

use chrono::{DateTime, Local};
use logbook_core::dto::{SearchHitDto, SessionDto, TimelineEntryDto};
use std::collections::HashMap;
use std::ops::Range;

/// One list row: a session, plus the search snippet when the list is showing results.
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub id: String,
    pub agent: &'static str,
    pub title: String,
    pub meta: String,
    pub snippet: Option<Snippet>,
}

/// A search snippet with the matched words as byte ranges into `text`.
#[derive(Debug, Clone, PartialEq)]
pub struct Snippet {
    pub text: String,
    pub hits: Vec<Range<usize>>,
}

const TITLE_MAX: usize = 120;

pub fn title(s: &SessionDto) -> String {
    let raw = s.name.as_deref().or(s.first_prompt.as_deref()).unwrap_or(&s.native_id);
    let one_line = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    truncate(&one_line, TITLE_MAX)
}

fn truncate(s: &str, max: usize) -> String {
    match s.char_indices().nth(max) {
        Some((i, _)) => format!("{}…", &s[..i]),
        None => s.to_string(),
    }
}

/// `2026-11-13 14:05` in local time; the raw string if it doesn't parse.
pub fn when(rfc3339: &str) -> String {
    match DateTime::parse_from_rfc3339(rfc3339) {
        Ok(d) => d.with_timezone(&Local).format("%Y-%m-%d %H:%M").to_string(),
        Err(_) => rfc3339.to_string(),
    }
}

pub fn meta(s: &SessionDto) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(repo) = s.repository.as_deref().filter(|r| !r.is_empty()) {
        parts.push(repo.to_string());
    }
    if let Some(branch) = s.branch.as_deref().filter(|b| !b.is_empty()) {
        parts.push(branch.to_string());
    }
    if let Some(at) = s.updated_at.as_deref().or(s.created_at.as_deref()) {
        parts.push(when(at));
    }
    parts.push(format!("{} msg", s.message_count));
    parts.join(" · ")
}

fn row(s: &SessionDto, snippet: Option<Snippet>) -> Row {
    Row { id: s.id.clone(), agent: s.agent.key(), title: title(s), meta: meta(s), snippet }
}

pub fn recent_rows(sessions: &[SessionDto]) -> Vec<Row> {
    sessions.iter().map(|s| row(s, None)).collect()
}

/// Search hits carry only an id and a snippet, so the session comes from `by_id`. A hit whose
/// session is gone (the index changed under the query) is dropped.
pub fn hit_rows(hits: &[SearchHitDto], by_id: &HashMap<String, SessionDto>) -> Vec<Row> {
    hits.iter()
        .filter_map(|h| {
            let s = by_id.get(&h.id)?;
            Some(row(s, h.snippet.as_deref().map(parse_snippet)))
        })
        .collect()
}

/// The core wraps each matched word in `\u{2}`..`\u{3}`; this strips the markers and records where
/// the words are.
pub fn parse_snippet(raw: &str) -> Snippet {
    let mut text = String::with_capacity(raw.len());
    let mut hits = Vec::new();
    let mut start = None;
    for c in raw.chars() {
        match c {
            '\u{2}' => start = Some(text.len()),
            '\u{3}' => {
                if let Some(s) = start.take() {
                    if text.len() > s {
                        hits.push(s..text.len());
                    }
                }
            }
            // The core joins fragments with newlines; the list shows one line.
            '\n' => text.push_str(" ⋯ "),
            c => text.push(c),
        }
    }
    Snippet { text, hits }
}

/// One transcript message, ready to draw.
#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    pub role: String,
    /// "You", "Assistant", or the raw role for anything else (tool output, system).
    pub label: String,
    pub when: Option<String>,
    pub text: String,
}

/// A message longer than this is cut, so one huge tool output can't stall layout.
pub const MESSAGE_MAX_CHARS: usize = 20_000;

pub fn messages(entries: &[TimelineEntryDto]) -> Vec<Message> {
    entries
        .iter()
        .map(|e| {
            let label = match e.role.as_str() {
                "user" => "You".to_string(),
                "assistant" => "Assistant".to_string(),
                other => other.to_string(),
            };
            let text = match e.text.char_indices().nth(MESSAGE_MAX_CHARS) {
                Some((i, _)) => format!("{}\n\n… (cut: {} more characters)", &e.text[..i], e.text[i..].chars().count()),
                None => e.text.clone(),
            };
            Message { role: e.role.clone(), label, when: e.timestamp.as_deref().map(when), text }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use logbook_core::models::Agent;

    fn session(id: &str) -> SessionDto {
        SessionDto {
            id: format!("copilot:{id}"),
            agent: Agent::Copilot,
            native_id: id.into(),
            name: None,
            user_named: false,
            cwd: None,
            git_root: None,
            repository: Some("me/repo".into()),
            branch: Some("main".into()),
            client_name: None,
            created_at: None,
            updated_at: Some("2026-11-13T12:05:00+00:00".into()),
            message_count: 4,
            first_prompt: Some("fix\nthe   bug".into()),
            path: String::new(),
            source_deleted_at: None,
            files_created: 0,
            files_edited: 0,
        }
    }

    #[test]
    fn title_prefers_name_then_prompt_then_id_on_one_line() {
        let mut s = session("a");
        assert_eq!(title(&s), "fix the bug");
        s.name = Some("Named".into());
        assert_eq!(title(&s), "Named");
        s.name = None;
        s.first_prompt = None;
        assert_eq!(title(&s), "a");
    }

    #[test]
    fn long_titles_truncate_on_a_char_boundary() {
        let mut s = session("a");
        s.name = Some("é".repeat(300));
        let t = title(&s);
        assert_eq!(t.chars().count(), TITLE_MAX + 1);
        assert!(t.ends_with('…'));
    }

    #[test]
    fn meta_skips_missing_parts() {
        let mut s = session("a");
        s.repository = None;
        s.branch = None;
        s.updated_at = None;
        assert_eq!(meta(&s), "4 msg");
        assert!(meta(&session("a")).starts_with("me/repo · main · 2026-11-"));
    }

    #[test]
    fn snippet_markers_become_ranges() {
        let s = parse_snippet("a \u{2}bug\u{3} in \u{2}é\u{3}x");
        assert_eq!(s.text, "a bug in éx");
        assert_eq!(s.hits, vec![2..5, 9..11]);
        assert_eq!(&s.text[s.hits[1].clone()], "é");
    }

    #[test]
    fn snippet_newlines_become_one_line_with_ranges_intact() {
        let s = parse_snippet("a\n\u{2}b\u{3}");
        assert_eq!(s.text, "a ⋯ b");
        assert_eq!(&s.text[s.hits[0].clone()], "b");
    }

    #[test]
    fn hits_for_unknown_sessions_are_dropped() {
        let known = session("a");
        let by_id: HashMap<_, _> = [(known.id.clone(), known)].into();
        let hit = |id: &str| SearchHitDto { id: id.into(), snippet: Some("\u{2}x\u{3}".into()), matched: "text", file: None };
        let rows = hit_rows(&[hit("copilot:a"), hit("copilot:gone")], &by_id);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].snippet.as_ref().unwrap().hits, vec![0..1]);
    }

    #[test]
    fn messages_label_roles_and_cut_huge_text_on_a_char_boundary() {
        let entry = |role: &str, text: String| TimelineEntryDto { role: role.into(), timestamp: Some("2026-11-13T12:05:00+00:00".into()), text };
        let out = messages(&[entry("user", "hi".into()), entry("tool", "é".repeat(MESSAGE_MAX_CHARS + 5))]);
        assert_eq!(out[0].label, "You");
        assert_eq!(out[1].label, "tool");
        assert!(out[0].when.as_deref().unwrap().starts_with("2026-11-"));
        assert!(out[1].text.starts_with(&"é".repeat(MESSAGE_MAX_CHARS)));
        assert!(out[1].text.ends_with("(cut: 5 more characters)"));
    }
}
