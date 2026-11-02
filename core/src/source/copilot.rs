//! The GitHub Copilot CLI: one folder per session under `~/.copilot/session-state/`, named by
//! the session's id, holding `workspace.yaml` (metadata) and `events.jsonl` (the conversation).

use super::{is_safe_id, Fingerprint, Resume, Scan, SessionFiles, Source};
use crate::models::{Agent, SessionRecord, TimelineEntry};
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
            let fingerprint = Fingerprint::of(&[&dir.join("events.jsonl"), &dir.join("workspace.yaml")]);
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
            record: SessionRecord { message_count: scan.message_count, first_prompt: scan.first_prompt, ..record },
            searchable_text: scan.searchable_text,
            kept: None,
        })
    }

    /// Reads from the indexed session folder, never from a path built out of the id.
    fn transcript(&self, record: &SessionRecord) -> Vec<TimelineEntry> {
        events_reader::scan(&Path::new(&record.path).join("events.jsonl")).timeline
    }

    fn resume(&self, record: &SessionRecord) -> Option<Resume> {
        is_safe_id(&record.native_id).then(|| Resume {
            program: "copilot".into(),
            args: vec![format!("--resume={}", record.native_id)],
            cwd: record.cwd.clone(),
        })
    }
}
