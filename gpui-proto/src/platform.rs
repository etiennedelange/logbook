//! What the app does to the outside world: open a terminal to resume a session, open folders and links,
//! remember the theme and zoom, and read `latest.json`. The same behaviour as `src-tauri/src/commands.rs`.

use logbook_core::source::Resume;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Opens a new terminal in the session's working folder running the agent's resume command.
#[cfg(windows)]
pub fn launch_terminal(resume: &Resume) -> std::io::Result<()> {
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
    Command::new("cmd").args(["/c", "start", "", "cmd", "/k"]).args(command).current_dir(&dir).spawn().map(|_| ())
}

#[cfg(not(windows))]
pub fn launch_terminal(_resume: &Resume) -> std::io::Result<()> {
    Err(std::io::Error::other("opening a terminal is only supported on Windows; copy the command instead"))
}

/// Opens a folder in the file manager.
pub fn open_folder(path: &str) -> Result<(), String> {
    if !Path::new(path).is_dir() {
        return Err(format!("{path} no longer exists"));
    }
    let program = if cfg!(windows) {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    std::process::Command::new(program).arg(path).spawn().map(|_| ()).map_err(|e| e.to_string())
}

/// Web links only: a transcript's text can hold anything.
pub fn check_link(url: &str) -> Result<(), String> {
    if url.starts_with("https://") || url.starts_with("http://") {
        Ok(())
    } else {
        Err(format!("only web links can be opened: {url}"))
    }
}

#[derive(Serialize, Deserialize, Default, Clone, Copy)]
pub struct Settings {
    /// `None`: no choice yet, so the system's.
    pub dark: Option<bool>,
    pub zoom: Option<f32>,
}

fn settings_path() -> PathBuf {
    let base = std::env::var_os("LOGBOOK_CACHE_DIR").map(PathBuf::from).or_else(|| dirs::data_local_dir().map(|d| d.join("Logbook")));
    base.unwrap_or_else(|| PathBuf::from(".")).join("gpui-settings.json")
}

pub fn load_settings() -> Settings {
    std::fs::read_to_string(settings_path()).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

pub fn save_settings(s: &Settings) {
    let path = settings_path();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).ok();
    }
    if let Ok(json) = serde_json::to_string(s) {
        std::fs::write(path, json).ok();
    }
}

pub const ZOOM_MIN: f32 = 0.5;
pub const ZOOM_MAX: f32 = 2.0;

pub fn clamp_zoom(z: f32) -> f32 {
    if z.is_finite() { z.clamp(ZOOM_MIN, ZOOM_MAX) } else { 1.0 }
}

/// Exponential step: one mouse-wheel notch (|delta| = 100) is about 10%.
pub fn next_zoom(current: f32, delta_y: f32) -> f32 {
    clamp_zoom(current * (-delta_y / 1000.0).exp())
}

/// Asks GitHub for the latest release's `latest.json`.
pub fn fetch_latest() -> Result<String, String> {
    let mut resp = ureq::get(logbook_gpui_proto::update::LATEST_JSON)
        .call()
        .map_err(|e| format!("Couldn't check for updates: {e}"))?;
    resp.body_mut().read_to_string().map_err(|e| format!("Couldn't check for updates: {e}"))
}
