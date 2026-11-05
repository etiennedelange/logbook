use dirs;
use std::path::PathBuf;

/// Root folder holding one subdirectory per Copilot CLI session:
/// `~/.copilot/session-state/{id}/`.
///
/// Honors a `COPILOT_HOME` environment variable override (pointing at the
/// `.copilot`-equivalent folder itself) so tests and Docker packaging can point at a
/// temp directory instead of the real `~/.copilot`.
pub fn session_state_root() -> PathBuf {
    if let Ok(home) = std::env::var("COPILOT_HOME") {
        return PathBuf::from(home).join("session-state");
    }
    dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")).join(".copilot").join("session-state")
}

/// Claude Code's folder of transcripts, one subfolder per project: `~/.claude/projects/`.
/// `CLAUDE_CONFIG_DIR` moves `~/.claude`, as it does for Claude Code itself.
pub fn claude_projects_root() -> PathBuf {
    if let Ok(dir) = std::env::var("CLAUDE_CONFIG_DIR") {
        return PathBuf::from(dir).join("projects");
    }
    dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")).join(".claude").join("projects")
}

/// Where this tool keeps its own search index, kept separate from Copilot's own data.
pub fn app_data_dir() -> PathBuf {
    let dir = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from(".")).join("Logbook");
    std::fs::create_dir_all(&dir).ok();
    dir
}

/// The cache file name. Bump the version whenever `index::Entry` changes shape or the
/// encoding changes: postcard has no schema, so an old cache must never be read as the new
/// layout. v4 switched from bincode to postcard; v5 added the agent to each session, and kept transcripts; v6 the files each session changed.
const CACHE_FILE: &str = "index-v6.bin";

/// Path to this tool's own postcard-serialized index cache.
///
/// Honors a `LOGBOOK_CACHE_DIR` environment variable override so tests and
/// packaging can point at a temp directory instead of the real app data dir.
pub fn index_cache_path() -> PathBuf {
    if let Ok(dir) = std::env::var("LOGBOOK_CACHE_DIR") {
        let dir = PathBuf::from(dir);
        std::fs::create_dir_all(&dir).ok();
        return dir.join(CACHE_FILE);
    }

    let dir = app_data_dir();
    // One-time cleanup of earlier cache formats.
    for stale in [
        "index.db",
        "index.db-shm",
        "index.db-wal",
        "index.bin",
        "index-v2.bin",
        "index-v3.bin",
        "index-v4.bin",
        "index-v5.bin",
    ] {
        let _ = std::fs::remove_file(dir.join(stale));
    }
    dir.join(CACHE_FILE)
}

/// `path` with the home folder written as `~`, the way the agents' own docs write it.
pub fn tilde(path: &std::path::Path) -> String {
    match dirs::home_dir().and_then(|home| path.strip_prefix(&home).ok().map(std::path::Path::to_path_buf)) {
        Some(rest) if rest.as_os_str().is_empty() => "~".into(),
        Some(rest) => format!("~{}{}", std::path::MAIN_SEPARATOR, rest.display()),
        None => path.display().to_string(),
    }
}

#[cfg(test)]
mod tilde_tests {
    use super::*;

    #[test]
    fn writes_the_home_folder_as_a_tilde() {
        let home = dirs::home_dir().unwrap();
        let sep = std::path::MAIN_SEPARATOR;
        assert_eq!(tilde(&home.join(".claude").join("projects")), format!("~{sep}.claude{sep}projects"));
        assert_eq!(tilde(&home), "~");
    }

    #[test]
    fn leaves_other_paths_alone() {
        let p = PathBuf::from(if cfg!(windows) { r"D:\elsewhere\projects" } else { "/elsewhere/projects" });
        assert_eq!(tilde(&p), p.display().to_string());
    }
}
