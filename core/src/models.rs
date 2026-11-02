use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A coding agent whose sessions Logbook reads. Each has one `source::Source`. On the wire
/// (and in the UI's `Agent` type) each is its `key`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Agent {
    #[serde(rename = "copilot")]
    Copilot,
    #[serde(rename = "claude")]
    ClaudeCode,
}

impl Agent {
    /// Short, stable key: the prefix of every session id, and the agent's name on the wire.
    pub fn key(self) -> &'static str {
        match self {
            Agent::Copilot => "copilot",
            Agent::ClaudeCode => "claude",
        }
    }

    /// The id Logbook gives a session: `<agent>:<native id>`, so two agents' ids can't collide.
    pub fn session_id(self, native_id: &str) -> String {
        format!("{}:{native_id}", self.key())
    }
}

/// Metadata describing a single agent session, whichever agent wrote it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRecord {
    /// `<agent>:<native id>` (see `Agent::session_id`).
    pub id: String,
    pub agent: Agent,
    /// The agent's own id for the session, as its resume command takes it.
    pub native_id: String,
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
    /// Full path to the session's folder or file.
    pub path: String,
    /// When Logbook found the session's file gone (an agent deleted it). The session is then
    /// served from Logbook's own kept copy and can't be resumed.
    pub source_deleted_at: Option<DateTime<Utc>>,
}

impl SessionRecord {
    /// Display name: the session's name, else its first prompt, else the agent's id for it.
    pub fn display_name(&self) -> &str {
        [&self.name, &self.first_prompt]
            .into_iter()
            .flatten()
            .find(|n| !n.trim().is_empty())
            .map_or(&self.native_id, String::as_str)
    }
}

/// Longest first prompt kept on a `SessionRecord`.
pub const FIRST_PROMPT_MAX: usize = 200;

/// A single user/assistant message from a session's transcript.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agents_go_over_the_wire_as_their_key() {
        for agent in [Agent::Copilot, Agent::ClaudeCode] {
            assert_eq!(serde_json::to_string(&agent).unwrap(), format!("\"{}\"", agent.key()));
        }
    }
}
