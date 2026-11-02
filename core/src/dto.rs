//! JSON-serializable view types for the HTTP API. These mirror the internal
//! `SessionRecord`/`TimelineEntry`/`SessionSearchResult` models but keep the wire
//! format (camelCase field names, RFC3339 date strings) independent of internal
//! representations.

use crate::models::{Agent, SessionRecord, SessionSearchResult, TimelineEntry};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionDto {
    /// `<agent>:<native id>`: what every other command takes.
    pub id: String,
    pub agent: Agent,
    /// The agent's own id for the session, as shown to the user.
    pub native_id: String,
    pub name: Option<String>,
    pub user_named: bool,
    pub cwd: Option<String>,
    pub git_root: Option<String>,
    pub repository: Option<String>,
    pub branch: Option<String>,
    pub client_name: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub message_count: i64,
    pub first_prompt: Option<String>,
    pub path: String,
    pub source_deleted_at: Option<String>,
}

impl From<SessionRecord> for SessionDto {
    fn from(record: SessionRecord) -> Self {
        Self {
            id: record.id,
            agent: record.agent,
            native_id: record.native_id,
            name: record.name,
            user_named: record.user_named,
            cwd: record.cwd,
            git_root: record.git_root,
            repository: record.repository,
            branch: record.branch,
            client_name: record.client_name,
            created_at: record.created_at.map(|d| d.to_rfc3339()),
            updated_at: record.updated_at.map(|d| d.to_rfc3339()),
            message_count: record.message_count,
            first_prompt: record.first_prompt,
            path: record.path,
            source_deleted_at: record.source_deleted_at.map(|d| d.to_rfc3339()),
        }
    }
}

/// A search hit as the UI receives it: just the session id and snippet, since the UI already
/// holds every session. Sending whole sessions made a one-letter query a ~600 KB payload.
#[derive(Debug, Clone, Serialize)]
pub struct SearchHitDto {
    pub id: String,
    pub snippet: Option<String>,
}

impl From<SessionSearchResult> for SearchHitDto {
    fn from(result: SessionSearchResult) -> Self {
        Self {
            id: result.session.id,
            snippet: result.snippet,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineEntryDto {
    pub role: String,
    pub timestamp: Option<String>,
    pub text: String,
}

impl From<TimelineEntry> for TimelineEntryDto {
    fn from(entry: TimelineEntry) -> Self {
        Self {
            role: entry.role,
            timestamp: entry.timestamp.map(|d| d.to_rfc3339()),
            text: entry.text,
        }
    }
}
