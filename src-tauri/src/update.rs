//! In-app updates from GitHub Releases. `tauri-plugin-updater` reads `latest.json` from the latest
//! release (`plugins.updater` in tauri.conf.json), and refuses an installer whose signature doesn't match
//! the public key there. See docs/RELEASING.md.

use serde::Serialize;
use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;
use tauri_plugin_updater::UpdaterExt;

const RELEASES: &str = "https://github.com/etiennedelange/logbook/releases";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub version: String,
    /// The release's CHANGELOG section (Markdown).
    pub notes: Option<String>,
    /// RFC 3339, as published in latest.json.
    pub date: Option<String>,
}

/// Asks GitHub whether a newer version is out. `None` when Logbook is up to date.
#[tauri::command]
pub async fn check_update(app: AppHandle) -> Result<Option<UpdateInfo>, String> {
    let found = async { app.updater()?.check().await }
        .await
        .map_err(|e| format!("Couldn't check for updates: {e}"))?;
    Ok(found.map(|u| UpdateInfo {
        date: u.raw_json.get("pub_date").and_then(|d| d.as_str()).map(str::to_owned),
        notes: u.body.filter(|b| !b.trim().is_empty()),
        version: u.version,
    }))
}

/// Downloads the update, checks its signature and runs the installer. On Windows the installer runs
/// in passive mode (a progress bar, no questions), exits this process and starts the new version.
#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), String> {
    let failed = |e: tauri_plugin_updater::Error| format!("Couldn't update Logbook: {e}");
    let update = app.updater().map_err(failed)?.check().await.map_err(failed)?;
    let Some(update) = update else {
        return Err("Logbook is already up to date.".into());
    };
    update.download_and_install(|_, _| {}, || {}).await.map_err(failed)?;
    // Only reached off Windows, where the new version is in place but not running yet.
    app.restart();
}

/// Opens the GitHub release page of `version`.
#[tauri::command]
pub fn open_release_page(app: AppHandle, version: String) -> Result<(), String> {
    let url = release_page(&version).ok_or_else(|| format!("\"{version}\" is not a version"))?;
    app.opener().open_url(url, None::<&str>).map_err(|e| e.to_string())
}

/// The release page URL, for a string made only of version characters (it ends up in a URL).
fn release_page(version: &str) -> Option<String> {
    let valid = !version.is_empty() && version.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-');
    valid.then(|| format!("{RELEASES}/tag/v{version}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_page_takes_only_version_characters() {
        assert_eq!(
            release_page("0.2.0").as_deref(),
            Some("https://github.com/etiennedelange/logbook/releases/tag/v0.2.0")
        );
        assert_eq!(
            release_page("1.0.0-rc.1").as_deref(),
            Some("https://github.com/etiennedelange/logbook/releases/tag/v1.0.0-rc.1")
        );
        assert_eq!(release_page(""), None);
        assert_eq!(release_page("1.0/../../evil"), None);
        assert_eq!(release_page("1.0?x=y"), None);
    }
}
