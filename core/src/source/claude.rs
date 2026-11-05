//! Claude Code: one JSONL transcript per session at `~/.claude/projects/<project>/<session-id>.jsonl`.
//!
//! The project folder's name is the working path with every non-alphanumeric replaced by `-`, so
//! it can't be turned back into a path: the folder and branch come from the records instead.
//! Next to each transcript there may be a `<session-id>/` folder (`subagents/`, `tool-results/`);
//! only top-level `.jsonl` files are sessions.
//!
//! Claude Code doesn't record the repository, so it's read from the folder's git config, in the
//! form the Copilot CLI records it, so both agents' sessions share one Repositories entry.
//!
//! Anthropic documents the line format as internal and liable to change between versions. So
//! this reader keeps only what it recognises (user and assistant text, titles, folder, branch,
//! times), ignores every other record type, and never fails a session over one bad line.
//! `core/tests/claude.rs` checks it against a small hand-written fixture.
//!
//! The files a session changed come from the results of its file tools (`Write`, `Edit`,
//! `MultiEdit`): a user record's `toolUseResult` with a `filePath` and a `structuredPatch`, whose
//! `type` is `create` for a new file. A failed edit has a plain-text result and isn't counted.

use super::{is_safe_id, kept_text, Fingerprint, Resume, Scan, SessionFiles, Source};
use crate::changes::{self, Edit};
use crate::models::{Agent, FileEdit, Hunk, SessionRecord, TimelineEntry, FIRST_PROMPT_MAX};
use crate::{git, paths};
use anyhow::Result;
use chrono::{DateTime, Utc};
use memchr::memmem;
use serde_json::Value;
use std::io::BufRead;
use std::path::{Path, PathBuf};

pub struct ClaudeCode;

impl Source for ClaudeCode {
    fn agent(&self) -> Agent {
        Agent::ClaudeCode
    }

    fn roots(&self) -> Vec<PathBuf> {
        vec![paths::claude_projects_root()]
    }

    fn discover(&self) -> Result<Vec<SessionFiles>> {
        let mut found = Vec::new();
        for project in std::fs::read_dir(paths::claude_projects_root())? {
            let Ok(project) = project else { continue };
            let Ok(files) = std::fs::read_dir(project.path()) else { continue };
            for file in files.flatten() {
                let path = file.path();
                if path.extension().is_none_or(|e| e != "jsonl") || !path.is_file() {
                    continue;
                }
                let Some(native_id) = path.file_stem().map(|s| s.to_string_lossy().into_owned()) else { continue };
                found.push(SessionFiles { native_id, fingerprint: Fingerprint::of(&[&path]), path });
            }
        }
        Ok(found)
    }

    fn read(&self, files: &SessionFiles) -> Option<Scan> {
        let t = read_transcript(&files.path)?;
        let first_prompt = t.timeline.iter().find(|e| e.role == "user").map(|e| {
            let collapsed = e.text.split_whitespace().collect::<Vec<_>>().join(" ");
            collapsed.chars().take(FIRST_PROMPT_MAX).collect()
        });
        let (name, user_named) = match (t.custom_title, t.ai_title) {
            (Some(custom), _) => (Some(custom), true),
            (None, generated) => (generated, false),
        };
        // Claude Code records only the folder; the repository comes from its git config, if it's still there.
        let (git_root, repository) = match t.cwd.as_deref().and_then(|c| git::repository_of(Path::new(c))) {
            Some((root, remote)) => (Some(root.to_string_lossy().into_owned()), remote),
            None => (None, None),
        };
        let changed = changes::files(&t.edits, t.cwd.as_deref());
        let record = SessionRecord {
            id: Agent::ClaudeCode.session_id(&files.native_id),
            agent: Agent::ClaudeCode,
            native_id: files.native_id.clone(),
            name,
            user_named,
            cwd: t.cwd,
            git_root,
            repository,
            branch: t.branch,
            client_name: None,
            created_at: t.first_at,
            updated_at: t.last_at,
            message_count: t.timeline.len() as i64,
            first_prompt,
            path: files.path.to_string_lossy().into_owned(),
            source_deleted_at: None,
            files: changed,
            commands: t.commands,
        };
        Some(Scan { record, searchable_text: kept_text(&t.timeline), kept: Some(t.timeline) })
    }

    /// The index keeps every Claude Code transcript, so this is only a fallback.
    fn transcript(&self, record: &SessionRecord) -> Vec<TimelineEntry> {
        read_transcript(Path::new(&record.path)).map(|t| t.timeline).unwrap_or_default()
    }

    fn file_edits(&self, record: &SessionRecord, path: &str) -> Vec<FileEdit> {
        let Some(t) = read_transcript(Path::new(&record.path)) else { return Vec::new() };
        t.edits
            .into_iter()
            .filter(|e| changes::relative(&e.path, t.cwd.as_deref()) == path)
            .map(Edit::into_file_edit)
            .collect()
    }

    fn resume(&self, record: &SessionRecord) -> Option<Resume> {
        is_safe_id(&record.native_id).then(|| Resume {
            program: "claude".into(),
            args: vec!["--resume".into(), record.native_id.clone()],
            cwd: record.cwd.clone(),
        })
    }
}

/// What a transcript holds that Logbook uses.
#[derive(Default)]
struct Transcript {
    timeline: Vec<TimelineEntry>,
    /// Set with `/rename`.
    custom_title: Option<String>,
    /// The title Claude Code generates (`ai-title`; `summary` in older versions).
    ai_title: Option<String>,
    cwd: Option<String>,
    branch: Option<String>,
    first_at: Option<DateTime<Utc>>,
    last_at: Option<DateTime<Utc>>,
    /// Successful writes by the file tools, in order.
    edits: Vec<Edit>,
    /// Shell commands that may have changed files (see `changes::may_change_files`).
    commands: Vec<String>,
}

/// Record types this reader uses. Lines without one of these are skipped unparsed: most of a
/// transcript is API request copies, tool results and file snapshots, often hundreds of KB a line.
const WANTED: [&[u8]; 5] = [
    b"\"type\":\"user\"",
    b"\"type\":\"assistant\"",
    b"\"type\":\"ai-title\"",
    b"\"type\":\"custom-title\"",
    b"\"type\":\"summary\"",
];

/// `None` when the file can't be opened.
fn read_transcript(path: &Path) -> Option<Transcript> {
    let file = std::fs::File::open(path).ok()?;
    let mut reader = std::io::BufReader::with_capacity(1 << 20, file);
    let finders: Vec<_> = WANTED.iter().map(memmem::Finder::new).collect();
    let mut t = Transcript::default();
    // Claude Code writes one line per content block, so one reply's text blocks share a message id.
    let mut last_assistant_id: Option<String> = None;
    let mut line = Vec::new();
    loop {
        line.clear();
        match reader.read_until(b'\n', &mut line) {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }
        if !finders.iter().any(|f| f.find(&line).is_some()) {
            continue;
        }
        let Ok(v) = serde_json::from_slice::<Value>(&line) else { continue };
        let str_of = |k: &str| v.get(k).and_then(Value::as_str).filter(|s| !s.trim().is_empty());
        match str_of("type") {
            Some("custom-title") => t.custom_title = str_of("customTitle").map(str::to_owned).or(t.custom_title),
            Some("ai-title") => t.ai_title = str_of("aiTitle").map(str::to_owned).or(t.ai_title),
            Some("summary") if t.ai_title.is_none() => t.ai_title = str_of("summary").map(str::to_owned),
            Some(kind @ ("user" | "assistant")) => {
                let at = str_of("timestamp")
                    .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                    .map(|d| d.with_timezone(&Utc));
                if at.is_some() {
                    t.first_at = t.first_at.or(at);
                    t.last_at = at.max(t.last_at);
                }
                // The session's folder is where it started; its branch is where it is now.
                if t.cwd.is_none() {
                    t.cwd = str_of("cwd").map(str::to_owned);
                }
                if let Some(branch) = str_of("gitBranch") {
                    t.branch = Some(branch.to_owned());
                }
                // A subagent's edits and commands changed files too, so these come before the skip below.
                if kind == "user" {
                    if let Some(edit) = v.get("toolUseResult").and_then(|r| file_edit(r, at)) {
                        t.edits.push(edit);
                    }
                } else {
                    for command in shell_commands(&v) {
                        changes::note_command(&mut t.commands, command);
                    }
                }

                // Subagent turns, and context Claude Code adds itself, aren't the conversation (though
                // they still count for the times, folder and branch above).
                if [v.get("isSidechain"), v.get("isMeta"), v.get("isCompactSummary")]
                    .iter()
                    .any(|f| f.and_then(Value::as_bool) == Some(true))
                {
                    continue;
                }

                let Some(message) = v.get("message") else { continue };
                if kind == "user" {
                    last_assistant_id = None;
                    if let Some(text) = user_text(&v, message) {
                        t.timeline.push(TimelineEntry { role: "user".into(), timestamp: at, text });
                    }
                    continue;
                }
                if message.get("model").and_then(Value::as_str) == Some("<synthetic>") {
                    continue;
                }
                let Some(text) = joined_text(message) else { continue };
                let id = message.get("id").and_then(Value::as_str).map(str::to_owned);
                match t.timeline.last_mut() {
                    Some(prev) if prev.role == "assistant" && id.is_some() && id == last_assistant_id => {
                        prev.text.push_str("\n\n");
                        prev.text.push_str(&text);
                    }
                    _ => t.timeline.push(TimelineEntry { role: "assistant".into(), timestamp: at, text }),
                }
                last_assistant_id = id;
            }
            _ => {}
        }
    }
    Some(t)
}

/// A file tool's successful result as an edit. `None` for any other result: other tools', and
/// failures, whose result is a string.
fn file_edit(result: &Value, at: Option<DateTime<Utc>>) -> Option<Edit> {
    let path = result.get("filePath").and_then(Value::as_str).filter(|p| !p.is_empty())?;
    let patch = result.get("structuredPatch")?.as_array()?;
    let created = result.get("type").and_then(Value::as_str) == Some("create");
    let (mut hunks, mut truncated) = (Vec::new(), false);
    let mut room = changes::MAX_EDIT_LINES;
    for h in patch {
        let num = |k: &str| h.get(k).and_then(Value::as_u64).unwrap_or(0) as u32;
        let all = h.get("lines").and_then(Value::as_array).map(Vec::as_slice).unwrap_or_default();
        let lines: Vec<String> = all.iter().filter_map(Value::as_str).take(room).map(str::to_owned).collect();
        truncated |= lines.len() < all.len();
        room -= lines.len();
        if !lines.is_empty() {
            hunks.push(Hunk { old_start: num("oldStart"), new_start: num("newStart"), lines });
        }
    }
    // A new file's patch is empty: its content is the diff.
    if created && hunks.is_empty() {
        let content = result.get("content").and_then(Value::as_str).unwrap_or_default();
        (hunks, truncated) = changes::created_hunk(content);
    }
    let (added, removed) = changes::counts(&hunks);
    Some(Edit { path: path.to_owned(), created, timestamp: at, hunks, truncated, added, removed })
}

/// The commands of an assistant record's shell tool calls (`Bash`, or `PowerShell` on Windows).
fn shell_commands(record: &Value) -> impl Iterator<Item = &str> {
    let blocks = record.get("message").and_then(|m| m.get("content")).and_then(Value::as_array);
    blocks.into_iter().flatten().filter_map(|b| {
        let name = b.get("name").and_then(Value::as_str)?;
        if b.get("type").and_then(Value::as_str) != Some("tool_use") || !matches!(name, "Bash" | "PowerShell") {
            return None;
        }
        b.get("input")?.get("command")?.as_str()
    })
}

/// What the person typed, or `None` for a user record that isn't a prompt: tool results,
/// notifications, shell escapes and the like.
fn user_text(record: &Value, message: &Value) -> Option<String> {
    // Newer versions say who wrote the turn; anything but a human (a task notification, another
    // session's message) isn't a prompt.
    if let Some(kind) = record.get("origin").and_then(|o| o.get("kind")).and_then(Value::as_str) {
        if kind != "human" {
            return None;
        }
    }
    let text = joined_text(message)?;
    let trimmed = text.trim_start();
    // A slash command is a prompt when it has arguments (`/review the parser`); a bare `/clear` isn't.
    if trimmed.starts_with("<command-") {
        let name = between(trimmed, "<command-name>", "</command-name>")?;
        let args = between(trimmed, "<command-args>", "</command-args>").map(str::trim).filter(|a| !a.is_empty())?;
        return Some(format!("{} {args}", name.trim()));
    }
    const NOT_PROMPTS: [&str; 6] =
        ["<local-command-", "<bash-", "<task-notification>", "<system-reminder>", "[Request interrupted", "Caveat: "];
    if NOT_PROMPTS.iter().any(|p| trimmed.starts_with(p)) {
        return None;
    }
    Some(text)
}

/// A message's text: its content when that's a string, else its `text` blocks joined. Thinking,
/// tool calls, tool results and images are left out. `None` when there's no text.
fn joined_text(message: &Value) -> Option<String> {
    let text = match message.get("content")? {
        Value::String(s) => s.clone(),
        Value::Array(blocks) => blocks
            .iter()
            .filter(|b| b.get("type").and_then(Value::as_str) == Some("text"))
            .filter_map(|b| b.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join("\n\n"),
        _ => return None,
    };
    (!text.trim().is_empty()).then_some(text)
}

fn between<'a>(s: &'a str, open: &str, close: &str) -> Option<&'a str> {
    let start = s.find(open)? + open.len();
    let end = s[start..].find(close)? + start;
    Some(&s[start..end])
}
