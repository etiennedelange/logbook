//! Claude Code sessions end to end, against a temp `CLAUDE_CONFIG_DIR` holding the fixtures in
//! `tests/fixtures/claude/` (no real `~/.claude` touched).

use logbook_core::service::SessionService;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

static ENV_GUARD: Mutex<()> = Mutex::new(());

const ID: &str = "5b7c1d2e-0000-4000-8000-00000000c1a0";

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/claude").join(name)
}

/// A config folder with the fixture as one session, plus the clutter Claude Code keeps
/// beside transcripts. Must be called while holding `ENV_GUARD`.
fn setup(name: &str) -> (PathBuf, PathBuf) {
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let root = std::env::temp_dir().join(format!("logbook-claude-{name}-{}-{nanos}", std::process::id()));
    let project = root.join("claude/projects/-home-me-demo");
    std::fs::create_dir_all(project.join(ID).join("subagents")).unwrap();
    let transcript = project.join(format!("{ID}.jsonl"));
    std::fs::copy(fixture("synthetic.jsonl"), &transcript).unwrap();
    std::fs::write(project.join(ID).join("subagents/agent-1.jsonl"), "{\"type\":\"user\"}\n").unwrap();
    std::fs::write(project.join("notes.txt"), "not a session").unwrap();
    std::fs::create_dir_all(root.join("cache")).unwrap();
    std::env::set_var("CLAUDE_CONFIG_DIR", root.join("claude"));
    std::env::set_var("COPILOT_HOME", root.join("no-copilot"));
    std::env::set_var("LOGBOOK_CACHE_DIR", root.join("cache"));
    (root, transcript)
}

#[test]
fn reads_a_transcript_into_a_session() {
    let _guard = ENV_GUARD.lock().unwrap();
    let (root, _) = setup("read");
    let service = SessionService::bootstrap().unwrap();
    assert_eq!(service.stats().total, 1, "subagent transcripts and other files aren't sessions");

    let s = service.get(&format!("claude:{ID}")).expect("session");
    assert_eq!(s.native_id, ID);
    // `/rename` beats a generated title, even a later one.
    assert_eq!(s.name.as_deref(), Some("Escapes in the parser"));
    assert!(s.user_named);
    assert_eq!(s.cwd.as_deref(), Some("/home/me/demo"));
    assert_eq!(s.branch.as_deref(), Some("feature/escapes"));
    assert_eq!(s.created_at.as_deref(), Some("2026-10-30T09:00:00+00:00"));
    assert_eq!(s.updated_at.as_deref(), Some("2026-10-30T09:08:00+00:00"));
    assert_eq!(s.first_prompt.as_deref(), Some("Where does the parser handle quoted strings?"));
    assert_eq!(s.message_count, 4);
    assert_eq!(s.source_deleted_at, None);

    let t = service.transcript(&s.id).unwrap();
    let lines: Vec<(&str, &str)> = t.iter().map(|e| (e.role.as_str(), e.text.as_str())).collect();
    // Left out: the tool's own context, a tool result, and the truncated last line. One reply's
    // text blocks are one message; a slash command with arguments is a prompt.
    assert_eq!(
        lines,
        [
            ("user", "Where does the   parser\nhandle quoted strings?"),
            ("assistant", "Quoted strings are handled in `lex_string`.\n\nEscapes are decoded in the same pass."),
            ("user", "/review the escape handling"),
            ("assistant", "The escape handling looks right."),
        ]
    );

    assert_eq!(service.resume_command(&s.id).as_deref(), Some(&*format!("claude --resume {ID}")));
    let resume = service.resume(&s.id).unwrap();
    assert_eq!(resume.cwd.as_deref(), Some("/home/me/demo"));

    // Searchable, and the tool output isn't.
    assert_eq!(service.search("escapes decoded", 10).len(), 1);
    assert!(service.search("tool output", 10).is_empty());

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn a_deleted_transcript_is_kept() {
    let _guard = ENV_GUARD.lock().unwrap();
    let (root, transcript) = setup("kept");
    let service = SessionService::bootstrap().unwrap();
    let id = format!("claude:{ID}");

    // Claude Code deletes transcripts after `cleanupPeriodDays`.
    let saved = std::fs::read(&transcript).unwrap();
    std::fs::remove_file(&transcript).unwrap();
    let stats = service.rebuild().unwrap();
    assert_eq!((stats.removed, stats.source_deleted), (0, 1));

    let s = service.get(&id).expect("still there");
    assert!(s.source_deleted_at.is_some());
    assert_eq!(service.transcript(&id).unwrap().len(), 4);
    assert_eq!(service.search("lex_string", 10).len(), 1);
    assert_eq!(service.recent(10).len(), 1);
    // Still has a command to copy.
    assert!(service.resume_command(&id).is_some());

    // It survives a restart, from the cache alone.
    let reopened = SessionService::open().unwrap();
    assert!(reopened.get(&id).unwrap().source_deleted_at.is_some());
    assert_eq!(reopened.transcript(&id).unwrap().len(), 4);

    // And if the file comes back, it's an ordinary session again.
    std::fs::write(&transcript, saved).unwrap();
    service.rebuild().unwrap();
    assert_eq!(service.get(&id).unwrap().source_deleted_at, None);

    std::fs::remove_dir_all(&root).ok();
}
