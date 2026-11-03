//! Integration tests for `SessionService` against a temp `session-state` dir (no real
//! `~/.copilot` touched).

use logbook_core::service::{is_safe_id, SessionService};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

// `COPILOT_HOME`/`CLAUDE_CONFIG_DIR`/`LOGBOOK_CACHE_DIR` are process-wide env vars, so serialize
// the setup+bootstrap step across tests to avoid one test's temp dir leaking into
// another's `SessionService::bootstrap()` call.
static ENV_GUARD: Mutex<()> = Mutex::new(());

fn unique_temp_dir(name: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    std::env::temp_dir().join(format!("logbook-test-{name}-{}-{nanos}", std::process::id()))
}

fn write_session(session_state_root: &Path, id: &str, name: &str) {
    let dir = session_state_root.join(id);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("workspace.yaml"),
        format!(
            "id: {id}\nname: {name}\ncwd: /repo\ncreated_at: 2026-01-01T00:00:00Z\nupdated_at: 2026-01-02T00:00:00Z\n"
        ),
    )
    .unwrap();
    std::fs::write(
        dir.join("events.jsonl"),
        "{\"type\":\"user.message\",\"timestamp\":\"2026-01-02T00:00:00Z\",\"data\":{\"content\":\"please build the dashboard env config\"}}\n\
         {\"type\":\"assistant.message\",\"timestamp\":\"2026-01-02T00:01:00Z\",\"data\":{\"content\":\"sure, working on the dashboard now\"}}\n",
    )
    .unwrap();
}

/// Points `COPILOT_HOME`/`LOGBOOK_CACHE_DIR` at `root` with one session in it and
/// bootstraps a service. Must be called while holding `ENV_GUARD`.
fn bootstrap_in(root: &Path) -> SessionService {
    write_session(&root.join("session-state"), "sess-1", "dashboard work");
    let cache_dir = root.join("cache");
    std::fs::create_dir_all(&cache_dir).unwrap();
    std::env::set_var("COPILOT_HOME", root);
    // No Claude Code folder: these tests are about Copilot alone.
    std::env::set_var("CLAUDE_CONFIG_DIR", root.join("claude"));
    std::env::set_var("LOGBOOK_CACHE_DIR", &cache_dir);
    SessionService::bootstrap().expect("bootstrap should succeed")
}

#[test]
fn search_returns_snippet_with_stx_etx_markers() {
    let _guard = ENV_GUARD.lock().unwrap();
    let root = unique_temp_dir("search");
    let service = bootstrap_in(&root);

    let items = service.search("dashboard", 10);
    assert!(!items.is_empty(), "expected at least one search result");
    let snippet = items[0].snippet.as_deref().expect("snippet");
    assert!(snippet.contains('\u{2}'), "snippet should contain STX marker: {snippet:?}");
    assert!(snippet.contains('\u{3}'), "snippet should contain ETX marker: {snippet:?}");

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn blank_search_matches_nothing() {
    let _guard = ENV_GUARD.lock().unwrap();
    let root = unique_temp_dir("search-blank");
    let service = bootstrap_in(&root);

    assert!(service.search("   ", 10).is_empty());

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn unknown_session_is_none() {
    let _guard = ENV_GUARD.lock().unwrap();
    let root = unique_temp_dir("detail-none");
    let service = bootstrap_in(&root);

    assert!(service.get("copilot:nope").is_none());
    assert!(service.transcript("copilot:nope").is_none());
    assert!(service.resume_command("copilot:nope").is_none());

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn session_detail_transcript_and_resume_command() {
    let _guard = ENV_GUARD.lock().unwrap();
    let root = unique_temp_dir("full-flow");
    let service = bootstrap_in(&root);

    let detail = service.get("copilot:sess-1").expect("session");
    assert_eq!(detail.name.as_deref(), Some("dashboard work"));
    assert_eq!(detail.first_prompt.as_deref(), Some("please build the dashboard env config"));

    let timeline = service.transcript("copilot:sess-1").expect("transcript");
    assert_eq!(timeline.len(), 2);
    assert_eq!(timeline[0].role, "user");

    assert_eq!(service.resume_command("copilot:sess-1").as_deref(), Some("copilot --resume=sess-1"));
    assert_eq!(service.stats().total, 1);

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn only_plain_ids_are_safe_for_a_command_line() {
    assert!(is_safe_id("3f1c9a2e-7b44-4d0e-9a51-1c2f0e8b7d10"));
    assert!(!is_safe_id(""));
    assert!(!is_safe_id("abc & calc"));
    assert!(!is_safe_id("..\\..\\x"));
}

#[test]
fn renaming_a_session_is_picked_up_without_new_messages() {
    let _guard = ENV_GUARD.lock().unwrap();
    let root = unique_temp_dir("rename");
    let service = bootstrap_in(&root);

    // Copilot rewrites workspace.yaml on rename and leaves events.jsonl alone.
    std::thread::sleep(std::time::Duration::from_millis(20));
    let yaml = root.join("session-state").join("sess-1").join("workspace.yaml");
    let renamed = std::fs::read_to_string(&yaml).unwrap().replace("name: dashboard work", "name: renamed");
    std::fs::write(&yaml, renamed).unwrap();

    let stats = service.rebuild().unwrap();
    assert_eq!(stats.scanned, 1);
    assert_eq!(service.get("copilot:sess-1").unwrap().name.as_deref(), Some("renamed"));
    assert_eq!(service.rebuild().unwrap().skipped, 1);

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn mismatched_yaml_id_is_keyed_by_folder_and_not_rescanned() {
    let _guard = ENV_GUARD.lock().unwrap();
    let root = unique_temp_dir("idmismatch");
    let service = bootstrap_in(&root);

    let yaml = root.join("session-state").join("sess-1").join("workspace.yaml");
    let text = std::fs::read_to_string(&yaml).unwrap().replace("id: sess-1", "id: some-other-id");
    std::fs::write(&yaml, text).unwrap();

    // The yaml change is rescanned once, then the session must stay put.
    let first = service.rebuild().unwrap();
    assert_eq!(first.removed, 0);
    assert!(service.get("copilot:sess-1").is_some());
    let second = service.rebuild().unwrap();
    assert_eq!((second.scanned, second.removed, second.skipped), (0, 0, 1));

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn session_whose_yaml_disappears_is_dropped() {
    let _guard = ENV_GUARD.lock().unwrap();
    let root = unique_temp_dir("noyaml");
    let service = bootstrap_in(&root);
    assert!(service.get("copilot:sess-1").is_some());

    std::fs::remove_file(root.join("session-state").join("sess-1").join("workspace.yaml")).unwrap();
    let stats = service.rebuild().unwrap();
    assert_eq!(stats.removed, 1);
    assert!(service.get("copilot:sess-1").is_none());

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn missing_sessions_folder_keeps_cached_sessions() {
    let _guard = ENV_GUARD.lock().unwrap();
    let root = unique_temp_dir("noroot");
    let service = bootstrap_in(&root);

    // E.g. a profile on a drive that isn't mounted: nothing to look at, so nothing is dropped.
    std::fs::remove_dir_all(root.join("session-state")).unwrap();
    let stats = service.rebuild().unwrap();
    assert_eq!((stats.total, stats.removed), (0, 0));
    assert!(service.get("copilot:sess-1").is_some());

    std::fs::remove_dir_all(&root).ok();
}
