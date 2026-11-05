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
    /// The files the agent created or edited with its own file tools, in the order it first
    /// touched them. Empty for agents whose edits Logbook doesn't read yet.
    pub files: Vec<FileChange>,
    /// Shell commands the agent ran that look like they change files (`git mv`, `rm`, `sed -i`,
    /// a formatter ...). Logbook can't tell which files they changed, so it only lists them.
    pub commands: Vec<String>,
}

/// One file a session created or edited, summed over all of its edits.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileChange {
    /// Relative to the session's folder when inside it, else as the agent wrote it.
    pub path: String,
    /// The session's first write to it created it.
    pub created: bool,
    pub edits: u32,
    pub added: u32,
    pub removed: u32,
}

/// One edit to a file, for the detail pane's diff. Read from the agent's file on demand.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FileEdit {
    pub timestamp: Option<DateTime<Utc>>,
    pub created: bool,
    pub hunks: Vec<Hunk>,
    /// Lines were left out of a very large edit.
    pub truncated: bool,
}

/// A run of changed lines with context, as a unified diff has them: each line starts with
/// ` `, `+` or `-`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Hunk {
    pub old_start: u32,
    pub new_start: u32,
    pub lines: Vec<String>,
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
    /// What the snippet comes from.
    pub matched: Matched,
}

/// Where a search hit's snippet comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Matched {
    /// The transcript, or the session's name, repository, branch or folder.
    Text,
    /// A file the session changed; the snippet is its path.
    File(FileChange),
    /// The session's id; the snippet is the id.
    Session,
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
