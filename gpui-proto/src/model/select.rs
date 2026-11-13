//! What the list shows: ranges, filters, repository and agent counts. A port of `select.ts`.

use super::{agents::present_agents, Session};
use chrono::{DateTime, Local, NaiveDate, TimeZone};
use logbook_core::dto::SearchHitDto;
use logbook_core::models::Agent;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Range {
    All,
    Today,
    Week,
    Month,
}

pub struct RangeInfo {
    pub key: Range,
    pub label: &'static str,
    pub hotkey: char,
}

pub const RANGES: [RangeInfo; 4] = [
    RangeInfo { key: Range::All, label: "All sessions", hotkey: '1' },
    RangeInfo { key: Range::Today, label: "Today", hotkey: '2' },
    RangeInfo { key: Range::Week, label: "Last 7 days", hotkey: '3' },
    RangeInfo { key: Range::Month, label: "Last 30 days", hotkey: '4' },
];

pub fn range_label(r: Range) -> &'static str {
    RANGES.iter().find(|i| i.key == r).map_or("", |i| i.label)
}

pub fn local_midnight_ms(d: NaiveDate) -> i64 {
    Local
        .from_local_datetime(&d.and_hms_opt(0, 0, 0).unwrap())
        .earliest()
        .map_or_else(|| d.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp_millis(), |t| t.timestamp_millis())
}

/// Start of the range as a local date, or `None` for all time.
pub fn range_start(range: Range, now: DateTime<Local>) -> Option<NaiveDate> {
    let today = now.date_naive();
    match range {
        Range::All => None,
        Range::Today => Some(today),
        Range::Week => Some(today - chrono::Days::new(6)),
        Range::Month => Some(today - chrono::Days::new(29)),
    }
}

/// A test for "is this session in the range", with the range's start worked out once.
fn range_test(range: Range, now: DateTime<Local>) -> impl Fn(&Session) -> bool {
    let start = range_start(range, now).map(local_midnight_ms);
    move |s| match start {
        None => true,
        Some(start) => s.time_ms.is_some_and(|t| t >= start),
    }
}

pub fn in_range(s: &Session, range: Range, now: DateTime<Local>) -> bool {
    range_test(range, now)(s)
}

#[derive(Clone)]
pub struct Row {
    pub session: Arc<Session>,
    /// The search hit the row is for, if any (it carries the snippet).
    pub hit: Option<SearchHitDto>,
}

impl Row {
    pub fn snippet(&self) -> Option<&str> {
        self.hit.as_ref().and_then(|h| h.snippet.as_deref())
    }
}

pub fn sessions_by_id(sessions: &[Arc<Session>]) -> HashMap<String, Arc<Session>> {
    sessions.iter().map(|s| (s.id.clone(), s.clone())).collect()
}

/// What the list shows: search hits when there is a query, otherwise every session; then the range,
/// repository and agent filters.
pub fn visible_rows(
    sessions: &[Arc<Session>],
    by_id: &HashMap<String, Arc<Session>>,
    hits: Option<&[SearchHitDto]>,
    range: Range,
    repo: Option<&str>,
    now: DateTime<Local>,
    agent: Option<Agent>,
) -> Vec<Row> {
    let ok = range_test(range, now);
    let keep = |s: &Session| {
        ok(s) && repo.is_none_or(|r| s.repo.as_deref() == Some(r)) && agent.is_none_or(|a| s.agent == a)
    };
    match hits {
        // A hit for a session that isn't loaded yet is skipped; a refresh re-runs the search once it is.
        Some(hits) => hits
            .iter()
            .filter_map(|h| by_id.get(&h.id).map(|s| (s, h)))
            .filter(|(s, _)| keep(s))
            .map(|(s, h)| Row { session: s.clone(), hit: Some(h.clone()) })
            .collect(),
        None => sessions.iter().filter(|s| keep(s)).map(|s| Row { session: s.clone(), hit: None }).collect(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoCount {
    pub name: String,
    pub count: usize,
}

/// Repositories in the given range (and agent, if one is picked), busiest first.
pub fn repos(sessions: &[Arc<Session>], range: Range, now: DateTime<Local>, agent: Option<Agent>) -> Vec<RepoCount> {
    let ok = range_test(range, now);
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for s in sessions {
        if let Some(name) = s.repo.as_deref() {
            if ok(s) && agent.is_none_or(|a| s.agent == a) {
                *counts.entry(name).or_default() += 1;
            }
        }
    }
    let mut out: Vec<RepoCount> = counts.into_iter().map(|(name, count)| RepoCount { name: name.into(), count }).collect();
    out.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    out
}

/// How many sessions each time range holds, for the sidebar.
pub fn range_counts(sessions: &[Arc<Session>], now: DateTime<Local>) -> HashMap<Range, usize> {
    RANGES
        .iter()
        .map(|r| {
            let ok = range_test(r.key, now);
            (r.key, sessions.iter().filter(|s| ok(s)).count())
        })
        .collect()
}

/// How many sessions each agent has in the range (and repository, if one is picked), in the order of
/// `present`. Every agent in `present` keeps its row, with 0 when it has none here.
pub fn agent_counts(
    sessions: &[Arc<Session>],
    present: &[Agent],
    range: Range,
    repo: Option<&str>,
    now: DateTime<Local>,
) -> Vec<(Agent, usize)> {
    let ok = range_test(range, now);
    let mut counts: HashMap<Agent, usize> = present.iter().map(|a| (*a, 0)).collect();
    for s in sessions {
        if ok(s) && repo.is_none_or(|r| s.repo.as_deref() == Some(r)) {
            if let Some(c) = counts.get_mut(&s.agent) {
                *c += 1;
            }
        }
    }
    present.iter().map(|a| (*a, counts[a])).collect()
}

/// Agents with at least one session, for the whole list.
pub fn agents_present(sessions: &[Arc<Session>]) -> Vec<Agent> {
    present_agents(sessions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::test_support::*;

    fn now() -> DateTime<Local> {
        local(2026, 10, 13, 12, 0)
    }
    fn at(days: i64) -> String {
        iso(now() - chrono::Duration::days(days))
    }
    fn sess(id: &str, repo: &str, days: i64) -> Arc<Session> {
        session(id, |s| {
            s.repository = Some(repo.into());
            s.updated_at = Some(at(days));
        })
    }
    fn ids(rows: &[Row]) -> Vec<&str> {
        rows.iter().map(|r| r.session.id.as_str()).collect()
    }
    fn rows(all: &[Arc<Session>], hits: Option<&[SearchHitDto]>, range: Range, repo: Option<&str>, agent: Option<Agent>) -> Vec<Row> {
        visible_rows(all, &sessions_by_id(all), hits, range, repo, now(), agent)
    }
    fn hit(s: &Session, snippet: Option<&str>) -> SearchHitDto {
        SearchHitDto { id: s.id.clone(), snippet: snippet.map(str::to_string), matched: "text", file: None }
    }

    #[test]
    fn filters_by_range_and_repository() {
        let (a, b, c) = (sess("a", "org/a", 0), sess("b", "org/b", 3), sess("c", "org/a", 20));
        let all = [a, b, c];
        assert_eq!(ids(&rows(&all, None, Range::All, None, None)), ["a", "b", "c"]);
        assert_eq!(ids(&rows(&all, None, Range::Today, None, None)), ["a"]);
        assert_eq!(ids(&rows(&all, None, Range::Week, None, None)), ["a", "b"]);
        assert_eq!(ids(&rows(&all, None, Range::All, Some("org/a"), None)), ["a", "c"]);
    }

    #[test]
    fn search_hits_replace_the_whole_list_and_keep_their_snippet() {
        let (a, b, c) = (sess("a", "org/a", 0), sess("b", "org/b", 3), sess("c", "org/a", 20));
        let h = hit(&c, Some("x \u{2}y\u{3}"));
        let all = [a, b, c.clone()];
        let r = rows(&all, Some(std::slice::from_ref(&h)), Range::All, None, None);
        assert_eq!(ids(&r), ["c"]);
        assert_eq!(r[0].snippet(), Some("x \u{2}y\u{3}"));
        assert!(rows(&all, Some(&[hit(&c, None)]), Range::Week, None, None).is_empty());
    }

    #[test]
    fn repos_count_in_range_busiest_first() {
        let all = [sess("a", "org/a", 0), sess("b", "org/b", 3), sess("c", "org/a", 20)];
        let names = |r: Vec<RepoCount>| r.into_iter().map(|r| (r.name, r.count)).collect::<Vec<_>>();
        assert_eq!(names(repos(&all, Range::All, now(), None)), [("org/a".into(), 2), ("org/b".into(), 1)]);
        assert_eq!(names(repos(&all, Range::Today, now(), None)), [("org/a".into(), 1)]);
    }

    #[test]
    fn range_starts_at_local_midnight() {
        let d = |y, m, day| NaiveDate::from_ymd_opt(y, m, day).unwrap();
        assert_eq!(range_start(Range::Today, now()), Some(d(2026, 10, 13)));
        assert_eq!(range_start(Range::Week, now()), Some(d(2026, 10, 7)));
        assert_eq!(range_start(Range::All, now()), None);
    }

    #[test]
    fn agent_filter_and_counts() {
        let claude = |id: &str, repo: &str, days| {
            session(id, |s| {
                s.agent = Agent::ClaudeCode;
                s.repository = Some(repo.into());
                s.updated_at = Some(at(days));
            })
        };
        let all = [sess("a", "org/a", 0), sess("b", "org/b", 3), sess("c", "org/a", 20), claude("claude:x", "org/a", 1), claude("claude:old", "org/b", 40)];
        let both = [Agent::Copilot, Agent::ClaudeCode];
        assert_eq!(ids(&rows(&all, None, Range::All, None, Some(Agent::ClaudeCode))), ["claude:x", "claude:old"]);
        assert_eq!(ids(&rows(&all, None, Range::Week, Some("org/a"), Some(Agent::ClaudeCode))), ["claude:x"]);
        assert_eq!(agent_counts(&all, &both, Range::All, None, now()), [(Agent::Copilot, 3), (Agent::ClaudeCode, 2)]);
        assert_eq!(agent_counts(&all, &both, Range::Today, Some("org/a"), now()), [(Agent::Copilot, 1), (Agent::ClaudeCode, 0)]);
        let r = repos(&all, Range::All, now(), Some(Agent::ClaudeCode));
        assert_eq!(r.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(), ["org/a", "org/b"]);
        let counts = range_counts(&all, now());
        assert_eq!((counts[&Range::All], counts[&Range::Today], counts[&Range::Week]), (5, 1, 3));
    }
}
