//! Text and time formatting, and the search-word matching used to mark transcripts and snippets.
//! A port of `ui/src/model/format.ts`.

use chrono::{DateTime, Datelike, Local, NaiveDate};
use logbook_core::dto::SessionDto;

pub fn parse_iso(iso: &str) -> Option<DateTime<Local>> {
    DateTime::parse_from_rfc3339(iso).ok().map(|d| d.with_timezone(&Local))
}

pub fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// List time: "now", "2m", "1h", the weekday within a week, otherwise "Oct 7".
pub fn rel_time(iso: Option<&str>, now: DateTime<Local>) -> String {
    let Some(d) = iso.and_then(parse_iso) else { return String::new() };
    let diff = (now - d).num_milliseconds();
    const MIN: i64 = 60_000;
    const HOUR: i64 = 60 * MIN;
    const DAY: i64 = 24 * HOUR;
    if diff < MIN {
        "now".into()
    } else if diff < HOUR {
        format!("{}m", diff / MIN)
    } else if diff < DAY {
        format!("{}h", diff / HOUR)
    } else if diff < 7 * DAY {
        d.format("%a").to_string()
    } else if d.year() == now.year() {
        d.format("%b %-d").to_string()
    } else {
        d.format("%b %-d, %Y").to_string()
    }
}

/// A day: "Oct 28, 2026".
pub fn date_only(iso: &str) -> String {
    parse_iso(iso).map(|d| d.format("%b %-d, %Y").to_string()).unwrap_or_default()
}

/// Full timestamp: "Oct 7, 2026, 14:55".
pub fn date_time(iso: Option<&str>) -> String {
    iso.and_then(parse_iso).map(|d| d.format("%b %-d, %Y, %H:%M").to_string()).unwrap_or_else(|| "—".into())
}

/// Message time inside a transcript: "14:55".
pub fn clock(iso: Option<&str>) -> String {
    iso.and_then(parse_iso).map(|d| d.format("%H:%M").to_string()).unwrap_or_default()
}

pub fn basename(path: &str) -> String {
    let trimmed = path.trim_end_matches(['/', '\\']);
    trimmed.rsplit(['/', '\\']).next().filter(|s| !s.is_empty()).unwrap_or(path).to_string()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TitleSource {
    Name,
    Prompt,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Title {
    pub text: String,
    pub source: TitleSource,
}

/// What a session is called: its name, else what it was first asked, else "Untitled session".
pub fn session_title(s: &SessionDto) -> Title {
    if let Some(name) = s.name.as_deref().map(str::trim).filter(|n| !n.is_empty()) {
        return Title { text: name.to_string(), source: TitleSource::Name };
    }
    if let Some(p) = &s.first_prompt {
        let cleaned = p.replace('`', "").replace("**", "");
        let cleaned = cleaned.trim_start_matches(|c: char| c == '#' || c == '>' || c.is_whitespace());
        let text = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
        if !text.is_empty() {
            return Title { text, source: TitleSource::Prompt };
        }
    }
    Title { text: "Untitled session".into(), source: TitleSource::None }
}

/// Grouping key for the sidebar: the repository, else the git root or working folder's name.
pub fn repo_of(s: &SessionDto) -> Option<String> {
    if let Some(r) = s.repository.as_deref().filter(|r| !r.trim().is_empty()) {
        return Some(r.to_string());
    }
    s.git_root.as_deref().or(s.cwd.as_deref()).map(basename)
}

/// "Org/Project/Repo" shows as "Repo"; the full name goes in the tooltip.
pub fn short_repo(repo: &str) -> &str {
    repo.rsplit('/').next().filter(|s| !s.is_empty()).unwrap_or(repo)
}

fn day(d: DateTime<Local>) -> NaiveDate {
    d.date_naive()
}

/// List section for a session: "Today", "Yesterday", the weekday within a week, then month and year.
pub fn day_group(iso: Option<&str>, now: DateTime<Local>) -> String {
    let Some(d) = iso.and_then(parse_iso) else { return "Undated".into() };
    day_group_at(d, now)
}

pub fn day_group_at(d: DateTime<Local>, now: DateTime<Local>) -> String {
    let days = (day(now) - day(d)).num_days();
    match days {
        ..=0 => "Today".into(),
        1 => "Yesterday".into(),
        2..=6 => d.format("%A").to_string(),
        _ => d.format("%B %Y").to_string(),
    }
}

/// Search words as typed (same split as the Rust `fts_query::build`).
pub fn query_tokens(q: &str) -> Vec<String> {
    q.to_lowercase().split(|c: char| !is_word_char(c)).filter(|s| !s.is_empty()).map(str::to_string).collect()
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '-'
}

/// How a search matched, best first (mirrors `Rank` in core/src/index.rs): the words in order as
/// typed, every word whole somewhere, or only as word prefixes ("gets" in "getSpecification").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchMode {
    Phrase,
    Words,
    Prefix,
}

/// Alternating separator/word parts, like JS `split(/([\p{L}\p{N}_-]+)/u)`: even indexes are
/// separators (possibly empty), odd indexes are words, and the list starts and ends with a separator.
fn split_words(text: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut in_word = false;
    for (i, c) in text.char_indices() {
        let w = is_word_char(c);
        if w != in_word {
            parts.push(&text[start..i]);
            start = i;
            in_word = w;
        }
    }
    parts.push(&text[start..]);
    if in_word {
        parts.push("");
    }
    parts
}

fn is_phrase_at(lower: &[String], i: usize, tokens: &[String]) -> bool {
    tokens.iter().enumerate().all(|(j, t)| match lower.get(i + 2 * j) {
        Some(w) => {
            if j == tokens.len() - 1 {
                w.starts_with(t.as_str())
            } else {
                w == t
            }
        }
        None => false,
    })
}

/// The best way `tokens` match `text`, or `None` when they don't all match.
pub fn match_mode(text: &str, tokens: &[String]) -> Option<MatchMode> {
    if tokens.is_empty() {
        return None;
    }
    let lower: Vec<String> = split_words(text).iter().map(|p| p.to_lowercase()).collect();
    let words = lower.iter().enumerate().filter(|(i, _)| i % 2 == 1).map(|(_, w)| w.as_str());
    if tokens.len() > 1 && (0..lower.len()).filter(|i| i % 2 == 1).any(|i| is_phrase_at(&lower, i, tokens)) {
        return Some(MatchMode::Phrase);
    }
    let words: Vec<&str> = words.collect();
    if tokens.iter().all(|t| words.contains(&t.as_str())) {
        return Some(MatchMode::Words);
    }
    if tokens.iter().all(|t| words.iter().any(|w| w.starts_with(t.as_str()))) {
        return Some(MatchMode::Prefix);
    }
    None
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    pub text: String,
    pub hit: bool,
}

fn seg(text: &str, hit: bool) -> Segment {
    Segment { text: text.to_string(), hit }
}

/// Splits text into runs with the search matches marked: whole phrases (separators included) in
/// `Phrase` mode, whole words in `Words` mode, word prefixes in `Prefix` mode.
pub fn highlight_words(text: &str, tokens: &[String], mode: Option<MatchMode>) -> Vec<Segment> {
    let Some(mode) = mode.filter(|_| !tokens.is_empty()) else { return vec![seg(text, false)] };
    let parts = split_words(text);
    let lower: Vec<String> = parts.iter().map(|p| p.to_lowercase()).collect();
    let mut hit = vec![false; parts.len()];
    for i in (1..parts.len()).step_by(2) {
        match mode {
            MatchMode::Phrase => {
                if is_phrase_at(&lower, i, tokens) {
                    for h in &mut hit[i..=i + 2 * (tokens.len() - 1)] {
                        *h = true;
                    }
                }
            }
            MatchMode::Words => hit[i] = tokens.contains(&lower[i]),
            MatchMode::Prefix => hit[i] = tokens.iter().any(|t| lower[i].starts_with(t.as_str())),
        }
    }
    let mut out: Vec<Segment> = Vec::new();
    for (i, p) in parts.iter().enumerate() {
        if p.is_empty() {
            continue;
        }
        match out.last_mut() {
            Some(last) if last.hit == hit[i] => last.text.push_str(p),
            _ => out.push(seg(p, hit[i])),
        }
    }
    out
}

/// Markdown syntax that only adds noise to a one-line snippet, and runs of whitespace as one space.
fn plain_snippet(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut at_line_start = true;
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        let next_line_start = c == '\n';
        match c {
            '`' => {
                while chars.peek() == Some(&'`') {
                    chars.next();
                }
            }
            '*' | '_' if chars.peek() == Some(&c) => {
                chars.next();
            }
            // `^#+\s` on a line of its own.
            '#' if at_line_start => {
                let mut rest = chars.clone();
                while rest.peek() == Some(&'#') {
                    rest.next();
                }
                if rest.peek().is_some_and(|c| c.is_whitespace()) {
                    rest.next();
                    chars = rest;
                } else {
                    out.push(c);
                }
            }
            c if c.is_whitespace() => {
                if !out.ends_with(' ') {
                    out.push(' ');
                }
            }
            c => out.push(c),
        }
        at_line_start = next_line_start;
    }
    out
}

/// Splits a search snippet on its STX/ETX markers into plain and highlighted runs, dropping markdown
/// syntax and joining marked words that are only separated by spaces into one run.
pub fn snippet_segments(snippet: &str) -> Vec<Segment> {
    let mut out: Vec<Segment> = Vec::new();
    let mut push = |text: &str, hit: bool| {
        if text.is_empty() {
            return;
        }
        match out.last_mut() {
            Some(last) if last.hit == hit => last.text.push_str(text),
            _ => out.push(seg(text, hit)),
        }
    };
    for part in snippet.split('\u{2}') {
        match part.find('\u{3}') {
            None => push(&plain_snippet(part), false),
            Some(end) => {
                push(&part[..end], true);
                push(&plain_snippet(&part[end + 1..]), false);
            }
        }
    }
    // "what" " " "gets" reads as one highlighted phrase.
    let joined: Vec<Segment> = out
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let between = !s.hit
                && s.text == " "
                && i > 0
                && out[i - 1].hit
                && out.get(i + 1).is_some_and(|n| n.hit);
            if between { seg(" ", true) } else { s.clone() }
        })
        .collect();
    let mut merged: Vec<Segment> = Vec::new();
    for s in joined {
        match merged.last_mut() {
            Some(last) if last.hit == s.hit => last.text.push_str(&s.text),
            _ => merged.push(s),
        }
    }
    merged
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::test_support::*;

    fn hits(segs: &[Segment]) -> Vec<&str> {
        segs.iter().filter(|s| s.hit).map(|s| s.text.as_str()).collect()
    }

    #[test]
    fn snippet_segments_split_on_markers() {
        assert_eq!(
            snippet_segments("... build the \u{2}dashboard\u{3} env \u{2}config\u{3}"),
            vec![seg("... build the ", false), seg("dashboard", true), seg(" env ", false), seg("config", true)]
        );
        assert_eq!(snippet_segments("plain"), vec![seg("plain", false)]);
    }

    #[test]
    fn snippets_drop_markdown_and_join_a_marked_phrase() {
        assert_eq!(
            snippet_segments("... **\u{2}What\u{3} \u{2}gets\u{3} \u{2}scheduled\u{3}:** only `a` key"),
            vec![seg("... ", false), seg("What gets scheduled", true), seg(": only a key", false)]
        );
    }

    #[test]
    fn rel_time_is_short_within_a_day_and_a_date_after_a_week() {
        let now = local(2026, 10, 13, 12, 0);
        let at = |d: DateTime<Local>| rel_time(Some(&iso(d)), now);
        assert_eq!(at(now - chrono::Duration::seconds(30)), "now");
        assert_eq!(at(now - chrono::Duration::minutes(45)), "45m");
        assert_eq!(at(now - chrono::Duration::hours(10)), "10h");
        assert_eq!(at(local(2026, 9, 1, 12, 0)), "Sep 1");
        assert_eq!(at(local(2025, 9, 1, 12, 0)), "Sep 1, 2025");
        assert_eq!(rel_time(None, now), "");
    }

    #[test]
    fn titles_fall_back_to_the_first_prompt_then_untitled() {
        let t = |f: fn(&mut SessionDto)| session_title(&session("a", f).dto);
        assert_eq!(t(|s| s.name = Some("Named".into())), Title { text: "Named".into(), source: TitleSource::Name });
        let p = t(|s| {
            s.name = Some("  ".into());
            s.first_prompt = Some("Why is CI red?".into());
        });
        assert_eq!(p, Title { text: "Why is CI red?".into(), source: TitleSource::Prompt });
        assert_eq!(t(|_| {}).text, "Untitled session");
        assert_eq!(t(|s| s.first_prompt = Some("What does `git rebase` do?".into())).text, "What does git rebase do?");
        assert_eq!(t(|s| s.first_prompt = Some("## **Fix** the bug".into())).text, "Fix the bug");
    }

    #[test]
    fn repos_group_by_repository_else_the_folder_name() {
        assert_eq!(repo_of(&session("a", |s| s.repository = Some("org/app".into())).dto).as_deref(), Some("org/app"));
        assert_eq!(repo_of(&session("a", |s| s.cwd = Some("C:\\Repos\\thing\\".into())).dto).as_deref(), Some("thing"));
        assert_eq!(repo_of(&session("a", |_| {}).dto), None);
        assert_eq!(basename("/home/me/proj"), "proj");
        assert_eq!(short_repo("Org/Project/Repo.Name"), "Repo.Name");
        assert_eq!(short_repo("solo"), "solo");
    }

    #[test]
    fn words_starting_with_a_token_are_marked() {
        let segs = highlight_words("Build the dashboard, then dash off", &query_tokens("DASH"), Some(MatchMode::Prefix));
        assert_eq!(hits(&segs), ["dashboard", "dash"]);
        assert_eq!(segs.iter().map(|s| s.text.as_str()).collect::<String>(), "Build the dashboard, then dash off");
        assert_eq!(highlight_words("plain text", &[], Some(MatchMode::Prefix)), vec![seg("plain text", false)]);
    }

    #[test]
    fn phrase_search_prefers_the_phrase_then_words_then_prefixes() {
        let tokens = query_tokens("What gets sched");
        let text = "So what now? **What gets scheduled:** only a key. It gets what it needs.";
        assert_eq!(match_mode(text, &tokens), Some(MatchMode::Phrase));
        assert_eq!(match_mode("what is scheduled, it gets done", &query_tokens("what gets scheduled")), Some(MatchMode::Words));
        assert_eq!(match_mode("what getSpecification scheduled", &query_tokens("what gets scheduled")), Some(MatchMode::Prefix));
        assert_eq!(match_mode("nothing here", &tokens), None);
        let segs = highlight_words(text, &tokens, Some(MatchMode::Phrase));
        assert_eq!(hits(&segs), ["What gets scheduled"]);
    }

    #[test]
    fn day_groups_name_recent_days_and_fall_back_to_the_month() {
        let now = local(2026, 10, 13, 12, 0);
        let g = |d| day_group_at(d, now);
        assert_eq!(g(local(2026, 10, 13, 1, 0)), "Today");
        assert_eq!(g(local(2026, 10, 12, 23, 0)), "Yesterday");
        assert_eq!(g(local(2026, 10, 9, 0, 0)), "Friday");
        assert_eq!(g(local(2026, 8, 2, 0, 0)), "August 2026");
        assert_eq!(day_group(None, now), "Undated");
    }

    #[test]
    fn split_words_alternates_separators_and_words() {
        assert_eq!(split_words("a, b"), vec!["", "a", ", ", "b", ""]);
        assert_eq!(split_words(""), vec![""]);
        assert_eq!(split_words("--"), vec!["", "--", ""]);
        assert_eq!(split_words("x!"), vec!["", "x", "!"]);
    }
}
