//! Read-only service layer over `SessionIndex`/`indexer::run`, translating internal
//! models into the DTOs handed to the desktop UI. Owns no business logic beyond thin
//! translation + locking — all scanning/caching/search logic lives in `index`/`indexer`/
//! `fts_query`.

use crate::dto::{SearchHitDto, SessionDto, TimelineEntryDto};
use crate::index::SessionIndex;
use crate::indexer::{self, IndexStats};
use crate::{events_reader, fts_query, paths};
use anyhow::Result;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

pub struct SessionService {
    index: RwLock<SessionIndex>,
    /// Stats from the last indexer run, so they can be reported without re-scanning.
    last_stats: RwLock<IndexStats>,
    /// Goes up whenever an indexer run changes the index, so the UI can tell when its copy
    /// of the session list is stale.
    generation: AtomicU64,
}

// A panic while a lock was held must not take every later command down with it: the
// index is only ever replaced whole, so the data behind a poisoned lock is still sound.
fn read<T>(lock: &RwLock<T>) -> RwLockReadGuard<'_, T> {
    lock.read().unwrap_or_else(|e| e.into_inner())
}

fn write<T>(lock: &RwLock<T>) -> RwLockWriteGuard<'_, T> {
    lock.write().unwrap_or_else(|e| e.into_inner())
}

impl SessionService {
    /// Opens (or creates) the on-disk index cache without scanning, so the UI can show
    /// cached sessions immediately; call `rebuild` to pick up new/changed sessions.
    pub fn open() -> Result<Self> {
        Ok(Self {
            index: RwLock::new(SessionIndex::open(&paths::index_cache_path())?),
            last_stats: RwLock::new(IndexStats::default()),
            generation: AtomicU64::new(0),
        })
    }

    /// `open` followed by one indexer run.
    pub fn bootstrap() -> Result<Self> {
        let service = Self::open()?;
        service.rebuild()?;
        Ok(service)
    }

    /// The stats from the most recent indexer run, without re-scanning.
    pub fn stats(&self) -> IndexStats {
        *read(&self.last_stats)
    }

    /// How many sessions the UI lists (see `SessionIndex::listed_count`).
    pub fn listed_count(&self) -> usize {
        read(&self.index).listed_count()
    }

    /// See `generation` on the struct.
    pub fn generation(&self) -> u64 {
        self.generation.load(Ordering::SeqCst)
    }

    pub fn recent(&self, limit: usize) -> Vec<SessionDto> {
        read(&self.index).all(limit).into_iter().map(SessionDto::from).collect()
    }

    /// Ranked word search (see `SessionIndex::search`); wrapping the query in double
    /// quotes keeps only exact-phrase matches. Blank input matches nothing.
    pub fn search(&self, q: &str, limit: usize) -> Vec<SearchHitDto> {
        let q = q.trim();
        let phrase_only = q.len() > 1 && q.starts_with('"') && q.ends_with('"');
        let tokens = fts_query::build(q);
        read(&self.index)
            .search(&tokens, phrase_only, limit)
            .into_iter()
            .map(SearchHitDto::from)
            .collect()
    }

    pub fn get(&self, id: &str) -> Option<SessionDto> {
        read(&self.index).get(id).map(SessionDto::from)
    }

    /// `None` for an unknown id, so it reliably reads as "not found" rather than an empty transcript.
    /// Reads from the indexed session folder, never from a path built out of the id.
    pub fn transcript(&self, id: &str) -> Option<Vec<TimelineEntryDto>> {
        let session = self.get(id)?;
        let scan = events_reader::scan(&Path::new(&session.path).join("events.jsonl"));
        Some(scan.timeline.into_iter().map(TimelineEntryDto::from).collect())
    }

    pub fn resume_command(&self, id: &str) -> Option<String> {
        self.get(id).filter(|_| is_safe_id(id)).map(|_| format!("copilot --resume={id}"))
    }

    /// Scans a copy of the index, so reads keep being served from the current one while
    /// the scan runs; the write lock is only held to swap the result in.
    pub fn rebuild(&self) -> Result<IndexStats> {
        let mut next = read(&self.index).clone();
        let stats = indexer::run(&mut next)?;
        if stats.changed() {
            *write(&self.index) = next;
            self.generation.fetch_add(1, Ordering::SeqCst);
        }
        *write(&self.last_stats) = stats;
        Ok(stats)
    }
}

/// Session ids are UUIDs; anything else (from a hand-edited or corrupt `workspace.yaml`)
/// must never reach a shell command line.
pub fn is_safe_id(id: &str) -> bool {
    !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}
