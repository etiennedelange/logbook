use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Metadata describing a single GitHub Copilot CLI session, as stored under
/// `~/.copilot/session-state/{id}/`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRecord {
    pub id: String,
    pub name: Option<String>,
    pub user_named: bool,
    pub cwd: Option<String>,
    pub git_root: Option<String>,
    pub repository: Option<String>,
    pub branch: Option<String>,
    pub client_name: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub message_count: i64,
    /// The first thing the user asked, whitespace-collapsed and cut to `FIRST_PROMPT_MAX` chars.
    pub first_prompt: Option<String>,
    /// Full path to the session folder.
    pub path: String,
}

impl SessionRecord {
    /// Display name: the session's name, else its first prompt, else the id.
    pub fn display_name(&self) -> &str {
        [&self.name, &self.first_prompt]
            .into_iter()
            .flatten()
            .find(|n| !n.trim().is_empty())
            .map_or(&self.id, String::as_str)
    }
}

/// Longest first prompt kept on a `SessionRecord`.
pub const FIRST_PROMPT_MAX: usize = 200;

/// A single user/assistant message extracted from a session's events.jsonl.
#[derive(Debug, Clone, Serialize)]
pub struct TimelineEntry {
    pub role: String,
    pub timestamp: Option<DateTime<Utc>>,
    pub text: String,
}

/// A session search hit, with an optional highlighted snippet of matching text.
#[derive(Debug, Clone)]
pub struct SessionSearchResult {
    pub session: SessionRecord,
    pub snippet: Option<String>,
}
