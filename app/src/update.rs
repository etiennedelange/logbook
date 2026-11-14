//! In-app updates: reading `latest.json` from GitHub Releases, comparing versions, and checking the
//! installer's signature before it is run. The file and the signature format are the ones Tauri's updater
//! used, so releases made before this app was Tauri-free (0.6.x) and after it are updated the same way.

use base64::Engine as _;
use serde::Deserialize;
use std::collections::HashMap;

pub const LATEST_JSON: &str = "https://github.com/etiennedelange/logbook/releases/latest/download/latest.json";
const RELEASES: &str = "https://github.com/etiennedelange/logbook/releases";

/// The public half of the key that signs every installer (`pnpm tauri signer generate`). A copy of an
/// installer that doesn't verify against it is never run.
pub const PUBKEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDQ5RTE0QzE5NkI5RUQ4RjQKUldUMDJKNXJHVXpoU1hBTng3Z01nU3lOOHgrQklaZXEybzdRWXAzYkZrTy8yMk44MzU4YkxwMzAK";

/// The installer to download for an update, and its minisign signature (the `.sig` file's text, base64).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Installer {
    pub url: String,
    pub signature: String,
}

/// A newer release found on GitHub.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateInfo {
    pub version: String,
    /// The release's CHANGELOG section, as Markdown.
    pub notes: Option<String>,
    /// RFC 3339.
    pub date: Option<String>,
    /// The Windows installer, when the release has one.
    pub installer: Option<Installer>,
}

#[derive(Deserialize)]
struct Platform {
    url: String,
    signature: String,
}

#[derive(Deserialize)]
struct Latest {
    version: String,
    notes: Option<String>,
    pub_date: Option<String>,
    #[serde(default)]
    platforms: HashMap<String, Platform>,
}

/// `latest.json` keys this build can use, best first.
const PLATFORMS: [&str; 2] = ["windows-x86_64-nsis", "windows-x86_64"];

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
    // Only a plain https link is followed: the file comes off the network.
    let installer = PLATFORMS
        .iter()
        .find_map(|k| latest.platforms.get(*k))
        .filter(|p| p.url.starts_with("https://"))
        .map(|p| Installer { url: p.url.clone(), signature: p.signature.trim().to_string() });
    Ok(Some(UpdateInfo {
        version: latest.version.trim_start_matches('v').to_string(),
        notes: latest.notes.filter(|n| !n.trim().is_empty()),
        date: latest.pub_date,
        installer,
    }))
}

fn decode_text(b64: &str) -> Result<String, String> {
    let bytes = base64::engine::general_purpose::STANDARD.decode(b64.trim()).map_err(|e| format!("not base64: {e}"))?;
    String::from_utf8(bytes).map_err(|_| "not text".to_string())
}

/// Checks `data` against a release's signature with `pubkey` (both base64 of minisign's text files, as
/// `latest.json` and the app carry them). Errors say why, never what was inside.
pub fn verify(data: &[u8], signature: &str, pubkey: &str) -> Result<(), String> {
    let key = minisign_verify::PublicKey::decode(&decode_text(pubkey)?).map_err(|e| format!("the update key isn't usable: {e}"))?;
    let sig = minisign_verify::Signature::decode(&decode_text(signature)?).map_err(|e| format!("the update's signature isn't readable: {e}"))?;
    key.verify(data, &sig, true).map_err(|_| "the downloaded installer doesn't match its signature, so it was not run".to_string())
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

    // A throwaway key made with `tauri signer generate`, and a signature of `DATA` made with it.
    const TEST_KEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDA0NjU3NDg5NjRDQjA5M0UKUldRK0NjdGtpWFJsQkVydGZPdDJ0Sml3R2MvU3hBd0EwSTBab0lnMmY1UDRsUE5OK0dmT1FqbksK";
    const TEST_SIG: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZSBmcm9tIHRhdXJpIHNlY3JldCBrZXkKUlVRK0NjdGtpWFJsQkM3OTY3SjFjZ3BVbFZwcFltSjlIT294Ni8xdzlhaStiZzZQZXhtYk5mVUJjZW9zUWVOWFNiTTltSjlNWFJMOGZmbUVtaDkvQ0NML1E5K2g2aWFybUFjPQp0cnVzdGVkIGNvbW1lbnQ6IHRpbWVzdGFtcDoxNzk0Njg1NTYzCWZpbGU6ZGF0YS5iaW4KV3RxdmovYVkybHpyQnBqdU5qOWZWTWF4MDR2cDFPamVkMTFVS1lsTkRndWNEYllNQTRZN3k5eFB3c1NxczhUd2liQ1ZCY09RSmYrRXNwNE5KblNDQWc9PQo=";
    const DATA: &[u8] = b"logbook installer bytes";

    #[test]
    fn a_signed_installer_verifies_and_a_changed_one_does_not() {
        assert_eq!(verify(DATA, TEST_SIG, TEST_KEY), Ok(()));
        assert!(verify(b"logbook installer bytez", TEST_SIG, TEST_KEY).unwrap_err().contains("was not run"));
        // The wrong key, and text that isn't a key or signature at all.
        assert!(verify(DATA, TEST_SIG, PUBKEY).is_err());
        assert!(verify(DATA, "!!!", TEST_KEY).unwrap_err().contains("base64"));
        assert!(verify(DATA, TEST_SIG, "aGVsbG8=").is_err());
    }

    #[test]
    fn the_installer_comes_from_the_windows_entry_over_https_only() {
        let json = |url: &str| format!(r#"{{"version":"0.7.0","platforms":{{"windows-x86_64":{{"url":"{url}","signature":" sig \n"}}}}}}"#);
        let u = parse_latest(&json("https://github.com/x/releases/download/v0.7.0/Logbook_0.7.0_x64-setup.exe"), "0.6.5").unwrap().unwrap();
        assert_eq!(u.installer.unwrap(), Installer { url: "https://github.com/x/releases/download/v0.7.0/Logbook_0.7.0_x64-setup.exe".into(), signature: "sig".into() });
        assert_eq!(parse_latest(&json("http://example.com/a.exe"), "0.6.5").unwrap().unwrap().installer, None);
        assert_eq!(parse_latest(r#"{"version":"0.7.0"}"#, "0.6.5").unwrap().unwrap().installer, None);
    }

    #[test]
    fn release_page_takes_only_version_characters() {
        assert_eq!(release_page("0.2.0").as_deref(), Some("https://github.com/etiennedelange/logbook/releases/tag/v0.2.0"));
        assert_eq!(release_page(""), None);
        assert_eq!(release_page("1.0/../../evil"), None);
    }
}
