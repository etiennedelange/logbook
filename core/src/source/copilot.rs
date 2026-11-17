//! The GitHub Copilot CLI: one folder per session under `~/.copilot/session-state/`, named by
//! the session's id, holding `workspace.yaml` (metadata) and `events.jsonl` (the conversation).

use super::{is_safe_id, Fingerprint, Resume, Scan, SessionFiles, Source};
use crate::changes;
use crate::models::{Agent, FileChange, FileEdit, SessionRecord, TimelineEntry};
use crate::{events_reader, paths, yaml_reader};
use anyhow::Result;
use std::path::{Path, PathBuf};

#[derive(Default)]
pub struct Copilot;

impl Source for Copilot {
    fn agent(&self) -> Agent {
        Agent::Copilot
    }

    fn roots(&self) -> Vec<PathBuf> {
        vec![paths::session_state_root()]
    }

    fn discover(&self) -> Result<Vec<SessionFiles>> {
        let mut found = Vec::new();
        for entry in std::fs::read_dir(paths::session_state_root())? {
            let Ok(entry) = entry else { continue };
            let dir = entry.path();
            if !dir.is_dir() {
                continue;
            }
            // Copilot rewrites `workspace.yaml` (e.g. on rename) without touching `events.jsonl`,
            // so both are part of the fingerprint.
            // Files in the session's `files/` folder count too, so a new one triggers a re-read.
            let attached = scan_files(&dir);
            let events = dir.join("events.jsonl");
            let workspace = dir.join("workspace.yaml");
            let mut stamped: Vec<&Path> = vec![&events, &workspace];
            stamped.extend(attached.iter().map(|(_, p)| p.as_path()));
            let fingerprint = Fingerprint::of(&stamped);
            found.push(SessionFiles {
                native_id: entry.file_name().to_string_lossy().into_owned(),
                path: dir,
                fingerprint,
            });
        }
        Ok(found)
    }

    fn read(&self, files: &SessionFiles) -> Option<Scan> {
        let record = yaml_reader::try_read(&files.path)?;
        let scan = events_reader::scan_for_index(&files.path.join("events.jsonl"));
        Some(Scan {
            record: SessionRecord {
                message_count: scan.message_count,
                first_prompt: scan.first_prompt,
                files: attached_changes(&files.path),
                ..record
            },
            searchable_text: scan.searchable_text,
            kept: None,
        })
    }

    /// Reads from the indexed session folder, never from a path built out of the id.
    fn transcript(&self, record: &SessionRecord) -> Vec<TimelineEntry> {
        events_reader::scan(&Path::new(&record.path).join("events.jsonl")).timeline
    }

    /// Only a path from the scan of `files/` is read: the caller's path is never joined.
    fn file_edits(&self, record: &SessionRecord, path: &str) -> Vec<FileEdit> {
        let found = scan_files(Path::new(&record.path));
        let Some((_, full)) = found.iter().find(|(rel, _)| rel == path) else { return Vec::new() };
        let Ok(bytes) = std::fs::read(full) else { return Vec::new() };
        let Ok(text) = String::from_utf8(bytes) else { return Vec::new() };
        let timestamp = std::fs::metadata(full).and_then(|m| m.modified()).ok().map(chrono::DateTime::from);
        let (hunks, truncated) = changes::created_hunk(&text);
        vec![FileEdit { created: true, timestamp, hunks, truncated }]
    }

    fn resume(&self, record: &SessionRecord) -> Option<Resume> {
        is_safe_id(&record.native_id).then(|| Resume {
            program: "copilot".into(),
            args: vec![format!("--resume={}", record.native_id)],
            cwd: record.cwd.clone(),
        })
    }
}

const MAX_FILES: usize = 200;
const MAX_DEPTH: usize = 4;

/// The files under `<session>/files/` as (path like `files/notes.md`, full path), sorted by that
/// path. Symlinks are not followed.
fn scan_files(session: &Path) -> Vec<(String, PathBuf)> {
    let mut found = Vec::new();
    walk(&session.join("files"), "files", 1, &mut found);
    found.sort_by(|a, b| a.0.cmp(&b.0));
    found.truncate(MAX_FILES);
    found
}

fn walk(dir: &Path, rel: &str, depth: usize, out: &mut Vec<(String, PathBuf)>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let Ok(kind) = entry.file_type() else { continue };
        let name = format!("{rel}/{}", entry.file_name().to_string_lossy());
        if kind.is_file() {
            out.push((name, entry.path()));
        } else if kind.is_dir() && depth < MAX_DEPTH {
            walk(&entry.path(), &name, depth + 1, out);
        }
    }
}

fn attached_changes(session: &Path) -> Vec<FileChange> {
    scan_files(session)
        .into_iter()
        .map(|(path, full)| {
            let added = std::fs::read(&full)
                .ok()
                .and_then(|b| String::from_utf8(b).ok())
                .map_or(0, |t| t.lines().count() as u32);
            FileChange { path, created: true, edits: 1, added, removed: 0 }
        })
        .collect()
}
