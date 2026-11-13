//! Everything the screens show, worked out as plain data: a port of `ui/src/model/*.ts`. No GPUI in
//! here, so it is unit-tested on any platform and the views stay thin.

pub mod agents;
pub mod format;
pub mod select;
pub mod stats;

use chrono::{DateTime, Local};
use logbook_core::dto::SessionDto;
use std::ops::Deref;
use std::sync::Arc;

/// A session as the UI holds it: the backend's record plus what every filter and row needs, worked
/// out once when the list is fetched rather than on every filter click and frame.
pub struct Session {
    pub dto: SessionDto,
    pub updated: Option<DateTime<Local>>,
    pub created: Option<DateTime<Local>>,
    /// When the session was last active (else when it started), in ms since the epoch.
    pub time_ms: Option<i64>,
    pub repo: Option<String>,
    pub title: format::Title,
}

impl Deref for Session {
    type Target = SessionDto;
    fn deref(&self) -> &SessionDto {
        &self.dto
    }
}

fn parse(iso: &Option<String>) -> Option<DateTime<Local>> {
    iso.as_deref().and_then(|s| DateTime::parse_from_rfc3339(s).ok()).map(|d| d.with_timezone(&Local))
}

impl Session {
    pub fn new(dto: SessionDto) -> Self {
        let updated = parse(&dto.updated_at);
        let created = parse(&dto.created_at);
        Self {
            time_ms: updated.or(created).map(|d| d.timestamp_millis()),
            repo: format::repo_of(&dto),
            title: format::session_title(&dto),
            updated,
            created,
            dto,
        }
    }
}

/// Wraps a fetched list. The order is the backend's (most recently active first).
pub fn load(dtos: Vec<SessionDto>) -> Vec<Arc<Session>> {
    dtos.into_iter().map(|d| Arc::new(Session::new(d))).collect()
}

#[cfg(test)]
pub(crate) mod test_support {
    use super::*;
    use chrono::TimeZone;
    use logbook_core::models::Agent;

    pub fn dto(id: &str) -> SessionDto {
        SessionDto {
            id: id.into(),
            agent: Agent::Copilot,
            native_id: id.into(),
            name: None,
            user_named: false,
            cwd: None,
            git_root: None,
            repository: None,
            branch: None,
            client_name: None,
            created_at: None,
            updated_at: None,
            message_count: 1,
            first_prompt: None,
            path: String::new(),
            source_deleted_at: None,
            files_created: 0,
            files_edited: 0,
        }
    }

    pub fn session(id: &str, f: impl FnOnce(&mut SessionDto)) -> Arc<Session> {
        let mut d = dto(id);
        f(&mut d);
        Arc::new(Session::new(d))
    }

    pub fn local(y: i32, m: u32, d: u32, h: u32, min: u32) -> DateTime<Local> {
        Local.with_ymd_and_hms(y, m, d, h, min, 0).earliest().unwrap()
    }

    pub fn iso(d: DateTime<Local>) -> String {
        d.to_rfc3339()
    }
}
