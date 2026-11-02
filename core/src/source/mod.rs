//! Where sessions come from. Each coding agent Logbook reads is one `Source`: it finds the
//! agent's sessions on disk, reads them, and says how to resume one. The indexer and service
//! only ever go through this trait, so adding an agent is one new module here.

pub mod claude;
pub mod copilot;

use crate::models::{Agent, SessionRecord, TimelineEntry};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub trait Source: Send + Sync {
    fn agent(&self) -> Agent;

    /// The folders this source reads, as About shows them. When none of them exists the
    /// agent isn't installed (or has no history yet) and its cached sessions are left alone.
    fn roots(&self) -> Vec<PathBuf>;

    /// One entry per session found, cheaply: only file metadata, no reading. The indexer
    /// re-reads a session only when its fingerprint has changed.
    fn discover(&self) -> Result<Vec<SessionFiles>>;

    /// Reads one session for the index. `None` when it can't be read at all, which drops it
    /// from the index. The indexer sets the record's `id`, `agent` and `native_id`.
    fn read(&self, files: &SessionFiles) -> Option<Scan>;

    /// The conversation, for the detail pane.
    fn transcript(&self, record: &SessionRecord) -> Vec<TimelineEntry>;

    /// The command that resumes the session in the agent's own CLI. `None` when the session's
    /// id isn't safe to put on a command line.
    fn resume(&self, record: &SessionRecord) -> Option<Resume>;
}

/// Every source Logbook reads.
pub fn all() -> Vec<Box<dyn Source>> {
    vec![Box::new(copilot::Copilot), Box::new(claude::ClaudeCode)]
}

/// A session as `Source::discover` found it.
#[derive(Debug, Clone)]
pub struct SessionFiles {
    /// The agent's own id for the session.
    pub native_id: String,
    /// The session's folder or file, as `SessionRecord::path` holds it.
    pub path: PathBuf,
    pub fingerprint: Fingerprint,
}

/// What `Source::read` returns: the session's record, with its message count and first prompt
/// filled in, and the text that search matches against.
pub struct Scan {
    pub record: SessionRecord,
    pub searchable_text: String,
    /// The transcript, from a source whose agent deletes its own files (Claude Code, after
    /// `cleanupPeriodDays`). Logbook keeps it, so the session stays readable and searchable after
    /// its file is gone. `searchable_text` must then be its texts joined with `\n` (see `kept_text`).
    pub kept: Option<Vec<TimelineEntry>>,
}

/// The searchable text of a kept transcript. The index stores only the transcript and rebuilds
/// the text from it, so a kept session costs its text once in the cache, not twice.
pub fn kept_text(timeline: &[TimelineEntry]) -> String {
    timeline.iter().map(|e| e.text.as_str()).collect::<Vec<_>>().join("\n")
}

/// A command line, as a program and its arguments: never one shell string, so nothing in a
/// session's metadata can be read as shell syntax.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resume {
    pub program: String,
    pub args: Vec<String>,
    /// The session's working folder, if it recorded one. It may no longer exist.
    pub cwd: Option<String>,
}

impl Resume {
    /// The command as a user would type it, for Copy.
    pub fn command_line(&self) -> String {
        std::iter::once(self.program.as_str())
            .chain(self.args.iter().map(String::as_str))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// What a session's files looked like when it was last read: each file's path, size and
/// modified time. A file that doesn't exist is left out, so its appearing counts as a change.
#[derive(Serialize, Deserialize, Clone, Default, PartialEq, Eq, Debug)]
pub struct Fingerprint(pub Vec<FileStamp>);

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct FileStamp {
    pub path: String,
    pub size: u64,
    pub mtime_ms: u128,
}

impl Fingerprint {
    /// Stamps each of `files` that exists.
    pub fn of(files: &[&Path]) -> Self {
        Self(
            files
                .iter()
                .filter_map(|p| {
                    let meta = std::fs::metadata(p).ok()?;
                    Some(FileStamp { path: p.to_string_lossy().into_owned(), size: meta.len(), mtime_ms: mtime_ms(&meta) })
                })
                .collect(),
        )
    }
}

fn mtime_ms(meta: &std::fs::Metadata) -> u128 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_millis())
}

/// Native session ids are UUIDs or similar; anything else (from a hand-edited or corrupt file)
/// must never reach a command line.
pub fn is_safe_id(id: &str) -> bool {
    !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}
