use std::path::Path;
use std::sync::atomic::Ordering;

use serde::Serialize;
use logbook_core::dto::{SearchHitDto, SessionDto, TimelineEntryDto};
use logbook_core::models::Agent;
use logbook_core::source::Resume;
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

use crate::AppState;

type CmdResult<T> = Result<T, String>;

fn err<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

/// The UI filters sessions and search hits by time range and repository itself, so both
/// lists come back whole by default: cutting them short first would hide real matches.
const NO_LIMIT: usize = usize::MAX;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexStatus {
    pub indexing: bool,
    pub total: usize,
    pub scanned: usize,
    pub skipped: usize,
    /// Changes whenever the index does; the UI refetches sessions only when it moves.
    pub generation: u64,
}

#[tauri::command]
pub fn index_status(state: State<'_, AppState>) -> IndexStatus {
    let stats = state.service.stats();
    IndexStatus {
        indexing: state.indexing.load(Ordering::SeqCst),
        // What the list shows, so the footer agrees with "All sessions".
        total: state.service.listed_count(),
        scanned: stats.scanned,
        skipped: stats.skipped,
        generation: state.service.generation(),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceInfo {
    pub agent: Agent,
    /// Where the agent keeps its sessions. Logbook reads these and never writes to them.
    pub dir: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: String,
    pub sources: Vec<SourceInfo>,
    /// Logbook's own index cache: derived, safe to delete.
    pub cache_file: String,
}

/// What the About panel shows: the version and where Logbook reads and keeps its data.
#[tauri::command]
pub fn app_info(app: AppHandle, state: State<'_, AppState>) -> AppInfo {
    AppInfo {
        version: app.package_info().version.to_string(),
        sources: state
            .service
            .sources()
            .into_iter()
            .flat_map(|(agent, roots)| roots.into_iter().map(move |r| SourceInfo { agent, dir: r.display().to_string() }))
            .collect(),
        cache_file: logbook_core::paths::index_cache_path().display().to_string(),
    }
}

/// Starts an incremental re-scan; `index-updated` fires when it finishes.
#[tauri::command]
pub fn reindex_now(app: AppHandle) {
    crate::reindex(&app);
}

#[tauri::command]
pub async fn recent_sessions(state: State<'_, AppState>, limit: Option<usize>) -> CmdResult<Vec<SessionDto>> {
    Ok(state.service.recent(limit.unwrap_or(NO_LIMIT)))
}

#[tauri::command]
pub async fn search_sessions(
    state: State<'_, AppState>,
    q: String,
    limit: Option<usize>,
) -> CmdResult<Vec<SearchHitDto>> {
    Ok(state.service.search(&q, limit.unwrap_or(NO_LIMIT)))
}

#[tauri::command]
pub async fn get_session(state: State<'_, AppState>, id: String) -> CmdResult<Option<SessionDto>> {
    Ok(state.service.get(&id))
}

#[tauri::command]
pub async fn get_transcript(state: State<'_, AppState>, id: String) -> CmdResult<Vec<TimelineEntryDto>> {
    state.service.transcript(&id).ok_or_else(|| format!("session {id} not found"))
}

#[tauri::command]
pub async fn resume_command(state: State<'_, AppState>, id: String) -> CmdResult<String> {
    state.service.resume_command(&id).ok_or_else(|| format!("session {id} not found"))
}

/// Opens a new terminal in the session's working folder running the agent's resume command.
#[tauri::command]
pub async fn resume_session(state: State<'_, AppState>, id: String) -> CmdResult<()> {
    let session = state.service.get(&id).ok_or_else(|| format!("session {id} not found"))?;
    if session.source_deleted_at.is_some() {
        return Err("This session’s file was deleted, so it can’t be resumed".into());
    }
    let resume = state.service.resume(&id).ok_or_else(|| {
        format!("“{}” isn’t a valid session id, so it can’t be resumed from here", session.native_id)
    })?;
    launch_terminal(&resume).map_err(err)
}

#[cfg(windows)]
fn launch_terminal(resume: &Resume) -> std::io::Result<()> {
    use std::process::Command;
    let dir = resume
        .cwd
        .clone()
        .filter(|c| Path::new(c).is_dir())
        .unwrap_or_else(|| std::env::var("USERPROFILE").unwrap_or_else(|_| ".".into()));
    // Passed as separate arguments, never as one shell string.
    let command = std::iter::once(resume.program.as_str()).chain(resume.args.iter().map(String::as_str));

    // Windows Terminal when installed, otherwise a plain console window.
    let wt = Command::new("wt.exe").args(["-d", &dir, "cmd", "/k"]).args(command.clone()).spawn();
    if wt.is_ok() {
        return Ok(());
    }
    Command::new("cmd")
        .args(["/c", "start", "", "cmd", "/k"])
        .args(command)
        .current_dir(&dir)
        .spawn()
        .map(|_| ())
}

#[cfg(not(windows))]
fn launch_terminal(_resume: &Resume) -> std::io::Result<()> {
    Err(std::io::Error::other("opening a terminal is only supported on Windows; copy the command instead"))
}

/// Opens the session's working folder in the system file manager.
#[tauri::command]
pub async fn open_folder(app: AppHandle, path: String) -> CmdResult<()> {
    if !Path::new(&path).is_dir() {
        return Err(format!("{path} no longer exists"));
    }
    app.opener().open_path(path, None::<&str>).map_err(err)
}

/// Opens a link from a transcript in the default browser (web links only).
#[tauri::command]
pub async fn open_link(app: AppHandle, url: String) -> CmdResult<()> {
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err(format!("only web links can be opened: {url}"));
    }
    app.opener().open_url(url, None::<&str>).map_err(err)
}
