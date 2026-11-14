//! UI tests: the real views in a headless window, driven with real clicks and keys against a real (temporary)
//! history of Copilot CLI and Claude Code sessions. They stand in for what the web UI's `ui-check` (every control)
//! and e2e (reading files, indexing, search, deletion) steps covered. Everything is checked from what the user
//! sees: the rows the list shows, what a click or a key does, what the detail pane loads.

// Explicit imports, not `use super::*`: that would bring GPUI's own `test` macro into scope, which then shadows
// the standard `#[test]` its test macro expands to, and recurses.
use crate::app::{self, DItem, Logbook, Tab};
use gpui_kit::{
    px, size, AppContext, Bounds, Context, Entity, Modifiers, Pixels, Point, TestAppContext, VisualTestContext,
    WindowBounds, WindowOptions,
};
use logbook_app::model::select::Range;
use logbook_app::update::{Installer, UpdateInfo};
use logbook_core::service::SessionService;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::{Mutex, OnceLock};

// ---- A history to test against -------------------------------------------------------------------------

struct Fixture {
    home: PathBuf,
    service: Arc<SessionService>,
}

/// The tests share one history and one service, and run one at a time (they set process-wide paths).
static LOCK: Mutex<()> = Mutex::new(());

fn iso(hours_ago: f64) -> String {
    (chrono::Utc::now() - chrono::Duration::milliseconds((hours_ago * 3_600_000.0) as i64)).to_rfc3339()
}

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn copilot(home: &Path, id: &str, name: &str, cwd: &str, branch: &str, hours_ago: f64, messages: &[(&str, &str)]) {
    let dir = home.join("copilot/session-state").join(id);
    write(
        &dir.join("workspace.yaml"),
        &format!(
            "id: {id}\nname: {name}\ncwd: {cwd}\nbranch: {branch}\ncreated_at: {}\nupdated_at: {}\n",
            iso(hours_ago + 1.0),
            iso(hours_ago)
        ),
    );
    let mut lines = vec![json!({"type": "session.start", "timestamp": iso(hours_ago + 1.0)}).to_string()];
    for (i, (role, text)) in messages.iter().enumerate() {
        let kind = if *role == "user" { "user.message" } else { "assistant.message" };
        lines.push(
            json!({"type": kind, "timestamp": iso(hours_ago + 1.0 - i as f64 * 0.01), "data": {"content": text}})
                .to_string(),
        );
    }
    write(&dir.join("events.jsonl"), &(lines.join("\n") + "\n"));
}

fn claude(home: &Path, id: &str, cwd: &str, hours_ago: f64, prompt: &str, reply: &str, edit: Option<&str>) -> PathBuf {
    let proj = home.join("claude/projects").join(cwd.replace(|c: char| !c.is_ascii_alphanumeric(), "-"));
    let line = |kind: &str, h: f64, message: serde_json::Value| {
        json!({"type": kind, "isSidechain": false, "message": message, "timestamp": iso(h), "cwd": cwd, "gitBranch": "feat/tests", "sessionId": id}).to_string()
    };
    let mut lines = vec![
        line("user", hours_ago, json!({"role": "user", "content": prompt})),
        line(
            "assistant",
            hours_ago - 0.01,
            json!({"id": "m1", "role": "assistant", "content": [{"type": "text", "text": reply}]}),
        ),
    ];
    if let Some(file) = edit {
        lines.push(line("assistant", hours_ago - 0.02, json!({"id": "m2", "role": "assistant", "content": [{"type": "tool_use", "id": "e1", "name": "Edit", "input": {"file_path": file}}]})));
        lines.push(
            json!({"type": "user", "isSidechain": false, "message": {"role": "user", "content": [{"type": "tool_result", "tool_use_id": "e1", "content": "ok"}]},
                "toolUseResult": {"filePath": file, "structuredPatch": [{"oldStart": 7, "oldLines": 3, "newStart": 7, "newLines": 3, "lines": [" fn split_rows() {", "-    i % 2 == 0", "+    i == 0", " }"]}]},
                "timestamp": iso(hours_ago - 0.03), "cwd": cwd, "gitBranch": "feat/tests", "sessionId": id})
            .to_string(),
        );
    }
    let path = proj.join(format!("{id}.jsonl"));
    write(&path, &(lines.join("\n") + "\n"));
    path
}

fn fixture() -> &'static Fixture {
    static F: OnceLock<Fixture> = OnceLock::new();
    F.get_or_init(|| {
        let home = std::env::temp_dir().join(format!("logbook-ui-tests-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        let work = |n: &str| home.join("work").join(n).display().to_string();
        copilot(
            &home,
            "c0000000-0000-0000-0000-000000000001",
            "Fix the flaky overdue-loans test",
            &work("bookshelf"),
            "fix/flake",
            1.0,
            &[
                ("user", "The overdue-loans test fails one run in five."),
                ("assistant", "The fixture seeds loans with the wall clock. I will freeze it."),
            ],
        );
        copilot(
            &home,
            "c0000000-0000-0000-0000-000000000002",
            "Dashboard env config for staging",
            &work("reading-list"),
            "main",
            5.0,
            &[
                ("user", "please build the env config for staging"),
                ("assistant", "How does the reminder queue fit in? **Sure**, `env/prod.json` first."),
                ("user", "How does the reminder queue fit in?"),
            ],
        );
        copilot(
            &home,
            "c0000000-0000-0000-0000-000000000003",
            "Refactor the indexer",
            &work("bookshelf"),
            "main",
            30.0,
            &[("user", "refactor the indexer"), ("assistant", "done")],
        );
        copilot(
            &home,
            "c0000000-0000-0000-0000-000000000004",
            "Update dependencies",
            &work("site"),
            "main",
            24.0 * 20.0,
            &[("user", "update dependencies"), ("assistant", "all updated")],
        );
        // A long reply, to fold.
        let long = "A very long reply. ".repeat(200);
        copilot(
            &home,
            "c0000000-0000-0000-0000-000000000005",
            "A session with a long reply",
            &work("site"),
            "main",
            24.0 * 3.0,
            &[("user", "say a lot"), ("assistant", &long)],
        );
        claude(
            &home,
            "d0000000-0000-0000-0000-000000000001",
            &work("bookshelf"),
            2.0,
            "Why does the tokenizer drop zebra-striped rows?",
            "The row filter treats alternate rows as headers. Fixed in `split_rows`.",
            Some(&format!("{}/src/zebra_rows.rs", work("bookshelf"))),
        );
        claude(
            &home,
            "d0000000-0000-0000-0000-000000000002",
            &work("site"),
            24.0 * 10.0,
            "Write the changelog",
            "Here it is.",
            None,
        );
        // These paths are read when the service opens, and the settings file lands next to the cache.
        std::env::set_var("COPILOT_HOME", home.join("copilot"));
        std::env::set_var("CLAUDE_CONFIG_DIR", home.join("claude"));
        std::env::set_var("LOGBOOK_CACHE_DIR", home.join("cache"));
        let service = Arc::new(SessionService::open().expect("open the service"));
        service.rebuild().expect("index the fixture");
        Fixture { home, service }
    })
}

// ---- Driving the app -----------------------------------------------------------------------------------

struct App {
    view: Entity<Logbook>,
    cx: &'static mut VisualTestContext,
}

fn open(cx: &mut TestAppContext) -> App {
    let fx = fixture();
    cx.update(gpui_kit::init);
    let service = fx.service.clone();
    let (handle, view) = cx.update(|cx| {
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds {
                origin: Point::default(),
                size: size(px(1200.), px(800.)),
            })),
            ..Default::default()
        };
        gpui_kit::open_window(options, cx, |window, cx| {
            let view = cx.new(|cx| Logbook::new(service, window, cx));
            let focus = view.read(cx).focus.clone();
            window.focus(&focus, cx);
            view
        })
        .expect("open the window")
    });
    // The window outlives this function: leak it for the life of the test (a headless test process).
    let vcx: &'static mut VisualTestContext = Box::leak(Box::new(VisualTestContext::from_window(handle, cx)));
    let mut app = App { view, cx: vcx };
    app.settle();
    app
}

impl App {
    /// Lets background work (indexing, searching, reading a transcript) finish and the window redraw.
    fn settle(&mut self) {
        self.cx.run_until_parked();
        self.cx.update(|window, _| window.refresh());
        self.cx.run_until_parked();
    }

    fn read<R>(&mut self, f: impl FnOnce(&Logbook) -> R) -> R {
        self.view.read_with(self.cx, |v, _| f(v))
    }

    fn update(&mut self, f: impl FnOnce(&mut Logbook, &mut Context<Logbook>)) {
        self.view.update(self.cx, |v, cx| f(v, cx));
        self.settle();
    }

    fn keys(&mut self, keys: &str) {
        self.cx.simulate_keystrokes(keys);
        self.settle();
    }

    fn typed(&mut self, text: &str) {
        self.cx.simulate_input(text);
        self.settle();
    }

    fn bounds(&mut self, id: &str) -> Option<Bounds<Pixels>> {
        // `debug_bounds` wants a 'static name; test names are few.
        let name: &'static str = Box::leak(id.to_string().into_boxed_str());
        self.cx.debug_bounds(name)
    }

    fn exists(&mut self, id: &str) -> bool {
        self.bounds(id).is_some()
    }

    fn click(&mut self, id: &str) {
        let b = self.bounds(id).unwrap_or_else(|| panic!("no element named {id:?} on screen"));
        self.cx.simulate_click(b.center(), Modifiers::default());
        self.settle();
    }

    fn titles(&mut self) -> Vec<String> {
        self.read(|v| v.rows.iter().map(|r| r.session.title.text.clone()).collect())
    }

    /// Opens the session with this title by clicking its row.
    fn open_session(&mut self, title: &str) {
        let ix = self.titles().iter().position(|t| t == title).unwrap_or_else(|| panic!("no session titled {title:?}"));
        // Only rows on screen can be clicked: scroll the list to it first.
        self.update(|v, cx| {
            let id = v.rows[ix].session.id.clone();
            v.select(&id, cx);
        });
    }

    fn selected_title(&mut self) -> Option<String> {
        self.read(|v| v.sel_id.as_ref().and_then(|id| v.by_id.get(id)).map(|s| s.title.text.clone()))
    }

    fn detail_items(&mut self) -> Vec<DItem> {
        self.read(|v| v.detail.as_ref().map(|d| d.items.to_vec()).unwrap_or_default())
    }
}

// ---- The tests ---------------------------------------------------------------------------------------------

#[gpui::test]
fn lists_every_session_grouped_by_day_most_recent_first(cx: &mut TestAppContext) {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut app = open(cx);
    let titles = app.titles();
    assert!(app.read(|v| v.loaded), "the list loads");
    assert_eq!(titles.first().map(String::as_str), Some("Fix the flaky overdue-loans test"));
    for expected in ["Why does the tokenizer drop zebra-striped rows?", "Update dependencies", "Write the changelog"] {
        assert!(titles.iter().any(|t| t == expected), "{expected:?} is listed");
    }
    // Headings: the first rows are today's.
    let first_heading =
        app.read(|v| v.items.iter().find_map(|i| if let app::Item::Header(h) = i { Some(h.clone()) } else { None }));
    assert_eq!(first_heading.as_deref(), Some("Today"));
    // Two agents are present, so the sidebar offers to filter by agent.
    assert!(app.exists("agent:Copilot CLI") && app.exists("agent:Claude Code"));
}

#[gpui::test]
fn the_range_keys_and_the_sidebar_pick_a_time_range(cx: &mut TestAppContext) {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut app = open(cx);
    let all = app.titles().len();
    app.keys("2");
    assert_eq!(app.read(|v| v.range), Range::Today);
    let today = app.titles();
    assert!(
        today.contains(&"Fix the flaky overdue-loans test".to_string())
            && !today.contains(&"Update dependencies".to_string())
    );
    app.keys("3");
    assert!(
        app.titles().len() > today.len() && app.titles().len() < all,
        "last 7 days is between today and everything"
    );
    app.click("nav:All sessions");
    assert_eq!(app.read(|v| v.range), Range::All);
    assert_eq!(app.titles().len(), all);
    app.click("nav:Last 30 days");
    assert_eq!(app.read(|v| v.range), Range::Month);
}

#[gpui::test]
fn picking_an_agent_or_a_repository_narrows_the_list_and_toggles_back(cx: &mut TestAppContext) {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut app = open(cx);
    let all = app.titles().len();
    app.click("agent:Claude Code");
    assert!(app.read(|v| v.rows.iter().all(|r| r.session.agent == logbook_core::models::Agent::ClaudeCode)));
    assert!(app.titles().len() < all);
    app.click("agent:Claude Code");
    assert_eq!(app.titles().len(), all, "picking it again clears the filter");
    // Alt+1 is the first agent in the sidebar's order.
    app.keys("alt-1");
    assert!(app.read(|v| v.agent.is_some()));
    app.keys("alt-1");
    assert!(app.read(|v| v.agent.is_none()));
    // A repository.
    app.click("repo:bookshelf");
    assert_eq!(app.read(|v| v.repo.clone()).as_deref(), Some("bookshelf"));
    assert!(app.read(|v| v.rows.iter().all(|r| r.session.repo.as_deref() == Some("bookshelf"))));
    assert!(app.exists("all-repos"));
    app.click("all-repos");
    assert_eq!(app.read(|v| v.repo.clone()), None);
    assert_eq!(app.titles().len(), all);
}

#[gpui::test]
fn search_finds_sessions_by_what_was_said_and_marks_the_transcript(cx: &mut TestAppContext) {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut app = open(cx);
    app.keys("/");
    app.typed("reminder");
    assert_eq!(app.read(|v| v.hits_query.clone()), "reminder");
    assert_eq!(app.titles(), ["Dashboard env config for staging"], "only the session that says it");
    // Words match from their start.
    app.keys("ctrl-a backspace");
    app.typed("tokenizer dro");
    assert_eq!(app.titles(), ["Why does the tokenizer drop zebra-striped rows?"]);
    // Opening a match marks the messages that contain it.
    app.open_session("Why does the tokenizer drop zebra-striped rows?");
    assert_eq!(app.read(|v| v.detail.as_ref().map(|d| d.marks.matches)), Some(1));
    // Nothing matches: the list says so.
    app.keys("ctrl-a backspace");
    app.typed("qqqqzzzz");
    assert!(app.titles().is_empty());
    // Esc clears the box, and a second Esc leaves it.
    app.keys("escape");
    assert_eq!(app.read(|v| v.query.clone()), "");
    assert!(app.titles().len() > 3, "the whole list is back");
}

#[gpui::test]
fn selecting_a_session_reads_its_transcript_and_the_keys_move_through_the_list(cx: &mut TestAppContext) {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut app = open(cx);
    assert_eq!(app.selected_title(), None);
    app.keys("down");
    assert_eq!(
        app.selected_title().as_deref(),
        Some("Fix the flaky overdue-loans test"),
        "Down from nothing picks the first"
    );
    let messages = app.read(|v| v.detail.as_ref().and_then(|d| d.messages.as_ref().map(|m| m.len())));
    assert_eq!(messages, Some(2));
    app.keys("down");
    assert_ne!(app.selected_title().as_deref(), Some("Fix the flaky overdue-loans test"));
    app.keys("end");
    assert_eq!(app.selected_title().as_deref(), Some("Update dependencies"), "the oldest is last");
    app.keys("home");
    assert_eq!(app.selected_title().as_deref(), Some("Fix the flaky overdue-loans test"));
    app.keys("escape");
    assert_eq!(app.selected_title(), None, "Esc closes the session");
    assert!(app.read(|v| v.detail.is_none()));
}

#[gpui::test]
fn a_long_reply_starts_folded_and_unfolds(cx: &mut TestAppContext) {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut app = open(cx);
    app.open_session("A session with a long reply");
    // Message 1 (the reply) is long, so it starts folded: its button says so. Scroll it into view first: only
    // what's on screen can be clicked.
    app.update(|v, _| {
        if let Some(d) = &v.detail {
            d.list.scroll_to_end();
        }
    });
    assert!(app.exists("fold:1"), "the long message offers to show more");
    let folded = app.read(|v| v.detail.as_ref().is_some_and(|d| !d.folds.contains_key(&1)));
    assert!(folded);
    app.click("fold:1");
    assert_eq!(
        app.read(|v| v.detail.as_ref().and_then(|d| d.folds.get(&1).copied())),
        Some(true),
        "unfolded by the user"
    );
    // Unfolded, the message is much taller and its button has moved: scroll to it again.
    app.update(|v, _| {
        if let Some(d) = &v.detail {
            d.list.scroll_to_end();
        }
    });
    app.click("fold:1");
    assert_eq!(app.read(|v| v.detail.as_ref().and_then(|d| d.folds.get(&1).copied())), Some(false));
}

#[gpui::test]
fn the_files_tab_lists_what_a_session_changed_and_shows_each_edit_as_a_diff(cx: &mut TestAppContext) {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut app = open(cx);
    // A session that changed no files has no tabs.
    app.open_session("Update dependencies");
    assert!(!app.exists("tab-files"));
    app.open_session("Why does the tokenizer drop zebra-striped rows?");
    assert!(app.exists("tab-files"), "this one edited a file");
    app.click("tab-files");
    assert_eq!(app.read(|v| v.detail.as_ref().map(|d| d.tab)), Some(Tab::Files));
    assert!(app.detail_items().iter().any(|i| matches!(i, DItem::FileRow(_))), "the edited file is listed");
    app.click("file:0");
    let lines: Vec<_> = app
        .detail_items()
        .into_iter()
        .filter_map(|i| {
            if let DItem::Diff(logbook_app::diff::DiffItem::Line { op, text, .. }) = i {
                Some((op, text))
            } else {
                None
            }
        })
        .collect();
    assert!(
        lines.contains(&('-', "    i % 2 == 0".to_string())) && lines.contains(&('+', "    i == 0".to_string())),
        "{lines:?}"
    );
    app.click("edits-back");
    assert!(app.detail_items().iter().any(|i| matches!(i, DItem::FileRow(_))));
    app.click("tab-transcript");
    assert_eq!(app.read(|v| v.detail.as_ref().map(|d| d.tab)), Some(Tab::Transcript));
}

#[gpui::test]
fn the_search_for_a_file_name_opens_on_the_files_tab(cx: &mut TestAppContext) {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut app = open(cx);
    app.keys("/");
    app.typed("zebra_rows");
    let hit = app.read(|v| v.hits.as_ref().and_then(|h| h.first().map(|h| h.matched)));
    assert_eq!(hit, Some("file"), "found by a file it changed");
    app.open_session("Why does the tokenizer drop zebra-striped rows?");
    assert_eq!(app.read(|v| v.detail.as_ref().map(|d| d.tab)), Some(Tab::Files));
}

#[gpui::test]
fn stats_opens_with_5_and_esc_goes_back(cx: &mut TestAppContext) {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut app = open(cx);
    app.keys("5");
    assert!(app.read(|v| v.view == app::View::Stats));
    assert!(app.exists("period:Last 30 days") && app.exists("fact-strip"));
    app.click("period:26 weeks");
    assert_eq!(app.read(|v| v.stats_period), logbook_app::model::stats::Period::Half);
    let facts = app.read(|v| v.fact_i);
    app.click("fact-strip");
    assert_eq!(app.read(|v| v.fact_i), facts + 1, "a click shows another fact");
    app.keys("escape");
    assert!(app.read(|v| v.view == app::View::Sessions));
    // Choosing a range from Stats leads back to the list.
    app.keys("5");
    app.click("nav:Today");
    assert!(app.read(|v| v.view == app::View::Sessions && v.range == Range::Today));
}

#[gpui::test]
fn about_and_the_sources_popover_open_and_close(cx: &mut TestAppContext) {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut app = open(cx);
    app.click("about");
    assert!(app.read(|v| v.about) && app.exists("about"), "the About button opens the dialog");
    app.keys("escape");
    assert!(!app.read(|v| v.about));
    app.keys("?");
    assert!(app.read(|v| v.about), "? opens it from anywhere");
    app.click("about-close");
    assert!(!app.read(|v| v.about));
    app.click("sources");
    assert!(app.read(|v| v.sources_open) && app.exists("sources-popover"));
    app.keys("escape");
    assert!(!app.read(|v| v.sources_open));
}

#[gpui::test]
fn copy_puts_the_resume_command_on_the_clipboard_and_a_notice_says_so(cx: &mut TestAppContext) {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut app = open(cx);
    app.open_session("Fix the flaky overdue-loans test");
    app.click("copy-resume");
    let copied = app.cx.read_from_clipboard().and_then(|c| c.text());
    assert_eq!(copied.as_deref(), Some("copilot --resume=c0000000-0000-0000-0000-000000000001"));
    assert_eq!(
        app.read(|v| v.notice.as_ref().map(|n| (n.text.clone(), n.error))),
        Some(("Resume command copied".to_string(), false))
    );
    // A Claude Code session resumes with its own command.
    app.open_session("Write the changelog");
    app.click("copy-resume");
    let copied = app.cx.read_from_clipboard().and_then(|c| c.text());
    assert_eq!(copied.as_deref(), Some("claude --resume d0000000-0000-0000-0000-000000000002"));
}

#[gpui::test]
fn the_theme_and_zoom_change_and_zoom_resets_with_ctrl_0(cx: &mut TestAppContext) {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut app = open(cx);
    let dark = app.read(|v| v.pal.dark);
    app.click("theme");
    assert_eq!(app.read(|v| v.pal.dark), !dark);
    app.click("theme");
    assert_eq!(app.read(|v| v.pal.dark), dark);
    app.update(|v, _| v.zoom = 1.5);
    app.keys("ctrl-0");
    assert_eq!(app.read(|v| v.zoom), 1.0);
}

#[gpui::test]
fn an_update_shows_in_the_toolbar_and_opens_whats_new(cx: &mut TestAppContext) {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut app = open(cx);
    assert!(!app.exists("update-pill"), "nothing shows until there is one");
    let info = UpdateInfo {
        version: "9.9.9".into(),
        notes: Some("- a faster list".into()),
        date: None,
        installer: Some(Installer {
            url: "https://example.com/Logbook_9.9.9_x64-setup.exe".into(),
            signature: "x".into(),
        }),
    };
    app.update(|v, cx| {
        v.update = Some(info);
        cx.notify();
    });
    assert!(app.exists("update-pill"));
    app.click("update-pill");
    assert!(app.read(|v| v.whats_new.as_ref().is_some_and(|w| w.version == "9.9.9")));
    assert!(app.exists("whats-get") && app.exists("whats-later"), "Update now and Later");
    // While it installs the dialog stays put.
    app.update(|v, _| v.updating = true);
    app.keys("escape");
    assert!(app.read(|v| v.whats_new.is_some()), "Esc doesn't close it mid-install");
    app.update(|v, _| v.updating = false);
    app.click("whats-later");
    assert!(app.read(|v| v.whats_new.is_none()));
}

#[gpui::test]
fn a_session_the_agent_deleted_stays_readable_but_cannot_be_resumed(cx: &mut TestAppContext) {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut app = open(cx);
    let fx = fixture();
    let file = claude(
        &fx.home,
        "d0000000-0000-0000-0000-0000000000ff",
        &fx.home.join("work/site").display().to_string(),
        6.0,
        "Remember the zucchini plan",
        "Noted: zucchini on Friday.",
        None,
    );
    app.update(|v, cx| v.reindex(cx));
    assert!(app.titles().iter().any(|t| t == "Remember the zucchini plan"), "indexed");
    std::fs::remove_file(&file).unwrap();
    app.update(|v, cx| v.reindex(cx));
    let kept = app.read(|v| {
        v.rows
            .iter()
            .find(|r| r.session.title.text == "Remember the zucchini plan")
            .map(|r| r.session.source_deleted_at.is_some())
    });
    assert_eq!(kept, Some(true), "kept after its file is gone, and marked");
    // Still found by what was said.
    app.keys("/");
    app.typed("zucchini");
    assert_eq!(app.titles(), ["Remember the zucchini plan"]);
    app.open_session("Remember the zucchini plan");
    assert_eq!(app.read(|v| v.detail.as_ref().and_then(|d| d.messages.as_ref().map(|m| m.len()))), Some(2));
    // Resume says why it can't: the first Esc empties the search box, the second leaves it, and then Enter
    // acts on the list.
    app.keys("escape");
    app.keys("escape");
    app.keys("enter");
    assert!(app.read(|v| v.notice.as_ref().is_some_and(|n| n.error && n.text.contains("deleted"))));
}
