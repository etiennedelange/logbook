use crate::index::SessionIndex;
use crate::models::{Agent, SessionRecord, TimelineEntry};
use crate::source::{Fingerprint, Scan, SessionFiles, Source};
use anyhow::Result;
use rayon::prelude::*;
use serde::Serialize;
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct IndexStats {
    pub total: usize,
    pub scanned: usize,
    pub skipped: usize,
    /// Sessions dropped because their files are gone or can't be read.
    pub removed: usize,
    /// Sessions whose files were found gone this run but which Logbook keeps (see `Scan::kept`).
    pub source_deleted: usize,
}

impl IndexStats {
    /// Whether the run changed the index at all.
    pub fn changed(&self) -> bool {
        self.scanned > 0 || self.removed > 0 || self.source_deleted > 0
    }
}

/// Scans every source's sessions and incrementally updates the search index. Sessions whose
/// files haven't changed since the last run (same sizes and mtimes) are skipped entirely for speed.
pub fn run(index: &mut SessionIndex, sources: &[Box<dyn Source>]) -> Result<IndexStats> {
    let mut stats = IndexStats::default();
    let mut present: HashSet<String> = HashSet::new();
    // An agent whose folders are all missing isn't installed here, or its folder is on a drive
    // that isn't mounted: either way its cached sessions stay until it can be looked at again.
    let mut unavailable: Vec<Agent> = Vec::new();

    for source in sources {
        if !source.roots().iter().any(|r| r.exists()) {
            unavailable.push(source.agent());
            continue;
        }
        let found = source.discover()?;
        stats.total += found.len();

        // Decide per session whether it needs re-reading, and do the reading across all cores:
        // the first run parses tens of MB of transcripts and each session is independent. The
        // index is only read here; results are applied serially below.
        let shared: &SessionIndex = index;
        let agent = source.agent();
        let outcomes: Vec<Option<Scanned>> =
            found.par_iter().map(|files| scan(shared, source.as_ref(), agent, files)).collect();

        // A session that can't be read any more can't be refreshed, so it isn't `present` and is dropped.
        for outcome in outcomes.into_iter().flatten() {
            match outcome {
                Scanned::Unchanged(id) => {
                    present.insert(id);
                    stats.skipped += 1;
                }
                Scanned::Fresh { record, text, fingerprint, kept } => {
                    present.insert(record.id.clone());
                    index.upsert(*record, text, fingerprint, kept);
                    stats.scanned += 1;
                }
            }
        }
    }

    (stats.removed, stats.source_deleted) = index.retire_missing(&present, &unavailable, chrono::Utc::now());

    // The cache is tens of MB; with nothing new (the usual case) there's no need to rewrite it.
    if stats.changed() {
        index.flush()?;
    }
    Ok(stats)
}

enum Scanned {
    Unchanged(String),
    Fresh { record: Box<SessionRecord>, text: String, fingerprint: Fingerprint, kept: Option<Vec<TimelineEntry>> },
}

/// `None` when the source can't read the session.
fn scan(index: &SessionIndex, source: &dyn Source, agent: Agent, files: &SessionFiles) -> Option<Scanned> {
    // What the source found on disk is the session's identity, whatever its files say about
    // themselves: otherwise a session would be re-read on every run and never match its files.
    let id = agent.session_id(&files.native_id);
    // A kept copy whose file is back is re-read even when the file looks the same (restored from the
    // Recycle Bin, which keeps its size and modified time): that is what clears "source deleted".
    if index.fingerprint(&id) == Some(&files.fingerprint) && !index.is_source_deleted(&id) {
        return Some(Scanned::Unchanged(id));
    }
    let Scan { record, searchable_text, kept } = source.read(files)?;
    // A session read from its file isn't deleted, even if it was before (the file came back).
    let record = SessionRecord { id, agent, native_id: files.native_id.clone(), source_deleted_at: None, ..record };
    Some(Scanned::Fresh {
        record: Box::new(record),
        text: searchable_text,
        fingerprint: files.fingerprint.clone(),
        kept,
    })
}
