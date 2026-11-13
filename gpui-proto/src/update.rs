//! The update check: reading `latest.json` from GitHub Releases and comparing versions. This build only
//! tells you a newer release exists and opens its page; installing stays with the Tauri app's updater,
//! which verifies the installer's signature.

use serde::Deserialize;

pub const LATEST_JSON: &str = "https://github.com/etiennedelange/logbook/releases/latest/download/latest.json";
const RELEASES: &str = "https://github.com/etiennedelange/logbook/releases";

/// A newer release found on GitHub.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateInfo {
    pub version: String,
    /// The release's CHANGELOG section, as Markdown.
    pub notes: Option<String>,
    /// RFC 3339.
    pub date: Option<String>,
}

#[derive(Deserialize)]
struct Latest {
    version: String,
    notes: Option<String>,
    pub_date: Option<String>,
}

/// `(numbers, is_release)`: "1.0.0-rc.1" sorts before "1.0.0".
fn version_key(v: &str) -> Option<(Vec<u64>, bool)> {
    let v = v.trim().trim_start_matches('v');
    let (core, pre) = match v.split_once('-') {
        Some((c, _)) => (c, true),
        None => (v, false),
    };
    let nums: Option<Vec<u64>> = core.split('.').map(|p| p.parse().ok()).collect();
    nums.map(|n| (n, !pre))
}

pub fn is_newer(candidate: &str, current: &str) -> bool {
    match (version_key(candidate), version_key(current)) {
        (Some(a), Some(b)) => a > b,
        _ => false,
    }
}

/// `Some` when `json` (a Tauri `latest.json`) names a version newer than `current`.
pub fn parse_latest(json: &str, current: &str) -> Result<Option<UpdateInfo>, String> {
    let latest: Latest = serde_json::from_str(json).map_err(|e| format!("latest.json isn't readable: {e}"))?;
    if !is_newer(&latest.version, current) {
        return Ok(None);
    }
    Ok(Some(UpdateInfo {
        version: latest.version.trim_start_matches('v').to_string(),
        notes: latest.notes.filter(|n| !n.trim().is_empty()),
        date: latest.pub_date,
    }))
}

/// The release page URL, for a string made only of version characters (it ends up in a URL).
pub fn release_page(version: &str) -> Option<String> {
    let valid = !version.is_empty() && version.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-');
    valid.then(|| format!("{RELEASES}/tag/v{version}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newer_versions_compare_numerically_and_prereleases_sort_first() {
        assert!(is_newer("0.10.0", "0.9.9"));
        assert!(is_newer("v1.0.0", "1.0.0-rc.1"));
        assert!(!is_newer("1.0.0-rc.1", "1.0.0"));
        assert!(!is_newer("0.6.5", "0.6.5"));
        assert!(!is_newer("garbage", "0.6.5"));
    }

    #[test]
    fn latest_json_is_read_only_when_newer() {
        let json = r#"{"version":"0.7.0","notes":"- faster","pub_date":"2026-11-10T00:00:00Z","platforms":{}}"#;
        let u = parse_latest(json, "0.6.5").unwrap().unwrap();
        assert_eq!((u.version.as_str(), u.notes.as_deref()), ("0.7.0", Some("- faster")));
        assert_eq!(parse_latest(json, "0.7.0").unwrap(), None);
        assert!(parse_latest("not json", "0.6.5").is_err());
        let blank = r#"{"version":"0.7.0","notes":"  "}"#;
        assert_eq!(parse_latest(blank, "0.6.5").unwrap().unwrap().notes, None);
    }

    #[test]
    fn release_page_takes_only_version_characters() {
        assert_eq!(release_page("0.2.0").as_deref(), Some("https://github.com/etiennedelange/logbook/releases/tag/v0.2.0"));
        assert_eq!(release_page(""), None);
        assert_eq!(release_page("1.0/../../evil"), None);
    }
}
