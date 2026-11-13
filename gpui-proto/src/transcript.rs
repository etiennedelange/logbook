//! A transcript as the detail pane draws it: messages, which of them fold, and which match the search.

use crate::model::format::{self, highlight_words, match_mode, MatchMode};
use chrono::{DateTime, Local};
use logbook_core::dto::TimelineEntryDto;

/// Messages longer than this start folded, unless they contain a search match.
pub const FOLD_CHARS: usize = 1600;

/// A message longer than this is cut, so one huge tool output can't stall layout.
pub const MESSAGE_MAX_CHARS: usize = 100_000;

#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    pub user: bool,
    pub ts: Option<String>,
    pub text: String,
    /// Longer than `FOLD_CHARS`: it starts folded (worked out once, not per frame).
    pub long: bool,
    /// Its length in characters, for "Show full message (4k characters)".
    pub chars: usize,
    /// The day heading above this message, when it starts a new day (never for the first message).
    pub day: Option<String>,
}

/// Cuts at a char boundary, `max` characters in; returns the cut text and how many characters went.
fn cut(text: &str, max: usize) -> Option<(&str, usize)> {
    text.char_indices().nth(max).map(|(i, _)| (&text[..i], text[i..].chars().count()))
}

pub fn messages(entries: &[TimelineEntryDto], now: DateTime<Local>) -> Vec<Message> {
    let mut last_day = String::new();
    entries
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let text = match cut(&e.text, MESSAGE_MAX_CHARS) {
                Some((head, more)) => format!("{head}\n\n… (cut: {more} more characters)"),
                None => e.text.clone(),
            };
            let day = e.timestamp.as_deref().map(|t| format::day_group(Some(t), now)).filter(|d| !d.is_empty());
            let show = day.as_ref().filter(|d| **d != last_day && i > 0).cloned();
            if let Some(d) = &day {
                last_day = d.clone();
            }
            Message { user: e.role == "user", ts: e.timestamp.clone(), long: is_long(&text), chars: text.chars().count(), text, day: show }
        })
        .collect()
}

/// The start of a long message: its first `FOLD_CHARS`, back to a word boundary, then "…".
pub fn folded(text: &str) -> String {
    let head = cut(text, FOLD_CHARS).map_or(text, |(h, _)| h);
    let trimmed = match head.rfind(char::is_whitespace) {
        // `/\s+\S*$/`: drop the last (partial) word and the space before it.
        Some(i) => head[..i].trim_end(),
        None => head,
    };
    format!("{trimmed}…")
}

pub fn is_long(text: &str) -> bool {
    text.chars().nth(FOLD_CHARS).is_some()
}

/// How a search lands on a transcript.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Marks {
    /// The search tokens joined by spaces: what these marks answer.
    pub key: String,
    pub tokens: Vec<String>,
    /// One mode for the whole conversation, so a phrase search marks the phrase, not every "what".
    pub mode: Option<MatchMode>,
    pub hit_at: Vec<bool>,
    pub matches: usize,
    pub first: Option<usize>,
}

pub fn marks(messages: &[Message], query: &str) -> Marks {
    let tokens = format::query_tokens(query);
    if tokens.is_empty() {
        return Marks::default();
    }
    let all: String = messages.iter().map(|m| m.text.as_str()).collect::<Vec<_>>().join("\n");
    let mode = match_mode(&all, &tokens);
    let hit_at: Vec<bool> = messages.iter().map(|m| highlight_words(&m.text, &tokens, mode).iter().any(|s| s.hit)).collect();
    Marks {
        key: tokens.join(" "),
        matches: hit_at.iter().filter(|h| **h).count(),
        first: hit_at.iter().position(|h| *h),
        tokens,
        mode,
        hit_at,
    }
}

/// The latest of your messages that has scrolled up out of view (every message before `top`), so it can
/// stay pinned above the replies to it. `None` while there is no such message.
pub fn pinned_prompt(messages: &[Message], top: usize) -> Option<usize> {
    messages[..top.min(messages.len())].iter().rposition(|m| m.user)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::test_support::*;

    fn entry(role: &str, ts: DateTime<Local>, text: &str) -> TimelineEntryDto {
        TimelineEntryDto { role: role.into(), timestamp: Some(iso(ts)), text: text.into() }
    }

    #[test]
    fn day_headings_appear_when_the_day_changes_but_never_above_the_first_message() {
        let now = local(2026, 11, 3, 12, 0);
        let out = messages(
            &[
                entry("user", local(2026, 11, 2, 9, 0), "a"),
                entry("assistant", local(2026, 11, 2, 9, 1), "b"),
                entry("user", local(2026, 11, 3, 9, 0), "c"),
            ],
            now,
        );
        assert_eq!(out.iter().map(|m| m.day.as_deref()).collect::<Vec<_>>(), [None, None, Some("Today")]);
        assert_eq!(out.iter().map(|m| m.user).collect::<Vec<_>>(), [true, false, true]);
    }

    #[test]
    fn huge_messages_are_cut_on_a_char_boundary() {
        let out = messages(&[entry("assistant", local(2026, 11, 3, 9, 0), &"é".repeat(MESSAGE_MAX_CHARS + 5))], local(2026, 11, 3, 12, 0));
        assert!(out[0].text.ends_with("(cut: 5 more characters)"));
    }

    #[test]
    fn folding_keeps_whole_words() {
        let text = "word ".repeat(1000);
        assert!(is_long(&text));
        let f = folded(&text);
        assert!(f.ends_with("word…"), "{}", &f[f.len() - 12..]);
        assert!(f.chars().count() <= FOLD_CHARS + 1);
        assert!(!is_long("short"));
    }

    #[test]
    fn marks_find_the_matching_messages_with_one_mode_for_all() {
        let now = local(2026, 11, 3, 12, 0);
        let m = messages(
            &[
                entry("user", now, "the reminder queue"),
                entry("assistant", now, "nothing here"),
                entry("assistant", now, "the reminder again"),
            ],
            now,
        );
        let k = marks(&m, "reminder");
        assert_eq!((k.hit_at.as_slice(), k.matches, k.first), ([true, false, true].as_slice(), 2, Some(0)));
        assert_eq!(k.mode, Some(MatchMode::Words));
        assert_eq!(marks(&m, "  ").matches, 0);
        assert!(marks(&m, "").key.is_empty());
    }

    #[test]
    fn the_pinned_prompt_is_the_latest_user_message_above_the_top() {
        let now = local(2026, 11, 3, 12, 0);
        let m = messages(
            &[entry("user", now, "a"), entry("assistant", now, "b"), entry("user", now, "c"), entry("assistant", now, "d")],
            now,
        );
        assert_eq!(pinned_prompt(&m, 0), None);
        assert_eq!(pinned_prompt(&m, 1), Some(0));
        assert_eq!(pinned_prompt(&m, 3), Some(2));
        assert_eq!(pinned_prompt(&m, 99), Some(2));
    }
}
