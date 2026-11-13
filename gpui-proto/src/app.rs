//! The application state and everything that changes it. A port of `ui/src/state/store.ts` and the
//! keyboard handling in `App.tsx`; the views (`view_*.rs`) only read this and call back into it.

use crate::platform::{self, Settings};
use crate::theme::{Palette, PaletteGlobal};
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::base::{RenderedText, TextViewState};
use gpui_kit::*;
use logbook_core::dto::{SearchHitDto, SessionFilesDto};
use logbook_core::models::Agent;
use logbook_core::service::SessionService;
use logbook_gpui_proto::diff::{flatten_edits, DiffItem};
use logbook_gpui_proto::model::agents::{self, SourceInfo};
use logbook_gpui_proto::model::format;
use logbook_gpui_proto::model::select::{self, RepoCount, Row};
use logbook_gpui_proto::model::stats::Period;
use logbook_gpui_proto::model::{self, Session};
use logbook_gpui_proto::transcript::{self, Marks, Message};
use logbook_gpui_proto::update::{self, UpdateInfo};
use select::Range;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// `LOGBOOK_TRACE=1` prints how long the operations that matter take, for measuring against a big history.
fn tracing() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("LOGBOOK_TRACE").is_some())
}

macro_rules! trace {
    ($started:expr, $($arg:tt)*) => {
        if tracing() {
            eprintln!("[trace {:>7.1} ms] {}", $started.elapsed().as_secs_f64() * 1000.0, format!($($arg)*));
        }
    };
}

/// Coming back to the window re-scans for new sessions, at most this often.
const REINDEX_ON_FOCUS: Duration = Duration::from_secs(30);
/// The startup update check waits this long, so it never competes with loading the sessions.
const UPDATE_CHECK_DELAY: Duration = Duration::from_secs(5);
/// Logbook is often left open for days, so it checks again on this interval until it finds an update.
const UPDATE_RECHECK: Duration = Duration::from_secs(3600);
/// A new fact this often in the Stats view, unless the pointer is on the strip.
const FACT_EVERY: Duration = Duration::from_secs(7);
const NOTICE_FOR: Duration = Duration::from_millis(3200);

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum View {
    Sessions,
    Stats,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Transcript,
    Files,
}

#[derive(Clone, Default)]
pub struct Status {
    pub indexing: bool,
}

pub struct Notice {
    pub text: String,
    pub error: bool,
    pub at: Instant,
}

#[derive(Clone)]
pub struct WhatsNew {
    pub version: String,
    pub notes: Option<String>,
}

/// A list line: a day heading, or the row at that index of `rows`.
#[derive(Clone)]
pub enum Item {
    Header(String),
    Row(usize),
}

/// One message's markdown view, made when the message first scrolls into view.
pub struct MdEntry {
    pub state: Entity<TextViewState>,
    /// Whether the view was given the whole message (it changes when a long message folds or unfolds).
    pub open: bool,
    /// What the highlights on it answer: the search key and theme, and the parse they were worked out on.
    pub marked: Option<(String, RenderedText)>,
}

/// One entry of the detail pane's single scrolling list. The facts, the tabs and the content all scroll
/// together, as in the web UI, and only the entries on screen are laid out.
#[derive(Clone)]
pub enum DItem {
    Facts,
    Note,
    Tabs,
    Skeleton,
    Message(String),
    MarksBanner,
    Msg(usize),
    Heading(&'static str),
    FileRow(usize),
    NoFiles,
    Commands,
    Footnote,
    EditsHead,
    Diff(DiffItem),
    Pad,
}

pub struct Detail {
    pub id: String,
    pub messages: Option<Arc<Vec<Message>>>,
    pub error: Option<String>,
    pub tab: Tab,
    pub items: Arc<Vec<DItem>>,
    pub list: ListState,
    /// The list index of the first message (what comes before it is the facts, note, tabs and banner).
    pub msg_base: usize,
    pub marks: Marks,
    pub mds: HashMap<usize, MdEntry>,
    /// Messages the user folded or unfolded themselves; the rest follow the search.
    pub folds: HashMap<usize, bool>,
    pub pinned_open: bool,
    pub files: Option<SessionFilesDto>,
    pub files_error: Option<String>,
    /// The file whose edits the Files tab shows, or `None` for the list.
    pub file: Option<String>,
    pub edits: Option<Arc<Vec<DiffItem>>>,
    pub edits_empty: bool,
    pub edits_error: Option<String>,
}

impl Detail {
    fn new(id: String) -> Self {
        Self {
            id,
            messages: None,
            error: None,
            tab: Tab::Transcript,
            items: Arc::new(Vec::new()),
            list: ListState::new(0, ListAlignment::Top, px(600.)),
            msg_base: 0,
            marks: Marks::default(),
            mds: HashMap::new(),
            folds: HashMap::new(),
            pinned_open: false,
            files: None,
            files_error: None,
            file: None,
            edits: None,
            edits_empty: false,
            edits_error: None,
        }
    }
}

pub struct Logbook {
    pub service: Arc<SessionService>,
    pub focus: FocusHandle,
    pub input: Entity<InputState>,
    pub repo_input: Entity<InputState>,
    pub pal: Palette,
    pub zoom: f32,
    pub now: chrono::DateTime<chrono::Local>,

    // The data.
    pub sessions: Arc<Vec<Arc<Session>>>,
    pub by_id: Arc<HashMap<String, Arc<Session>>>,
    pub present: Vec<Agent>,
    pub status: Status,
    pub loaded: bool,
    pub error: Option<String>,
    pub sources: Option<Vec<SourceInfo>>,
    pub read_at: Option<Instant>,
    listed_generation: Option<u64>,

    // Search.
    pub query: String,
    pub hits: Option<Arc<Vec<SearchHitDto>>>,
    pub hits_query: String,
    pub searching: bool,
    search_seq: u64,
    search_in_flight: bool,
    pending_query: Option<String>,

    // Filters and views.
    pub range: Range,
    pub repo: Option<String>,
    pub agent: Option<Agent>,
    pub view: View,
    pub stats_period: Period,
    pub all_agents: bool,
    pub repo_query: String,

    // Worked out from the above whenever they change.
    pub rows: Arc<Vec<Row>>,
    pub items: Arc<Vec<Item>>,
    pub list: ListState,
    pub range_counts: HashMap<Range, usize>,
    pub repos: Vec<RepoCount>,
    pub agent_counts: Vec<(Agent, usize)>,

    pub sel_id: Option<String>,
    pub detail: Option<Detail>,

    // Chrome.
    pub notice: Option<Notice>,
    pub about: bool,
    pub about_checking: bool,
    pub sources_open: bool,
    /// When an outside click last closed the Sources popover, so the same click on its button doesn't reopen it.
    pub sources_closed_at: Option<Instant>,
    pub update: Option<UpdateInfo>,
    pub whats_new: Option<WhatsNew>,
    /// The Stats view's numbers, worked out once per list, period and day rather than on every frame.
    stats_cache: Option<((Period, u64, chrono::NaiveDate), Arc<logbook_gpui_proto::model::stats::Stats>)>,
    pub fact_i: usize,
    pub fact_held: bool,
    fact_at: Instant,
    last_scan: Instant,
    update_next: Option<Instant>,
    pub repos_scroll: ScrollHandle,
    pub stats_scroll: ScrollHandle,
    pub about_scroll: ScrollHandle,
    pub notes_scroll: ScrollHandle,
    pub sources_scroll: ScrollHandle,
    _subs: Vec<Subscription>,
}

impl Logbook {
    pub fn new(service: Arc<SessionService>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let settings = platform::load_settings();
        // No choice saved yet: follow the system, as the web UI does.
        let dark = settings.dark.unwrap_or_else(|| !matches!(window.appearance(), WindowAppearance::Light | WindowAppearance::VibrantLight));
        let zoom = platform::clamp_zoom(settings.zoom.unwrap_or(1.0));
        let pal = Palette::of(dark);
        cx.set_global(PaletteGlobal(pal));
        apply_component_theme(&pal, cx);
        window.set_rem_size(px(16.0 * zoom));

        let input = cx.new(|cx| InputState::new(window, cx).placeholder("Search names, repos and transcripts"));
        let repo_input = cx.new(|cx| InputState::new(window, cx).placeholder("Filter repositories"));
        let subs = vec![
            cx.subscribe_in(&input, window, |this, input, ev: &InputEvent, _window, cx| {
                if matches!(ev, InputEvent::Change) {
                    let q = input.read(cx).value().to_string();
                    this.set_query(q, cx);
                }
            }),
            cx.subscribe_in(&repo_input, window, |this, input, ev: &InputEvent, _window, cx| {
                if matches!(ev, InputEvent::Change) {
                    this.repo_query = input.read(cx).value().to_string();
                    cx.notify();
                }
            }),
            // Coming back to the window re-scans, at most every 30 seconds.
            cx.observe_window_activation(window, |this, window, cx| {
                if window.is_window_active() && this.last_scan.elapsed() >= REINDEX_ON_FOCUS {
                    this.reindex(cx);
                }
            }),
        ];

        let mut this = Self {
            service,
            focus: cx.focus_handle(),
            input,
            repo_input,
            pal,
            zoom,
            now: chrono::Local::now(),
            sessions: Arc::new(Vec::new()),
            by_id: Arc::new(HashMap::new()),
            present: Vec::new(),
            status: Status::default(),
            loaded: false,
            error: None,
            sources: None,
            read_at: None,
            listed_generation: None,
            query: String::new(),
            hits: None,
            hits_query: String::new(),
            searching: false,
            search_seq: 0,
            search_in_flight: false,
            pending_query: None,
            range: Range::All,
            repo: None,
            agent: None,
            view: View::Sessions,
            stats_period: Period::Month,
            all_agents: false,
            repo_query: String::new(),
            rows: Arc::new(Vec::new()),
            items: Arc::new(Vec::new()),
            list: ListState::new(0, ListAlignment::Top, px(600.)),
            range_counts: HashMap::new(),
            repos: Vec::new(),
            agent_counts: Vec::new(),
            sel_id: None,
            detail: None,
            notice: None,
            about: false,
            about_checking: false,
            sources_open: false,
            sources_closed_at: None,
            update: None,
            whats_new: None,
            stats_cache: None,
            fact_i: 0,
            fact_held: false,
            fact_at: Instant::now(),
            last_scan: Instant::now(),
            update_next: Some(Instant::now() + UPDATE_CHECK_DELAY),
            repos_scroll: ScrollHandle::new(),
            stats_scroll: ScrollHandle::new(),
            about_scroll: ScrollHandle::new(),
            notes_scroll: ScrollHandle::new(),
            sources_scroll: ScrollHandle::new(),
            _subs: subs,
        };
        // The cached sessions show at once; the first re-scan runs behind them.
        this.refresh(cx);
        this.reindex(cx);
        this.start_heartbeat(cx);
        // For looking at the update pill and What's New without a release to find: LOGBOOK_DEV_UPDATE=1.
        if std::env::var_os("LOGBOOK_DEV_UPDATE").is_some() {
            let u = UpdateInfo {
                version: "9.9.9".into(),
                notes: Some("## 9.9.9\n\n- A faster list\n- `Ctrl+wheel` zoom\n\n| a | b |\n|---|---|\n| 1 | 2 |".into()),
                date: None,
            };
            this.update = Some(u.clone());
            this.whats_new = Some(WhatsNew { version: u.version, notes: u.notes });
        }
        this
    }

    // ---- Loading -------------------------------------------------------------------------------

    /// Reads the index state, and the session list when the index changed since it was last read.
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        self.sources = Some(agents::source_infos(self.service.sources()));
        let generation = self.service.generation();
        self.status = Status { indexing: self.status.indexing };
        if !self.status.indexing {
            self.read_at = Some(Instant::now());
        }
        // A re-scan that found nothing new leaves the index as it was: keep the list.
        if self.loaded && self.error.is_none() && self.listed_generation == Some(generation) {
            cx.notify();
            return;
        }
        let service = self.service.clone();
        let started = Instant::now();
        cx.spawn(async move |this, cx| {
            let dtos = cx.background_spawn(async move { service.recent(usize::MAX) }).await;
            trace!(started, "recent() returned {} sessions", dtos.len());
            let sessions = cx.background_spawn(async move { model::load(dtos) }).await;
            trace!(started, "sessions wrapped");
            this.update(cx, |this, cx| {
                let t = Instant::now();
                this.sessions = Arc::new(sessions);
                this.by_id = Arc::new(select::sessions_by_id(&this.sessions));
                this.present = agents::present_agents(&this.sessions);
                this.listed_generation = Some(generation);
                this.loaded = true;
                this.error = None;
                this.refilter(false);
                trace!(t, "refilter ({} rows)", this.rows.len());
                // Re-run an active search so its hits reflect the new index.
                let q = this.query.clone();
                if !q.trim().is_empty() {
                    this.set_query(q, cx);
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// An incremental re-scan off the UI thread; the list is refetched when the index changed.
    pub fn reindex(&mut self, cx: &mut Context<Self>) {
        if self.status.indexing {
            return;
        }
        self.status.indexing = true;
        self.last_scan = Instant::now();
        cx.notify();
        let service = self.service.clone();
        let started = Instant::now();
        cx.spawn(async move |this, cx| {
            let result = cx.background_spawn(async move { service.rebuild() }).await;
            trace!(started, "re-scan finished: {:?}", result.as_ref().map(|s| (s.total, s.scanned, s.skipped)).map_err(|e| e.to_string()));
            this.update(cx, |this, cx| {
                this.status.indexing = false;
                match result {
                    Ok(_) => this.refresh(cx),
                    Err(e) => {
                        this.loaded = true;
                        this.error = Some(e.to_string());
                        cx.notify();
                    }
                }
            })
            .ok();
        })
        .detach();
    }

    /// The window's clock: the relative times, notices, the fact strip and the update checks.
    fn start_heartbeat(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| loop {
            cx.background_executor().timer(Duration::from_secs(1)).await;
            if this.update(cx, |this, cx| this.tick(cx)).is_err() {
                break;
            }
        })
        .detach();
    }

    fn tick(&mut self, cx: &mut Context<Self>) {
        let now = chrono::Local::now();
        let minute_changed = now.timestamp() / 60 != self.now.timestamp() / 60;
        let day_changed = now.date_naive() != self.now.date_naive();
        self.now = now;
        let mut dirty = minute_changed;
        if day_changed {
            self.refilter(false);
            dirty = true;
        }
        if self.notice.as_ref().is_some_and(|n| n.at.elapsed() >= NOTICE_FOR) {
            self.notice = None;
            dirty = true;
        }
        if self.view == View::Stats && !self.fact_held && self.fact_at.elapsed() >= FACT_EVERY {
            self.fact_i += 1;
            self.fact_at = Instant::now();
            dirty = true;
        }
        if self.update_next.is_some_and(|t| Instant::now() >= t) {
            self.update_next = None;
            self.check_update(false, cx);
        }
        if dirty {
            cx.notify();
        }
    }

    // ---- Updates -------------------------------------------------------------------------------

    /// Asks GitHub for a newer version. `manual` (from About) reports the outcome; otherwise failures stay quiet.
    pub fn check_update(&mut self, manual: bool, cx: &mut Context<Self>) {
        if manual {
            self.about_checking = true;
            cx.notify();
        }
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move {
                    let json = platform::fetch_latest()?;
                    update::parse_latest(&json, logbook_core::VERSION)
                })
                .await;
            this.update(cx, |this, cx| {
                this.about_checking = false;
                match result {
                    Ok(Some(u)) => {
                        this.update = Some(u.clone());
                        if manual {
                            this.about = false;
                            this.whats_new = Some(WhatsNew { version: u.version, notes: u.notes });
                        }
                    }
                    Ok(None) => {
                        if manual {
                            this.notify("Logbook is up to date", false);
                        }
                        // Not found yet: look again later, since the window is often left open for days.
                        this.update_next = Some(Instant::now() + UPDATE_RECHECK);
                    }
                    Err(e) => {
                        if manual {
                            this.notify(&e, true);
                        }
                        this.update_next = Some(Instant::now() + UPDATE_RECHECK);
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub fn open_release_page(&mut self, version: &str, cx: &mut Context<Self>) {
        match update::release_page(version) {
            Some(url) => cx.open_url(&url),
            None => self.notify(&format!("\"{version}\" is not a version"), true),
        }
    }

    // ---- Search and filters ---------------------------------------------------------------------

    /// Searching takes a few milliseconds, so it runs on every keystroke with no debounce. Only one
    /// request is in flight at a time: keystrokes made meanwhile collapse into `pending_query`, and a
    /// response is dropped unless it answers the latest query.
    pub fn set_query(&mut self, q: String, cx: &mut Context<Self>) {
        self.search_seq += 1;
        self.query = q.clone();
        if q.trim().is_empty() {
            self.pending_query = None;
            self.hits = None;
            self.hits_query.clear();
            self.searching = false;
            self.refilter(true);
            self.update_marks(cx);
            cx.notify();
            return;
        }
        self.searching = true;
        cx.notify();
        if self.search_in_flight {
            self.pending_query = Some(q);
            return;
        }
        self.run_search(q, self.search_seq, cx);
    }

    fn run_search(&mut self, q: String, seq: u64, cx: &mut Context<Self>) {
        self.search_in_flight = true;
        let service = self.service.clone();
        let started = Instant::now();
        cx.spawn(async move |this, cx| {
            let query = q.clone();
            let hits = cx.background_spawn(async move { service.search(&query, usize::MAX) }).await;
            trace!(started, "search {q:?}: {} hits", hits.len());
            this.update(cx, |this, cx| {
                this.search_in_flight = false;
                if seq == this.search_seq {
                    this.hits = Some(Arc::new(hits));
                    this.hits_query = q;
                    this.searching = false;
                    this.refilter(true);
                    trace!(started, "search results applied ({} rows)", this.rows.len());
                    this.update_marks(cx);
                }
                if let Some(next) = this.pending_query.take() {
                    let seq = this.search_seq;
                    this.run_search(next, seq, cx);
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// Picking a range, repository or agent shows its sessions, so from Stats they lead back to the list.
    pub fn set_range(&mut self, r: Range, cx: &mut Context<Self>) {
        self.range = r;
        self.view = View::Sessions;
        self.refilter(true);
        cx.notify();
    }

    pub fn set_repo(&mut self, r: Option<String>, cx: &mut Context<Self>) {
        self.repo = r;
        self.view = View::Sessions;
        self.refilter(true);
        cx.notify();
    }

    pub fn set_agent(&mut self, a: Option<Agent>, cx: &mut Context<Self>) {
        self.agent = a;
        self.view = View::Sessions;
        self.refilter(true);
        cx.notify();
    }

    pub fn set_view(&mut self, v: View, cx: &mut Context<Self>) {
        self.view = v;
        cx.notify();
    }

    /// Works out the rows, their day groups and the sidebar's counts from the data and the filters.
    pub fn refilter(&mut self, reset_scroll: bool) {
        let today = self.now;
        let hits = self.hits.as_ref().map(|h| h.as_slice());
        let rows = select::visible_rows(&self.sessions, &self.by_id, hits, self.range, self.repo.as_deref(), today, self.agent);
        let mut items = Vec::with_capacity(rows.len() + 16);
        let mut last: Option<String> = None;
        for (i, r) in rows.iter().enumerate() {
            let label = format::day_group(r.session.updated_at.as_deref().or(r.session.created_at.as_deref()), today);
            if last.as_deref() != Some(label.as_str()) {
                items.push(Item::Header(label.clone()));
                last = Some(label);
            }
            items.push(Item::Row(i));
        }
        if reset_scroll || self.items.len() == 0 {
            self.list.reset(items.len());
        } else {
            self.list.splice(0..self.items.len(), items.len());
        }
        self.rows = Arc::new(rows);
        self.items = Arc::new(items);
        self.range_counts = select::range_counts(&self.sessions, today);
        self.repos = select::repos(&self.sessions, self.range, today, self.agent);
        let shown = agents::shown_agents(&self.present, self.agent, self.all_agents);
        self.agent_counts = if self.present.len() < 2 { Vec::new() } else { select::agent_counts(&self.sessions, &shown, self.range, self.repo.as_deref(), today) };
    }

    /// The Stats view's numbers for the period, kept until the list, the period or the day changes.
    pub fn stats(&mut self) -> Arc<logbook_gpui_proto::model::stats::Stats> {
        let key = (self.stats_period, self.listed_generation.unwrap_or(0), self.now.date_naive());
        if let Some((k, s)) = &self.stats_cache {
            if *k == key {
                return s.clone();
            }
        }
        let s = Arc::new(logbook_gpui_proto::model::stats::compute_stats(&self.sessions, self.stats_period, self.now));
        self.stats_cache = Some((key, s.clone()));
        s
    }

    // ---- Selection and the detail pane -------------------------------------------------------------

    pub fn row_index_of(&self, id: &str) -> Option<usize> {
        self.rows.iter().position(|r| r.session.id == id)
    }

    pub fn select(&mut self, id: &str, cx: &mut Context<Self>) {
        if self.detail.as_ref().is_some_and(|d| d.id == id) {
            self.sel_id = Some(id.to_string());
            cx.notify();
            return;
        }
        // Found by a file it changed: open on that file's tab.
        let by_file = self.hits.as_ref().and_then(|h| h.iter().find(|h| h.id == id)).is_some_and(|h| h.matched == "file");
        let tab = if by_file { Tab::Files } else { Tab::Transcript };
        self.sel_id = Some(id.to_string());
        self.detail = Some(Detail::new(id.to_string()));
        self.rebuild_detail(true);
        let service = self.service.clone();
        let now = self.now;
        let load_id = id.to_string();
        let want = id.to_string();
        let started = Instant::now();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move {
                    service.transcript(&load_id).map(|t| transcript::messages(&t, now)).ok_or_else(|| format!("session {load_id} not found"))
                })
                .await;
            trace!(started, "transcript loaded ({} messages)", result.as_ref().map_or(0, |m| m.len()));
            this.update(cx, |this, cx| {
                let Some(d) = this.detail.as_mut().filter(|d| d.id == want) else { return };
                match result {
                    Ok(messages) => d.messages = Some(Arc::new(messages)),
                    Err(e) => d.error = Some(e),
                }
                this.update_marks(cx);
                this.rebuild_detail(true);
                cx.notify();
            })
            .ok();
        })
        .detach();
        if tab == Tab::Files {
            self.set_detail_tab(Tab::Files, cx);
        }
        cx.notify();
    }

    pub fn clear_selection(&mut self, cx: &mut Context<Self>) {
        self.sel_id = None;
        self.detail = None;
        cx.notify();
    }

    /// Moves the selection through the list by `delta` rows (or to the ends with `i32::MIN/MAX`).
    pub fn move_selection(&mut self, delta: i32, cx: &mut Context<Self>) {
        let len = self.rows.len();
        if len == 0 {
            return;
        }
        let at = self.sel_id.as_deref().and_then(|id| self.row_index_of(id));
        let next = match (delta, at) {
            (i32::MIN, _) => 0,
            (i32::MAX, _) => len - 1,
            (d, Some(i)) => (i as i64 + d as i64).clamp(0, len as i64 - 1) as usize,
            (_, None) => 0,
        };
        let id = self.rows[next].session.id.clone();
        self.select(&id, cx);
        if let Some(item_ix) = self.items.iter().position(|it| matches!(it, Item::Row(i) if *i == next)) {
            // Keep the heading above the first row of a group in view too.
            let target = if matches!(item_ix.checked_sub(1).and_then(|i| self.items.get(i)), Some(Item::Header(_))) { item_ix - 1 } else { item_ix };
            self.list.scroll_to_reveal_item(target);
        }
    }

    pub fn set_detail_tab(&mut self, tab: Tab, cx: &mut Context<Self>) {
        let Some(d) = self.detail.as_mut() else { return };
        d.tab = tab;
        self.rebuild_detail(true);
        cx.notify();
        let Some(d) = self.detail.as_mut() else { return };
        if tab != Tab::Files || d.files.is_some() {
            return;
        }
        let id = d.id.clone();
        let service = self.service.clone();
        cx.spawn(async move |this, cx| {
            let want = id.clone();
            let result = cx
                .background_spawn(async move { service.files(&id).ok_or_else(|| format!("session {id} not found")) })
                .await;
            this.update(cx, |this, cx| {
                let Some(d) = this.detail.as_mut().filter(|d| d.id == want) else { return };
                match result {
                    Ok(f) => d.files = Some(f),
                    Err(e) => d.files_error = Some(e),
                }
                this.rebuild_detail(false);
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// Shows one file's edits in the Files tab, or the list again (`None`).
    pub fn open_file(&mut self, file: Option<String>, cx: &mut Context<Self>) {
        let Some(d) = self.detail.as_mut() else { return };
        d.file = file.clone();
        d.edits = None;
        d.edits_empty = false;
        d.edits_error = None;
        let id = d.id.clone();
        self.rebuild_detail(true);
        cx.notify();
        let Some(path) = file else { return };
        let service = self.service.clone();
        cx.spawn(async move |this, cx| {
            let (want_id, want_path) = (id.clone(), path.clone());
            let result = cx
                .background_spawn(async move {
                    service.file_edits(&id, &path).map(|e| flatten_edits(&e)).ok_or_else(|| format!("session {id} not found"))
                })
                .await;
            this.update(cx, |this, cx| {
                let Some(d) = this.detail.as_mut().filter(|d| d.id == want_id && d.file.as_deref() == Some(want_path.as_str())) else { return };
                match result {
                    Ok((items, empty)) => {
                        d.edits = Some(Arc::new(items));
                        d.edits_empty = empty;
                    }
                    Err(e) => d.edits_error = Some(e),
                }
                this.rebuild_detail(false);
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// Works out which transcript messages match the search shown, off the UI thread, then jumps to the first.
    pub fn update_marks(&mut self, cx: &mut Context<Self>) {
        let query = if self.hits.is_some() { self.hits_query.clone() } else { String::new() };
        let Some(d) = self.detail.as_mut() else { return };
        let Some(messages) = d.messages.clone() else { return };
        let key = format::query_tokens(&query).join(" ");
        if d.marks.key == key {
            return;
        }
        if key.is_empty() {
            d.marks = Marks::default();
            self.rebuild_detail(false);
            cx.notify();
            return;
        }
        let want = d.id.clone();
        cx.spawn(async move |this, cx| {
            let marks = cx.background_spawn(async move { transcript::marks(&messages, &query) }).await;
            this.update(cx, |this, cx| {
                let Some(d) = this.detail.as_mut().filter(|d| d.id == want) else { return };
                let first = marks.first;
                d.marks = marks;
                d.mds.clear();
                this.rebuild_detail(false);
                // Land on the first search match rather than the top of a long conversation.
                if let (Some(first), Some(d)) = (first, this.detail.as_ref()) {
                    d.list.scroll_to(ListOffset { item_ix: d.msg_base + first, offset_in_item: px(0.) });
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// Works out the detail pane's entries for the tab and data it has, and resizes its list to match.
    /// `reset` starts at the top; otherwise the scroll position stays where it is.
    pub fn rebuild_detail(&mut self, reset: bool) {
        let by_id = self.by_id.clone();
        let Some(d) = self.detail.as_mut() else { return };
        let session = by_id.get(&d.id);
        let mut items = vec![DItem::Facts];
        if session.is_some_and(|s| s.source_deleted_at.is_some()) {
            items.push(DItem::Note);
        }
        let files_count = session.map_or(0, |s| s.files_created + s.files_edited);
        if files_count > 0 {
            items.push(DItem::Tabs);
        }
        let mut base = items.len();
        if d.tab == Tab::Files && files_count > 0 {
            match (&d.files_error, &d.files) {
                (Some(e), _) => items.push(DItem::Message(format!("Couldn’t read this session’s files: {e}"))),
                (None, None) => items.push(DItem::Skeleton),
                (None, Some(f)) => {
                    let opened = d.file.as_ref().filter(|p| f.files.iter().any(|x| &x.path == *p)).is_some();
                    if opened {
                        items.push(DItem::EditsHead);
                        match (&d.edits_error, &d.edits) {
                            (Some(e), _) => items.push(DItem::Message(format!("Couldn’t read the edits: {e}"))),
                            (None, None) => items.push(DItem::Skeleton),
                            (None, Some(edits)) if d.edits_empty => {
                                let _ = edits;
                                items.push(DItem::Message(if session.is_some_and(|s| s.source_deleted_at.is_some()) {
                                    "The agent deleted this session’s file, so its edits can’t be shown. Logbook kept the list of files.".into()
                                } else {
                                    "No edits to this file were found in the session’s file.".into()
                                }));
                            }
                            (None, Some(edits)) => items.extend(edits.iter().cloned().map(DItem::Diff)),
                        }
                        items.push(DItem::Pad);
                    } else {
                        for (label, created) in [("Created", true), ("Edited", false)] {
                            let mut any = false;
                            for (i, _file) in f.files.iter().enumerate().filter(|(_, x)| x.created == created) {
                                if !any {
                                    items.push(DItem::Heading(label));
                                    any = true;
                                }
                                items.push(DItem::FileRow(i));
                            }
                        }
                        if f.files.is_empty() {
                            items.push(DItem::NoFiles);
                        }
                        if !f.commands.is_empty() {
                            items.push(DItem::Commands);
                        }
                        items.push(DItem::Footnote);
                        items.push(DItem::Pad);
                    }
                }
            }
        } else {
            match (&d.error, &d.messages) {
                (Some(e), _) => items.push(DItem::Message(format!("Couldn’t read this transcript: {e}"))),
                (None, None) => items.push(DItem::Skeleton),
                (None, Some(m)) if m.is_empty() => items.push(DItem::Message("This session has no messages yet.".into())),
                (None, Some(m)) => {
                    if !d.marks.tokens.is_empty() {
                        items.push(DItem::MarksBanner);
                    }
                    base = items.len();
                    items.extend((0..m.len()).map(DItem::Msg));
                    items.push(DItem::Pad);
                }
            }
        }
        d.msg_base = base;
        let old = d.items.len();
        if reset || old == 0 {
            d.list.reset(items.len());
        } else {
            d.list.splice(0..old, items.len());
        }
        d.items = Arc::new(items);
    }

    pub fn toggle_fold(&mut self, ix: usize, open_now: bool, cx: &mut Context<Self>) {
        if let Some(d) = self.detail.as_mut() {
            d.folds.insert(ix, !open_now);
            let at = d.msg_base + ix;
            d.list.remeasure_items(at..at + 1);
            cx.notify();
        }
    }

    pub fn toggle_pinned(&mut self, cx: &mut Context<Self>) {
        if let Some(d) = self.detail.as_mut() {
            d.pinned_open = !d.pinned_open;
            cx.notify();
        }
    }

    pub fn jump_to_message(&mut self, ix: usize, cx: &mut Context<Self>) {
        if let Some(d) = self.detail.as_mut() {
            d.pinned_open = false;
            d.list.scroll_to(ListOffset { item_ix: d.msg_base + ix, offset_in_item: px(0.) });
            cx.notify();
        }
    }

    // ---- Actions ---------------------------------------------------------------------------------

    pub fn notify(&mut self, text: &str, error: bool) {
        self.notice = Some(Notice { text: text.to_string(), error, at: Instant::now() });
    }

    pub fn resume(&mut self, id: &str, cx: &mut Context<Self>) {
        if self.by_id.get(id).is_some_and(|s| s.source_deleted_at.is_some()) {
            self.notify("This session’s file was deleted, so it can’t be resumed. Copy still copies the command.", true);
        } else {
            match self.service.resume(id) {
                Some(resume) => match platform::launch_terminal(&resume) {
                    Ok(()) => self.notify("Opened a terminal to resume this session", false),
                    Err(e) => self.notify(&e.to_string(), true),
                },
                None => self.notify("This session can’t be resumed from here", true),
            }
        }
        cx.notify();
    }

    pub fn copy_resume(&mut self, id: &str, cx: &mut Context<Self>) {
        match self.service.resume_command(id) {
            Some(cmd) => {
                cx.write_to_clipboard(ClipboardItem::new_string(cmd));
                self.notify("Resume command copied", false);
            }
            None => self.notify(&format!("session {id} not found"), true),
        }
        cx.notify();
    }

    pub fn open_folder(&mut self, path: &str, cx: &mut Context<Self>) {
        if let Err(e) = platform::open_folder(path) {
            self.notify(&e, true);
            cx.notify();
        }
    }

    pub fn open_link(&mut self, url: &str, cx: &mut Context<Self>) {
        match platform::check_link(url) {
            Ok(()) => cx.open_url(url),
            Err(e) => {
                self.notify(&e, true);
                cx.notify();
            }
        }
    }

    pub fn toggle_theme(&mut self, cx: &mut Context<Self>) {
        let dark = !self.pal.dark;
        self.pal = Palette::of(dark);
        cx.set_global(PaletteGlobal(self.pal));
        apply_component_theme(&self.pal, cx);
        self.save_settings();
        cx.notify();
    }

    pub fn set_zoom(&mut self, z: f32, window: &mut Window, cx: &mut Context<Self>) {
        self.zoom = platform::clamp_zoom(z);
        window.set_rem_size(px(16.0 * self.zoom));
        // Row heights change with the rem size.
        self.list.remeasure();
        if let Some(d) = &self.detail {
            d.list.remeasure();
        }
        self.save_settings();
        cx.notify();
    }

    fn save_settings(&self) {
        platform::save_settings(&Settings { dark: Some(self.pal.dark), zoom: Some(self.zoom) });
    }

    pub fn set_about(&mut self, open: bool, cx: &mut Context<Self>) {
        self.about = open;
        cx.notify();
    }

    pub fn whats_new_close(&mut self, cx: &mut Context<Self>) {
        self.whats_new = None;
        cx.notify();
    }
}

/// The text boxes and scrollbars come from gpui-component, which reads its own theme: point it at ours.
fn apply_component_theme(p: &Palette, cx: &mut App) {
    let theme = cx.global_mut::<gpui_kit::component::Theme>();
    theme.foreground = p.ink;
    theme.muted_foreground = p.mute;
    theme.caret = p.accent;
    theme.selection = crate::ui::alpha(p.accent, 0.34);
    theme.ring = p.accent;
    theme.border = p.line;
    theme.input = p.line;
    theme.primary = p.accent;
    theme.link = p.accent_text;
    theme.muted = p.raised;
    theme.scrollbar_thumb = p.line;
    theme.scrollbar_thumb_hover = p.mute;
}
