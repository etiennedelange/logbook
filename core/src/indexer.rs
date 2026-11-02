use crate::index::{Fingerprint, SessionIndex};
use crate::{events_reader, paths, yaml_reader};
use crate::models::SessionRecord;
use anyhow::Result;
use rayon::prelude::*;
use serde::Serialize;
use std::collections::HashSet;
use std::path::Path;

#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct IndexStats {
    pub total: usize,
    pub scanned: usize,
    pub skipped: usize,
    /// Sessions dropped because their folder is gone or its `workspace.yaml` can't be read.
    pub removed: usize,
}

impl IndexStats {
    /// Whether the run changed the index at all.
    pub fn changed(&self) -> bool {
        self.scanned > 0 || self.removed > 0
    }
}

/// Scans every session under the Copilot session-state root and incrementally updates the
/// search index. Sessions whose events.jsonl hasn't changed since the last run (same size +
/// mtime) are skipped entirely for speed.
pub fn run(index: &mut SessionIndex) -> Result<IndexStats> {
    let root = paths::session_state_root();
    if !root.exists() {
        return Ok(IndexStats::default());
    }

    let dirs: Vec<_> = std::fs::read_dir(&root)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();

    // Decide per session whether it needs re-reading, and do the reading across all cores: the
    // first run parses tens of MB of transcripts and each session is independent. The index is
    // only read here; results are applied serially below.
    let shared: &SessionIndex = index;
    let outcomes: Vec<Option<Scanned>> = dirs.par_iter().map(|dir| scan_dir(shared, dir)).collect();

    // Sessions are matched to folders by name (see `yaml_reader::try_read`). One whose folder is
    // gone, or whose `workspace.yaml` no longer reads, can't be refreshed and is dropped.
    let mut present: HashSet<String> = HashSet::with_capacity(dirs.len());
    let mut scanned = 0usize;
    let mut skipped = 0usize;
    for (dir, outcome) in dirs.iter().zip(outcomes) {
        let Some(outcome) = outcome else { continue };
        if let Some(name) = dir.file_name() {
            present.insert(name.to_string_lossy().into_owned());
        }
        match outcome {
            Scanned::Unchanged => skipped += 1,
            Scanned::Fresh { record, text, fingerprint } => {
                index.upsert(record, text, fingerprint);
                scanned += 1;
            }
        }
    }

    let removed = index.remove_missing(&present);

    let stats = IndexStats { total: dirs.len(), scanned, skipped, removed };
    // The cache is tens of MB; with nothing new (the usual case) there's no need to rewrite it.
    if stats.changed() {
        index.flush()?;
    }
    Ok(stats)
}

enum Scanned {
    Unchanged,
    Fresh { record: SessionRecord, text: String, fingerprint: Fingerprint },
}

/// `None` when the folder has no readable `workspace.yaml`.
fn scan_dir(index: &SessionIndex, dir: &Path) -> Option<Scanned> {
    let record = yaml_reader::try_read(dir)?;

    let events_path = dir.join("events.jsonl");
    let events = std::fs::metadata(&events_path).ok();
    let fingerprint = Fingerprint {
        events_mtime_ms: events.as_ref().map_or(0, mtime_ms),
        events_size: events.as_ref().map_or(0, |m| m.len()),
        yaml_mtime_ms: std::fs::metadata(dir.join("workspace.yaml")).map_or(0, |m| mtime_ms(&m)),
    };

    if index.fingerprint(&record.id) == Some(&fingerprint) {
        return Some(Scanned::Unchanged);
    }

    let scan = events_reader::scan_for_index(&events_path);
    let record = SessionRecord {
        message_count: scan.message_count,
        first_prompt: scan.first_prompt,
        ..record
    };
    Some(Scanned::Fresh { record, text: scan.searchable_text, fingerprint })
}

fn mtime_ms(meta: &std::fs::Metadata) -> u128 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_millis())
}
