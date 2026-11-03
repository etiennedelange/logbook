use crate::models::{Agent, SessionRecord, SessionSearchResult, TimelineEntry};
use crate::source::{kept_text, Fingerprint};
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// A cached, parsed session plus enough fingerprint info to know whether it needs
/// re-scanning next run.
#[derive(Clone)]
struct Entry {
    record: SessionRecord,
    searchable_text: String,
    fingerprint: Fingerprint,
    /// Logbook's own copy of the transcript, for sources whose agent deletes its files.
    kept: Option<Vec<TimelineEntry>>,
}

/// An `Entry` as written to the cache. A kept transcript already holds the searchable text, so
/// the text is left empty and rebuilt from it on load (`kept_text`).
#[derive(Serialize)]
struct StoredRef<'a> {
    record: &'a SessionRecord,
    searchable_text: &'a str,
    fingerprint: &'a Fingerprint,
    kept: Option<&'a [TimelineEntry]>,
}

/// `StoredRef`, read back. Postcard encodes both the same way.
#[derive(Deserialize)]
struct Stored {
    record: SessionRecord,
    searchable_text: String,
    fingerprint: Fingerprint,
    kept: Option<Vec<TimelineEntry>>,
}

impl From<Stored> for Entry {
    fn from(s: Stored) -> Self {
        let searchable_text = s.kept.as_deref().map_or(s.searchable_text, kept_text);
        Self { record: s.record, searchable_text, fingerprint: s.fingerprint, kept: s.kept }
    }
}

/// An `Entry` with its header and transcript already split into words, so a search never
/// re-tokenizes text. Built when a session is loaded or re-scanned; never persisted.
struct Indexed {
    entry: Entry,
    header: Words,
    body: Words,
}

impl Indexed {
    fn new(entry: Entry) -> Self {
        Self { header: Words::new(&header_text(&entry.record)), body: Words::new(&entry.searchable_text), entry }
    }
}

/// An in-memory index over every agent's sessions, persisted to a flat `postcard`
/// cache file. Rebuilt incrementally: a session is only re-scanned when its files'
/// sizes or timestamps have changed since the last run (see `Fingerprint`). At the personal-scale
/// data sizes involved (about a thousand sessions, tens of MB of transcript text),
/// matching against each session's pre-split words (see `Words`) takes a few
/// milliseconds, so no database or search engine is needed.
#[derive(Clone)]
pub struct SessionIndex {
    path: PathBuf,
    /// `Arc` so cloning the index for a background rebuild doesn't copy every transcript.
    entries: HashMap<String, Arc<Indexed>>,
}

impl SessionIndex {
    pub fn open(cache_path: &Path) -> Result<Self> {
        let list = std::fs::read(cache_path)
            .ok()
            .and_then(|bytes| postcard::from_bytes::<Vec<Stored>>(&bytes).ok())
            .unwrap_or_default()
            .into_iter()
            .map(Entry::from)
            .collect();
        let entries = index_all(list).into_iter().map(|e| (e.entry.record.id.clone(), Arc::new(e))).collect();

        Ok(Self { path: cache_path.to_path_buf(), entries })
    }

    /// Persists the current in-memory index to disk. Writes a temp file and renames it over
    /// the cache, so a crash mid-write can't leave a truncated cache behind.
    pub fn flush(&self) -> Result<()> {
        let list: Vec<StoredRef> = self
            .entries
            .values()
            .map(|e| {
                let e = &e.entry;
                StoredRef {
                    record: &e.record,
                    searchable_text: if e.kept.is_some() { "" } else { &e.searchable_text },
                    fingerprint: &e.fingerprint,
                    kept: e.kept.as_deref(),
                }
            })
            .collect();
        let bytes = postcard::to_stdvec(&list)?;
        let tmp = self.path.with_extension("tmp");
        std::fs::write(&tmp, bytes)?;
        std::fs::rename(&tmp, &self.path)?;
        Ok(())
    }

    pub fn fingerprint(&self, id: &str) -> Option<&Fingerprint> {
        self.entries.get(id).map(|e| &e.entry.fingerprint)
    }

    pub fn upsert(
        &mut self,
        record: SessionRecord,
        searchable_text: String,
        fingerprint: Fingerprint,
        kept: Option<Vec<TimelineEntry>>,
    ) {
        let id = record.id.clone();
        let entry = Entry { record, searchable_text, fingerprint, kept };
        self.entries.insert(id, Arc::new(Indexed::new(entry)));
    }

    /// Logbook's kept copy of a session's transcript, if it has one.
    pub fn kept(&self, id: &str) -> Option<Vec<TimelineEntry>> {
        self.entries.get(id).and_then(|e| e.entry.kept.clone())
    }

    /// Deals with sessions that are no longer on disk, leaving alone those of the agents in
    /// `unavailable` (whose folders weren't there to look in). A session with a kept transcript
    /// and messages stays, marked as deleted at `now` the first time; any other is dropped.
    /// Returns how many were dropped and how many newly marked.
    pub fn retire_missing(
        &mut self,
        existing_ids: &HashSet<String>,
        unavailable: &[Agent],
        now: DateTime<Utc>,
    ) -> (usize, usize) {
        let before = self.entries.len();
        let mut marked = 0;
        self.entries.retain(|id, e| {
            let r = &e.entry.record;
            if existing_ids.contains(id) || unavailable.contains(&r.agent) {
                return true;
            }
            if e.entry.kept.is_none() || r.message_count == 0 {
                return false;
            }
            if r.source_deleted_at.is_none() {
                let mut entry = e.entry.clone();
                entry.record.source_deleted_at = Some(now);
                *e = Arc::new(Indexed::new(entry));
                marked += 1;
            }
            true
        });
        (before - self.entries.len(), marked)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn get(&self, id: &str) -> Option<SessionRecord> {
        self.entries.get(id).map(|e| e.entry.record.clone())
    }

    /// Sessions worth listing: those with at least one message. Copilot leaves behind many
    /// sessions that were opened and never used (often with no `events.jsonl` at all); with
    /// nothing to read or resume they would only bury the real ones. `get` still finds them.
    fn listed(&self) -> impl Iterator<Item = &Indexed> {
        self.entries.values().map(|e| &**e).filter(|e| e.entry.record.message_count > 0)
    }

    /// How many sessions `all` would return without a limit.
    pub fn listed_count(&self) -> usize {
        self.listed().count()
    }

    pub fn all(&self, limit: usize) -> Vec<SessionRecord> {
        let mut records: Vec<&SessionRecord> = self.listed().map(|e| &e.entry.record).collect();
        records.sort_by(|a, b| newest_first(a, b));
        records.into_iter().take(limit).cloned().collect()
    }

    /// Searches each session's name, repository, branch, folder and transcript. A session
    /// matches when every token starts some word in it; matches are ranked by how well
    /// they match (see `Rank`), then by whether the session's own details matched, then
    /// most recent first. `phrase_only` keeps only sessions containing the tokens in order.
    pub fn search(&self, tokens: &[String], phrase_only: bool, limit: usize) -> Vec<SessionSearchResult> {
        if tokens.is_empty() {
            return Vec::new();
        }

        let mut hits: Vec<Hit> = self.listed().filter_map(|e| score(e, tokens, phrase_only)).collect();
        hits.sort_by(|a, b| {
            b.rank
                .cmp(&a.rank)
                .then(b.in_header.cmp(&a.in_header))
                .then_with(|| newest_first(&a.entry.entry.record, &b.entry.entry.record))
        });
        hits.truncate(limit);
        // Snippets are only built for the hits actually returned.
        hits.into_iter()
            .map(|h| SessionSearchResult {
                session: h.entry.entry.record.clone(),
                snippet: build_snippet(h.entry, tokens, h.rank, h.body_phrase),
            })
            .collect()
    }
}

/// Most recently updated first; the id breaks ties so the order is the same every time.
fn newest_first(a: &SessionRecord, b: &SessionRecord) -> std::cmp::Ordering {
    b.updated_at.cmp(&a.updated_at).then_with(|| a.id.cmp(&b.id))
}

/// Splits every cached entry into words, spread across the CPU's cores: this is most of
/// the work of loading the cache at startup.
fn index_all(list: Vec<Entry>) -> Vec<Indexed> {
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get()).min(list.len());
    if threads <= 1 {
        return list.into_iter().map(Indexed::new).collect();
    }
    let mut buckets: Vec<Vec<Entry>> = (0..threads).map(|_| Vec::new()).collect();
    for (i, e) in list.into_iter().enumerate() {
        buckets[i % threads].push(e);
    }
    std::thread::scope(|s| {
        let handles: Vec<_> =
            buckets.into_iter().map(|b| s.spawn(move || b.into_iter().map(Indexed::new).collect::<Vec<_>>())).collect();
        handles.into_iter().flat_map(|h| h.join().expect("indexing thread panicked")).collect()
    })
}

struct Hit<'a> {
    entry: &'a Indexed,
    rank: Rank,
    in_header: bool,
    body_phrase: Option<usize>,
}

fn score<'a>(e: &'a Indexed, tokens: &[String], phrase_only: bool) -> Option<Hit<'a>> {
    let (header, body) = (&e.header, &e.body);
    // Cheapest test first: a token that starts no word anywhere rules the session out.
    if !tokens.iter().all(|t| body.has_prefix(t) || header.has_prefix(t)) {
        return None;
    }
    let body_phrase = body.find_phrase(tokens);
    let rank = if body_phrase.is_some() || header.find_phrase(tokens).is_some() {
        Rank::Phrase
    } else if tokens.iter().all(|t| body.id_of(t).is_some() || header.id_of(t).is_some()) {
        Rank::Words
    } else {
        Rank::Prefix
    };
    if phrase_only && rank != Rank::Phrase {
        return None;
    }
    let in_header = tokens.iter().all(|t| header.has_prefix(t));
    Some(Hit { entry: e, rank, in_header, body_phrase })
}

/// A text split into words: each distinct lowercased word once, sorted, plus the order the
/// words appear in as indexes into that list. Sorting puts every word that starts with a
/// given prefix in one contiguous run, so whole-word and prefix lookups are binary searches
/// and a phrase is a scan over integers.
struct Words {
    vocab: Vec<Box<str>>,
    /// `vocab` index of each word, in text order.
    seq: Vec<u32>,
    /// Byte range of each word in the original text, in text order.
    spans: Vec<(u32, u32)>,
}

impl Words {
    fn new(text: &str) -> Self {
        let words = tokenize_with_positions(text);
        let mut first_id: HashMap<Cow<str>, u32> = HashMap::new();
        let mut seq = Vec::with_capacity(words.len());
        let mut spans = Vec::with_capacity(words.len());
        for &(s, e, w) in &words {
            let next = first_id.len() as u32;
            seq.push(*first_id.entry(lowercase(w)).or_insert(next));
            spans.push((s as u32, e as u32));
        }

        // Renumber so ids follow sorted order.
        let mut vocab: Vec<(Cow<str>, u32)> = first_id.into_iter().collect();
        vocab.sort_unstable_by(|a, b| a.0.cmp(&b.0));
        let mut remap = vec![0u32; vocab.len()];
        for (sorted, (_, first)) in vocab.iter().enumerate() {
            remap[*first as usize] = sorted as u32;
        }
        for id in &mut seq {
            *id = remap[*id as usize];
        }
        let vocab = vocab.into_iter().map(|(w, _)| w.into_owned().into_boxed_str()).collect();
        Self { vocab, seq, spans }
    }

    fn id_of(&self, word: &str) -> Option<u32> {
        self.vocab.binary_search_by(|w| (**w).cmp(word)).ok().map(|i| i as u32)
    }

    /// Ids of every word starting with `prefix`.
    fn prefixed(&self, prefix: &str) -> Range<u32> {
        let lo = self.vocab.partition_point(|w| &**w < prefix);
        let len = self.vocab[lo..].partition_point(|w| w.starts_with(prefix));
        lo as u32..(lo + len) as u32
    }

    fn has_prefix(&self, prefix: &str) -> bool {
        !self.prefixed(prefix).is_empty()
    }

    /// Index of the first run of words matching `tokens` in order: whole words, except the
    /// last token, which only has to start its word. Needs at least two tokens.
    fn find_phrase(&self, tokens: &[String]) -> Option<usize> {
        let n = tokens.len();
        if n < 2 || self.seq.len() < n {
            return None;
        }
        let whole: Vec<u32> = tokens[..n - 1].iter().map(|t| self.id_of(t)).collect::<Option<_>>()?;
        let last = self.prefixed(&tokens[n - 1]);
        if last.is_empty() {
            return None;
        }
        self.seq.windows(n).position(|w| w[..n - 1] == whole[..] && last.contains(&w[n - 1]))
    }
}

/// Lowercases a word, borrowing it when it is already lowercase ASCII (most of them).
fn lowercase(w: &str) -> Cow<'_, str> {
    if w.bytes().all(|b| b.is_ascii() && !b.is_ascii_uppercase()) {
        Cow::Borrowed(w)
    } else {
        Cow::Owned(w.to_lowercase())
    }
}

/// Splits text into (start_byte, end_byte, word) triples for contiguous runs of
/// alphanumeric/`-`/`_` characters, preserving byte positions in the original string
/// so a matched word's surrounding punctuation/whitespace can be reproduced verbatim.
fn tokenize_with_positions(text: &str) -> Vec<(usize, usize, &str)> {
    let mut result = Vec::new();
    let mut start: Option<usize> = None;

    for (i, c) in text.char_indices() {
        let is_word = c.is_alphanumeric() || c == '-' || c == '_';
        match (is_word, start) {
            (true, None) => start = Some(i),
            (false, Some(s)) => {
                result.push((s, i, &text[s..i]));
                start = None;
            }
            _ => {}
        }
    }
    if let Some(s) = start {
        result.push((s, text.len(), &text[s..]));
    }
    result
}

/// How well a session matched a query, worst to best.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Rank {
    /// Every token starts some word ("gets" in "getSpecification").
    Prefix,
    /// Every token is a whole word somewhere, though not together.
    Words,
    /// The tokens appear in order, as typed (the last may still be half-typed).
    Phrase,
}

/// The session's own details, searched alongside its transcript.
fn header_text(r: &SessionRecord) -> String {
    [&r.name, &r.repository, &r.branch, &r.cwd].into_iter().flatten().map(String::as_str).collect::<Vec<_>>().join(" ")
}

/// Builds a short snippet around the best match, wrapping matched words in `\u{2}`/`\u{3}`
/// (STX/ETX) markers for the UI to style. A phrase match marks just the phrase; otherwise
/// the words that earned the rank (whole words, or prefixes) are marked.
fn build_snippet(e: &Indexed, tokens: &[String], rank: Rank, phrase_at: Option<usize>) -> Option<String> {
    let (text, words) = (&e.entry.searchable_text, &e.body);
    let whole: Vec<u32> = tokens.iter().filter_map(|t| words.id_of(t)).collect();
    let prefixed: Vec<Range<u32>> = tokens.iter().map(|t| words.prefixed(t)).collect();
    let is_hit = |i: usize| match (rank, phrase_at) {
        (Rank::Phrase, Some(p)) => i >= p && i < p + tokens.len(),
        (Rank::Words, _) | (Rank::Phrase, None) => whole.contains(&words.seq[i]),
        (Rank::Prefix, _) => prefixed.iter().any(|r| r.contains(&words.seq[i])),
    };

    let n = words.seq.len();
    let match_idx = phrase_at.or_else(|| (0..n).find(|&i| is_hit(i)))?;

    const BEFORE: usize = 5;
    const AFTER: usize = 8;
    let start_idx = match_idx.saturating_sub(BEFORE);
    let end_idx = (match_idx + tokens.len() + AFTER).min(n);

    let mut snippet = String::new();
    let mut last = words.spans[start_idx].0 as usize;
    for i in start_idx..end_idx {
        let (ws, we) = (words.spans[i].0 as usize, words.spans[i].1 as usize);
        snippet.push_str(&text[last..ws]);
        if is_hit(i) {
            snippet.push('\u{2}');
            snippet.push_str(&text[ws..we]);
            snippet.push('\u{3}');
        } else {
            snippet.push_str(&text[ws..we]);
        }
        last = we;
    }

    let prefix = if start_idx > 0 { "... " } else { "" };
    let suffix = if end_idx < n { " ..." } else { "" };
    Some(format!("{prefix}{snippet}{suffix}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::FileStamp;
    use chrono::{TimeZone, Utc};

    fn record(id: &str, name: &str, day: u32) -> SessionRecord {
        SessionRecord {
            id: id.into(),
            agent: Agent::Copilot,
            native_id: id.into(),
            name: Some(name.into()),
            user_named: false,
            cwd: None,
            git_root: None,
            repository: Some("Org/Project/Backend".into()),
            branch: None,
            client_name: None,
            created_at: None,
            updated_at: Some(Utc.with_ymd_and_hms(2026, 10, day, 12, 0, 0).unwrap()),
            message_count: 1,
            first_prompt: None,
            path: String::new(),
            source_deleted_at: None,
        }
    }

    fn index(sessions: &[(&str, &str, u32, &str)]) -> SessionIndex {
        let mut idx = SessionIndex::open(Path::new("does-not-exist/index.bin")).unwrap();
        for (id, name, day, text) in sessions {
            idx.upsert(record(id, name, *day), text.to_string(), Fingerprint::default(), None);
        }
        idx
    }

    fn ids(results: &[SessionSearchResult]) -> Vec<&str> {
        results.iter().map(|r| r.session.id.as_str()).collect()
    }

    fn toks(q: &str) -> Vec<String> {
        crate::fts_query::build(q)
    }

    #[test]
    fn exact_phrase_outranks_newer_scattered_matches() {
        let idx = index(&[
            ("prefix", "a", 9, "what getSpecification returns is scheduled later"),
            ("words", "b", 8, "what is scheduled? it gets done"),
            ("phrase", "c", 1, "What gets scheduled: only a key and a due time."),
        ]);
        let tokens = toks("what gets scheduled");
        assert_eq!(ids(&idx.search(&tokens, false, 10)), ["phrase", "words", "prefix"]);
        assert_eq!(ids(&idx.search(&tokens, true, 10)), ["phrase"]);
    }

    #[test]
    fn phrase_snippet_marks_only_the_phrase() {
        let idx = index(&[("s", "a", 1, "what now? Here is what gets scheduled: a key")]);
        let snippet = idx.search(&toks("what gets sched"), false, 10)[0].snippet.clone().unwrap();
        assert!(snippet.contains("\u{2}what\u{3} \u{2}gets\u{3} \u{2}scheduled\u{3}"), "{snippet:?}");
        assert!(snippet.starts_with("what now?"), "{snippet:?}");
    }

    #[test]
    fn matches_names_and_repositories_too() {
        let idx = index(&[("named", "Scheduler integration", 1, "nothing relevant here")]);
        assert_eq!(ids(&idx.search(&toks("scheduler"), false, 10)), ["named"]);
        assert_eq!(ids(&idx.search(&toks("backend"), false, 10)), ["named"]);
    }

    #[test]
    fn every_token_must_start_a_word() {
        let idx = index(&[("s", "a", 1, "Deploying the dashboard to staging")]);
        assert_eq!(ids(&idx.search(&toks("dash stag"), false, 10)), ["s"]);
        assert!(idx.search(&toks("dash prod"), false, 10).is_empty());
        // Inside a word isn't a match.
        assert!(idx.search(&toks("board"), false, 10).is_empty());
    }

    #[test]
    fn matching_ignores_case_including_non_ascii() {
        let idx = index(&[("s", "a", 1, "Über ÉCOLE Straße")]);
        assert_eq!(ids(&idx.search(&toks("über école straße"), false, 10)), ["s"]);
        assert_eq!(ids(&idx.search(&toks("ÜBER"), false, 10)), ["s"]);
        let snippet = idx.search(&toks("ecole école"), false, 10);
        assert!(snippet.is_empty(), "every token must match");
    }

    #[test]
    fn phrase_finds_a_later_occurrence_after_a_false_start() {
        let idx = index(&[("s", "a", 1, "what is this? what what gets scheduled")]);
        let hits = idx.search(&toks("what gets"), true, 10);
        assert_eq!(ids(&hits), ["s"]);
        assert!(hits[0].snippet.as_ref().unwrap().contains("what \u{2}what\u{3} \u{2}gets\u{3}"));
    }

    #[test]
    fn ties_break_by_id_and_limit_keeps_the_best() {
        let idx = index(&[("b", "x", 1, "alpha"), ("a", "x", 1, "alpha"), ("c", "x", 2, "alphabet")]);
        // Whole word beats prefix even when the prefix match is newer; same-day ties go by id.
        assert_eq!(ids(&idx.search(&toks("alpha"), false, 10)), ["a", "b", "c"]);
        assert_eq!(ids(&idx.search(&toks("alpha"), false, 2)), ["a", "b"]);
    }

    #[test]
    fn sessions_without_messages_are_left_out() {
        let mut idx = index(&[("used", "Backend work", 1, "some text")]);
        let mut empty = record("empty", "Backend idle", 2);
        empty.message_count = 0;
        idx.upsert(empty, String::new(), Fingerprint::default(), None);
        assert_eq!(idx.all(10).iter().map(|r| r.id.as_str()).collect::<Vec<_>>(), ["used"]);
        assert_eq!(ids(&idx.search(&toks("backend"), false, 10)), ["used"]);
        assert_eq!(idx.listed_count(), 1);
        assert!(idx.get("empty").is_some());
    }

    #[test]
    fn get_and_upsert_replace_in_place() {
        let mut idx = index(&[("s", "old", 1, "first version")]);
        idx.upsert(record("s", "new", 2), "second version".into(), Fingerprint::default(), None);
        assert_eq!(idx.len(), 1);
        assert_eq!(idx.get("s").unwrap().name.as_deref(), Some("new"));
        assert!(idx.search(&toks("first"), false, 10).is_empty());
        assert_eq!(ids(&idx.search(&toks("second"), false, 10)), ["s"]);
    }

    #[test]
    fn flushed_cache_reopens_with_records_fingerprints_and_text() {
        let dir = std::env::temp_dir().join(format!("logbook-cache-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("index.bin");
        let fp = Fingerprint(vec![
            FileStamp { path: "C:\\s\\events.jsonl".into(), size: 4096, mtime_ms: u128::MAX - 1 },
            FileStamp { path: "C:\\s\\workspace.yaml".into(), size: 120, mtime_ms: 1_700_000_000_000 },
        ]);

        let mut idx = SessionIndex::open(&path).unwrap();
        idx.upsert(record("s", "Loans report", 3), "overdue reminder queue".into(), fp.clone(), None);
        let kept = vec![
            TimelineEntry { role: "user".into(), timestamp: None, text: "where is the parser".into() },
            TimelineEntry { role: "assistant".into(), timestamp: None, text: "in src/parse.rs".into() },
        ];
        idx.upsert(record("k", "Kept one", 2), kept_text(&kept), Fingerprint::default(), Some(kept.clone()));
        idx.flush().unwrap();

        let reopened = SessionIndex::open(&path).unwrap();
        std::fs::remove_dir_all(&dir).ok();
        assert_eq!(reopened.fingerprint("s"), Some(&fp));
        assert_eq!(reopened.get("s").unwrap().updated_at, record("s", "", 3).updated_at);
        assert_eq!(ids(&reopened.search(&toks("reminder"), false, 10)), ["s"]);
        // A kept transcript is stored once and its searchable text rebuilt from it.
        assert_eq!(reopened.kept("k"), Some(kept));
        assert_eq!(ids(&reopened.search(&toks("parse rs"), false, 10)), ["k"]);
        assert_eq!(reopened.kept("s"), None);
    }

    #[test]
    fn missing_sessions_are_dropped_unless_kept() {
        let mut idx = index(&[("gone", "a", 1, "x"), ("here", "b", 1, "y")]);
        let kept = vec![TimelineEntry { role: "user".into(), timestamp: None, text: "hello".into() }];
        idx.upsert(record("kept", "c", 1), kept_text(&kept), Fingerprint::default(), Some(kept));
        let present: HashSet<String> = ["here".to_string()].into();
        let now = Utc.with_ymd_and_hms(2026, 11, 2, 9, 0, 0).unwrap();

        assert_eq!(idx.retire_missing(&present, &[], now), (1, 1));
        assert!(idx.get("gone").is_none());
        assert_eq!(idx.get("kept").unwrap().source_deleted_at, Some(now));
        // Marked once: a later run keeps the first time and reports nothing new.
        assert_eq!(idx.retire_missing(&present, &[], now + chrono::Duration::days(1)), (0, 0));
        assert_eq!(idx.get("kept").unwrap().source_deleted_at, Some(now));
        assert_eq!(ids(&idx.search(&toks("hello"), false, 10)), ["kept"]);
    }
}
