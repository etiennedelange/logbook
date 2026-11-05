//! Read-only service layer over `SessionIndex`/`indexer::run`, translating internal
//! models into the DTOs handed to the desktop UI. Owns no business logic beyond thin
//! translation + locking — all scanning/caching/search logic lives in `index`/`indexer`/
//! `fts_query`.

use crate::dto::{FileEditDto, SearchHitDto, SessionDto, SessionFilesDto, TimelineEntryDto};
use crate::index::SessionIndex;
use crate::indexer::{self, IndexStats};
use crate::models::{Agent, SessionRecord};
use crate::source::{self, Resume, Source};
use crate::{fts_query, paths};
use anyhow::Result;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

pub use crate::source::is_safe_id;

pub struct SessionService {
    index: RwLock<SessionIndex>,
    sources: Vec<Box<dyn Source>>,
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
            sources: source::all(),
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
        let tokens = fts_query::strip_resume_command(fts_query::build(q));
        read(&self.index).search(&tokens, phrase_only, limit).into_iter().map(SearchHitDto::from).collect()
    }

    pub fn get(&self, id: &str) -> Option<SessionDto> {
        read(&self.index).get(id).map(SessionDto::from)
    }

    /// `None` for an unknown id, so it reliably reads as "not found" rather than an empty transcript.
    pub fn transcript(&self, id: &str) -> Option<Vec<TimelineEntryDto>> {
        let (record, source) = self.session(id)?;
        let timeline = read(&self.index).kept(id).unwrap_or_else(|| source.transcript(&record));
        Some(timeline.into_iter().map(TimelineEntryDto::from).collect())
    }

    /// The files a session changed and the commands that may have changed others. `None` for an
    /// unknown id.
    pub fn files(&self, id: &str) -> Option<SessionFilesDto> {
        read(&self.index).get(id).map(SessionFilesDto::from)
    }

    /// Each edit the session made to `path` (one of `files`), oldest first, read from the agent's
    /// file. `None` for an unknown id; empty when the file is gone or holds no such edits.
    pub fn file_edits(&self, id: &str, path: &str) -> Option<Vec<FileEditDto>> {
        let (record, source) = self.session(id)?;
        if record.source_deleted_at.is_some() {
            return Some(Vec::new());
        }
        Some(source.file_edits(&record, path).into_iter().map(FileEditDto::from).collect())
    }

    /// How to resume a session. `None` for an unknown id, or one whose native id isn't safe
    /// to put on a command line. A session whose source was deleted still has a command (for
    /// Copy), though the agent can no longer resume it.
    pub fn resume(&self, id: &str) -> Option<Resume> {
        let (record, source) = self.session(id)?;
        source.resume(&record)
    }

    /// `resume` as one line, for Copy.
    pub fn resume_command(&self, id: &str) -> Option<String> {
        self.resume(id).map(|r| r.command_line())
    }

    /// Each source's agent and folders, for About.
    pub fn sources(&self) -> Vec<(Agent, Vec<PathBuf>)> {
        self.sources.iter().map(|s| (s.agent(), s.roots())).collect()
    }

    fn session(&self, id: &str) -> Option<(SessionRecord, &dyn Source)> {
        let record = read(&self.index).get(id)?;
        let source = self.sources.iter().find(|s| s.agent() == record.agent)?;
        Some((record, source.as_ref()))
    }

    /// Scans a copy of the index, so reads keep being served from the current one while
    /// the scan runs; the write lock is only held to swap the result in.
    pub fn rebuild(&self) -> Result<IndexStats> {
        let mut next = read(&self.index).clone();
        let stats = indexer::run(&mut next, &self.sources)?;
        if stats.changed() {
            *write(&self.index) = next;
            self.generation.fetch_add(1, Ordering::SeqCst);
        }
        *write(&self.last_stats) = stats;
        Ok(stats)
    }
}
