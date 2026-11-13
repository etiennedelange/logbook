//! Experiment: Logbook's search-and-list screen in GPUI, straight over `logbook-core`.
//! No IPC: the window holds a `SessionService` and calls it from background tasks.

mod rows;

use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::list::{List, ListDelegate, ListItem, ListState};
use gpui_kit::component::{h_flex, v_flex, ActiveTheme as _, IndexPath};
use gpui_kit::*;
use logbook_core::dto::SessionDto;
use logbook_core::service::SessionService;
use rows::{Row, Snippet};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

/// Wait this long after the last keystroke before searching.
const DEBOUNCE: Duration = Duration::from_millis(60);

/// The rows the list shows. `List` owns it and calls back into it.
struct Rows {
    service: Arc<SessionService>,
    rows: Vec<Row>,
    selected: Option<IndexPath>,
}

impl Rows {
    fn selected_id(&self) -> Option<&str> {
        self.selected.and_then(|ix| self.rows.get(ix.row)).map(|r| r.id.as_str())
    }
}

impl ListDelegate for Rows {
    type Item = ListItem;

    fn items_count(&self, _section: usize, _cx: &App) -> usize {
        self.rows.len()
    }

    fn render_item(&mut self, ix: IndexPath, _window: &mut Window, cx: &mut Context<ListState<Self>>) -> Option<ListItem> {
        let row = self.rows.get(ix.row)?;
        let muted = cx.theme().muted_foreground;
        let accent = cx.theme().primary;
        let mut body = v_flex()
            .gap_0p5()
            .overflow_hidden()
            .child(
                h_flex()
                    .gap_2()
                    .child(div().text_xs().text_color(muted).child(row.agent))
                    .child(div().truncate().child(row.title.clone())),
            )
            .child(div().text_xs().text_color(muted).truncate().child(row.meta.clone()));
        if let Some(Snippet { text, hits }) = &row.snippet {
            let highlights = hits
                .iter()
                .map(|r| (r.clone(), HighlightStyle { color: Some(accent), font_weight: Some(FontWeight::BOLD), ..Default::default() }))
                .collect::<Vec<_>>();
            body = body.child(
                div().text_xs().truncate().child(StyledText::new(text.clone()).with_highlights(highlights)),
            );
        }
        Some(ListItem::new(ix).selected(self.selected == Some(ix)).px_3().py_1().child(body))
    }

    fn set_selected_index(&mut self, ix: Option<IndexPath>, _window: &mut Window, cx: &mut Context<ListState<Self>>) {
        self.selected = ix;
        cx.notify();
    }

    /// Enter: copy the resume command (the prototype doesn't spawn a terminal).
    fn confirm(&mut self, _secondary: bool, _window: &mut Window, cx: &mut Context<ListState<Self>>) {
        if let Some(cmd) = self.selected_id().and_then(|id| self.service.resume_command(id)) {
            cx.write_to_clipboard(ClipboardItem::new_string(cmd));
        }
    }
}

struct App_ {
    service: Arc<SessionService>,
    input: Entity<InputState>,
    list: Entity<ListState<Rows>>,
    /// Every session by id: search hits carry only an id, as in the Tauri app.
    sessions: Arc<HashMap<String, SessionDto>>,
    recent: Vec<Row>,
    /// Bumped on every keystroke and index change; a finished search whose number is old is dropped.
    seq: u64,
    generation: u64,
    status: String,
    _search: Option<Task<()>>,
    _subs: Vec<Subscription>,
}

impl App_ {
    fn new(service: Arc<SessionService>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("Search sessions"));
        let list = cx.new(|cx| ListState::new(Rows { service: service.clone(), rows: Vec::new(), selected: None }, window, cx));
        let subs = vec![cx.subscribe_in(&input, window, |this, _, ev: &InputEvent, window, cx| match ev {
            InputEvent::Change => this.query_changed(window, cx),
            InputEvent::PressEnter { .. } => {
                // Focus stays in the box, so Enter there acts on the selected row too.
                this.list.update(cx, |l, cx| l.delegate_mut().confirm(false, window, cx));
            }
            _ => {}
        })];
        let mut this = Self {
            service,
            input,
            list,
            sessions: Arc::new(HashMap::new()),
            recent: Vec::new(),
            seq: 0,
            generation: 0,
            status: "Indexing…".into(),
            _search: None,
            _subs: subs,
        };
        this.load_cached(window, cx);
        this.rebuild(window, cx);
        this
    }

    fn load_cached(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let all = self.service.recent(usize::MAX);
        self.recent = rows::recent_rows(&all);
        self.sessions = Arc::new(all.into_iter().map(|s| (s.id.clone(), s)).collect());
        self.generation = self.service.generation();
        self.status = format!("{} sessions", self.service.listed_count());
        self.show(self.recent.clone(), window, cx);
    }

    /// An incremental re-scan off the UI thread; when the index changed, reload the list.
    fn rebuild(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let service = self.service.clone();
        let handle = window.window_handle();
        cx.spawn(async move |this, cx| {
            let result = cx.background_executor().spawn(async move { service.rebuild() }).await;
            handle
                .update(cx, |_, window, cx| {
                    this.update(cx, |this, cx| {
                        match result {
                            Ok(_) if this.service.generation() != this.generation => this.load_cached(window, cx),
                            Ok(_) => this.status = format!("{} sessions", this.service.listed_count()),
                            Err(e) => this.status = format!("Index error: {e}"),
                        }
                        cx.notify();
                    })
                    .ok();
                })
                .ok();
        })
        .detach();
    }

    fn show(&mut self, rows: Vec<Row>, window: &mut Window, cx: &mut Context<Self>) {
        self.list.update(cx, |l, cx| {
            l.delegate_mut().rows = rows;
            let first = (l.delegate().rows.len() > 0).then(|| IndexPath::new(0));
            l.set_selected_index(first, window, cx);
            if first.is_some() {
                l.scroll_to_selected_item(window, cx);
            }
            cx.notify();
        });
    }

    /// Up/Down from the search box: focus stays there, the selection moves in the list.
    fn move_selection(&mut self, delta: isize, window: &mut Window, cx: &mut Context<Self>) {
        self.list.update(cx, |l, cx| {
            let len = l.delegate().rows.len();
            if len == 0 {
                return;
            }
            let at = l.selected_index().map_or(0, |ix| ix.row) as isize;
            let next = (at + delta).clamp(0, len as isize - 1) as usize;
            l.set_selected_index(Some(IndexPath::new(next)), window, cx);
            l.scroll_to_selected_item(window, cx);
        });
    }

    fn query_changed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let q = self.input.read(cx).value().trim().to_string();
        self.seq += 1;
        let seq = self.seq;
        if q.is_empty() {
            self._search = None;
            self.status = format!("{} sessions", self.service.listed_count());
            self.show(self.recent.clone(), window, cx);
            return;
        }
        let service = self.service.clone();
        let sessions = self.sessions.clone();
        let handle = window.window_handle();
        self._search = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(DEBOUNCE).await;
            let rows = cx
                .background_executor()
                .spawn(async move { rows::hit_rows(&service.search(&q, usize::MAX), &sessions) })
                .await;
            handle
                .update(cx, |_, window, cx| {
                    this.update(cx, |this, cx| {
                        if this.seq == seq {
                            this.status = format!("{} results", rows.len());
                            this.show(rows, window, cx);
                            cx.notify();
                        }
                    })
                    .ok();
                })
                .ok();
        }));
    }
}

impl Render for App_ {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        v_flex()
            .size_full()
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, window, cx| match ev.keystroke.key.as_str() {
                "down" => this.move_selection(1, window, cx),
                "up" => this.move_selection(-1, window, cx),
                _ => {}
            }))
            .bg(theme.background)
            .text_color(theme.foreground)
            .child(div().p_2().child(Input::new(&self.input)))
            .child(div().flex_1().min_h_0().child(List::new(&self.list)))
            .child(div().px_3().py_1().text_xs().text_color(theme.muted_foreground).child(self.status.clone()))
    }
}

fn main() -> anyhow::Result<()> {
    // Cached sessions show at once; the first rebuild runs in the background.
    let service = Arc::new(SessionService::open()?);
    gpui_kit::application().run(move |cx| {
        gpui_kit::init(cx);
        gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
            let view = cx.new(|cx| App_::new(service, window, cx));
            let input = view.read(cx).input.clone();
            input.update(cx, |i, cx| i.focus(window, cx));
            view
        })
        .expect("failed to open window");
        cx.activate(true);
    });
    Ok(())
}
