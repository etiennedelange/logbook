//! Experiment: Logbook's search-and-list screen in GPUI, straight over `logbook-core`.
//! No IPC: the window holds a `SessionService` and calls it from background tasks.

use logbook_gpui_proto::rows;

use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::list::{List, ListDelegate, ListEvent, ListItem, ListState};
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::component::text::TextView;
use gpui_kit::component::{h_flex, v_flex, ActiveTheme as _, IndexPath};
use gpui_kit::*;
use logbook_core::dto::SessionDto;
use logbook_core::service::SessionService;
use rows::{Message, Row, Snippet};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

/// Wait this long after the selection settles before loading its transcript, so holding an arrow key
/// doesn't read a file per row it passes.
const TRANSCRIPT_DEBOUNCE: Duration = Duration::from_millis(120);

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

/// The transcript pane's state. `list` is GPUI's own variable-height virtual list, so only the
/// messages on screen are laid out.
struct Transcript {
    id: String,
    messages: Arc<Vec<Message>>,
    list: gpui_kit::ListState,
    loading: bool,
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
    transcript: Option<Transcript>,
    /// Bumped per transcript request; also keeps message element ids unique across sessions.
    tseq: u64,
    _load: Option<Task<()>>,
    _search: Option<Task<()>>,
    _subs: Vec<Subscription>,
}

impl App_ {
    fn new(service: Arc<SessionService>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("Search sessions"));
        let list = cx.new(|cx| ListState::new(Rows { service: service.clone(), rows: Vec::new(), selected: None }, window, cx));
        let list_sub = cx.subscribe_in(&list, window, |this, _, ev: &ListEvent, window, cx| {
            if matches!(ev, ListEvent::Select(_) | ListEvent::Confirm(_)) {
                this.sync_transcript(window, cx);
            }
        });
        let subs = vec![list_sub, cx.subscribe_in(&input, window, |this, _, ev: &InputEvent, window, cx| match ev {
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
            transcript: None,
            tseq: 0,
            _load: None,
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
        self.sync_transcript(window, cx);
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
        self.sync_transcript(window, cx);
    }

    /// Loads the selected session's transcript off the UI thread, once the selection has settled.
    fn sync_transcript(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let id = self.list.read(cx).delegate().selected_id().map(str::to_string);
        let Some(id) = id else {
            self.transcript = None;
            self._load = None;
            cx.notify();
            return;
        };
        if self.transcript.as_ref().is_some_and(|t| t.id == id) {
            return;
        }
        self.tseq += 1;
        let seq = self.tseq;
        self.transcript = Some(Transcript {
            id: id.clone(),
            messages: Arc::new(Vec::new()),
            list: gpui_kit::ListState::new(0, ListAlignment::Top, px(400.)),
            loading: true,
        });
        cx.notify();
        let service = self.service.clone();
        let handle = window.window_handle();
        self._load = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(TRANSCRIPT_DEBOUNCE).await;
            let load_id = id.clone();
            let messages = cx
                .background_executor()
                .spawn(async move { service.transcript(&load_id).map(|t| rows::messages(&t)).unwrap_or_default() })
                .await;
            handle
                .update(cx, |_, _window, cx| {
                    this.update(cx, |this, cx| {
                        if this.tseq == seq {
                            let list = gpui_kit::ListState::new(messages.len(), ListAlignment::Top, px(400.));
                            this.transcript = Some(Transcript { id, messages: Arc::new(messages), list, loading: false });
                            cx.notify();
                        }
                    })
                    .ok();
                })
                .ok();
        }));
    }

    fn render_transcript(&self, cx: &mut Context<Self>) -> AnyElement {
        let muted = cx.theme().muted_foreground;
        let note = |text: &str| div().size_full().flex().items_center().justify_center().text_color(muted).child(text.to_string()).into_any_element();
        let Some(t) = &self.transcript else { return note("Select a session") };
        if t.loading {
            return note("Loading…");
        }
        if t.messages.is_empty() {
            return note("No messages");
        }
        let header = self.sessions.get(&t.id).map(|s| (rows::title(s), rows::meta(s)));
        let messages = t.messages.clone();
        let seq = self.tseq;
        let accent = cx.theme().primary;
        let border = cx.theme().border;
        let rows = gpui_kit::list(t.list.clone(), move |ix, _window, _cx| {
            let m = &messages[ix];
            let who = div()
                .font_weight(FontWeight::BOLD)
                .text_color(if m.role == "user" { accent } else { muted })
                .child(m.label.clone());
            v_flex()
                .w_full()
                .px_4()
                .py_3()
                .gap_1()
                .border_b_1()
                .border_color(border)
                .child(h_flex().gap_2().text_xs().child(who).children(m.when.clone().map(|w| div().text_color(muted).child(w))))
                .child(TextView::markdown(SharedString::from(format!("m{seq}-{ix}")), m.text.clone()).selectable(true))
                .into_any_element()
        })
        .size_full();
        v_flex()
            .size_full()
            .child(match header {
                Some((title, meta)) => v_flex()
                    .px_4()
                    .py_2()
                    .border_b_1()
                    .border_color(border)
                    .child(div().truncate().child(title))
                    .child(div().text_xs().text_color(muted).truncate().child(meta))
                    .into_any_element(),
                None => div().into_any_element(),
            })
            .child(div().id("transcript").relative().flex_1().min_h_0().child(rows).vertical_scrollbar(&t.list))
            .into_any_element()
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
        let transcript = self.render_transcript(cx);
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
            .child(
                h_flex()
                    .flex_1()
                    .min_h_0()
                    .child(div().w(relative(0.4)).h_full().border_r_1().border_color(theme.border).child(List::new(&self.list)))
                    .child(div().flex_1().min_w_0().h_full().child(transcript)),
            )
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
