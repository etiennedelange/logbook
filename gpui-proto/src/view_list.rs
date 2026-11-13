//! The session list pane: header, search box, day-grouped virtual list of rows, and its empty states.

use crate::app::{Item, Logbook};
use crate::ui::*;
use crate::view_chrome::*;
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use logbook_gpui_proto::model::agents::agent_info;
use logbook_gpui_proto::model::format::{plural, rel_time, short_repo, snippet_segments, TitleSource};
use logbook_gpui_proto::model::select::{range_label, Range};

impl Logbook {
    pub fn list_pane(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = self.pal;
        let range_label = range_label(self.range);
        let agent_name = self.agent.map(|a| agent_info(a).name);
        let title = match &self.repo {
            Some(r) => short_repo(r).to_string(),
            None => agent_name.unwrap_or(range_label).to_string(),
        };
        // The counts describe the results on screen, which trail the search box while a search runs.
        let q = if self.hits.is_some() { self.hits_query.trim().to_string() } else { String::new() };
        let n = self.rows.len();
        let mut sub = vec![if q.is_empty() { plural(n, "session", "sessions") } else { format!("{} for “{q}”", plural(n, "match", "matches")) }];
        if self.repo.is_some() {
            if let Some(a) = agent_name {
                sub.push(a.to_string());
            }
        }
        if self.repo.is_some() || self.agent.is_some() {
            if !(self.range == Range::All && self.agent.is_some()) {
                sub.push(range_label.to_lowercase());
            }
        }
        let sub = sub.join(", ");

        let header_end = self.repo.is_some().then(|| {
            div()
                .id("all-repos")
                .rounded(u(6.))
                .bg(p.btn)
                .px(u(9.))
                .py(u(4.))
                .text_size(u(12.))
                .text_color(p.soft)
                .cursor_pointer()
                .hover(move |s| s.bg(p.btn_hover))
                .tooltip_text(format!("Stop filtering to {}", self.repo.clone().unwrap_or_default()))
                .on_click(cx.listener(|this, _, _, cx| this.set_repo(None, cx)))
                .child("All repositories")
        });

        let body: AnyElement = if let Some(e) = &self.error {
            self.empty_state("Couldn’t load sessions", Some(e.clone()), None, cx)
        } else if !self.loaded {
            self.list_skeleton()
        } else if self.rows.is_empty() {
            if !q.is_empty() {
                let filtered = self.range != Range::All || self.repo.is_some() || self.agent.is_some();
                self.empty_state(
                    "No matches",
                    Some("Every word has to start a word somewhere in the session. Try fewer or shorter words.".into()),
                    filtered.then_some(("Search all sessions", 0)),
                    cx,
                )
            } else if self.sessions.is_empty() {
                if self.status.indexing {
                    self.empty_state("Indexing your sessions…", Some("The first scan reads every transcript once; after that only changed sessions are read again.".into()), None, cx)
                } else {
                    self.empty_state("No sessions yet", Some("Sessions from the GitHub Copilot CLI (~/.copilot/session-state) and Claude Code (~/.claude/projects) show up here once they’re indexed.".into()), None, cx)
                }
            } else {
                self.empty_state("Nothing in this range", Some("No sessions were active in this period.".into()), Some(("Show all sessions", 1)), cx)
            }
        } else {
            let top = self.list.logical_scroll_top().item_ix;
            // The day heading the first visible row belongs to stays pinned at the top.
            let sticky = self.items.iter().take(top + 1).rev().find_map(|it| match it {
                Item::Header(l) => Some(l.clone()),
                _ => None,
            });
            div()
                .relative()
                .size_full()
                .child(
                    list(self.list.clone(), cx.processor(|this, ix, window, cx| this.list_item(ix, window, cx)))
                        .size_full()
                        .px(u(8.))
                        .pb(u(8.)),
                )
                .when_some(sticky, |d, label| {
                    d.child(
                        div()
                            .absolute()
                            .top_0()
                            .left(u(8.))
                            .right(u(8.))
                            .bg(p.list)
                            .px(u(12.))
                            .pb(u(5.))
                            .pt(u(12.))
                            .text_size(u(11.))
                            .font_weight(MEDIUM)
                            .text_color(p.mute)
                            .child(label),
                    )
                })
                .vertical_scrollbar(&self.list)
                .into_any_element()
        };

        let search = self.search_box(&self.input.clone(), "clear-search", &self.query.clone(), window, cx);
        div()
            .flex()
            .flex_col()
            .h_full()
            .min_h_0()
            .min_w_0()
            .bg(p.list)
            .border_r_1()
            .border_color(p.line)
            .child(self.pane_header("list-title", title, 16., 16., header_end.map(IntoElement::into_any_element).unwrap_or_else(|| div().into_any_element())))
            .child(
                div()
                    .flex_none()
                    .px(u(12.))
                    .pb(u(6.))
                    .pt(u(12.))
                    .child(search)
                    .when(!sub.is_empty(), |d| {
                        d.child(div().mt(u(8.)).px(u(4.)).truncate().text_size(u(12.)).text_color(p.mute).child(sub))
                    }),
            )
            .child(div().flex_1().min_h_0().when(self.searching, |d| d.opacity(0.6)).child(body))
    }

    fn list_skeleton(&self) -> AnyElement {
        let p = self.pal;
        div()
            .flex()
            .flex_col()
            .children([62., 48., 70., 54., 40., 66.].map(|w| {
                div()
                    .flex()
                    .flex_col()
                    .gap(u(9.))
                    .px(u(20.))
                    .py(u(13.))
                    .child(div().h(u(10.)).w(relative(w / 100.)).rounded(u(4.)).bg(p.raised))
                    .child(div().h(u(8.)).w(relative(0.34)).rounded(u(4.)).bg(p.raised))
            }))
            .into_any_element()
    }

    /// `action`: label and which action (0: search all sessions, 1: show all sessions).
    fn empty_state(&self, title: &str, body: Option<String>, action: Option<(&'static str, u8)>, cx: &mut Context<Self>) -> AnyElement {
        let p = self.pal;
        div()
            .flex()
            .flex_col()
            .items_center()
            .gap(u(6.))
            .px(u(24.))
            .py(u(48.))
            .text_center()
            .child(div().text_size(u(13.)).font_weight(MEDIUM).text_color(p.soft).child(title.to_string()))
            .children(body.map(|b| div().max_w(u(300.)).text_size(u(12.)).line_height(relative(1.55)).text_color(p.mute).child(b)))
            .children(action.map(|(label, which)| {
                plain_button("empty-action", label, p)
                    .mt(u(8.))
                    .font_weight(MEDIUM)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.range = Range::All;
                        if which == 0 {
                            this.repo = None;
                            this.agent = None;
                        }
                        this.refilter(true);
                        cx.notify();
                    }))
            }))
            .into_any_element()
    }

    /// One list line: a day heading or a session row.
    fn list_item(&mut self, ix: usize, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let p = self.pal;
        let Some(item) = self.items.get(ix).cloned() else { return div().into_any_element() };
        match item {
            Item::Header(label) => div()
                .w_full()
                .px(u(12.))
                .pb(u(5.))
                .pt(u(12.))
                .text_size(u(11.))
                .font_weight(MEDIUM)
                .text_color(p.mute)
                .child(label)
                .into_any_element(),
            Item::Row(ri) => {
                let Some(row) = self.rows.get(ri).cloned() else { return div().into_any_element() };
                let s = row.session.clone();
                let selected = self.sel_id.as_deref() == Some(s.id.as_str());
                let show_repo = self.repo.is_none();
                let show_agent = self.agent.is_none() && self.present.len() > 1;
                let title = &s.title;
                let id = s.id.clone();

                let time = rel_time(s.updated_at.as_deref(), self.now);
                let mut meta = div().flex().items_center().gap(u(6.)).min_w_0().text_size(u(12.)).text_color(p.mute);
                if show_repo {
                    if let Some(r) = &s.repo {
                        meta = meta.child(div().truncate().child(short_repo(r).to_string()));
                    }
                }
                if let Some(b) = &s.branch {
                    if show_repo && s.repo.is_some() {
                        meta = meta.child(div().child("·"));
                    }
                    meta = meta.child(div().min_w_0().truncate().font_family(MONO).text_size(u(11.)).child(b.clone()));
                }
                if s.source_deleted_at.is_some() {
                    meta = meta.child(
                        div()
                            .flex_none()
                            .whitespace_nowrap()
                            .rounded(u(4.))
                            .border_1()
                            .border_color(p.line)
                            .px(u(5.))
                            .text_size(u(11.))
                            .line_height(u(16.))
                            .text_color(p.mute)
                            .child("source deleted"),
                    );
                }
                meta = meta.child(
                    div()
                        .flex()
                        .flex_none()
                        .items_center()
                        .gap(u(10.))
                        .ml_auto()
                        .font_family(MONO)
                        .text_size(u(11.))
                        .when(show_agent, |d| d.child(agent_info(s.agent).short))
                        .child(s.message_count.to_string()),
                );

                let snippet = row.snippet().map(str::to_string).map(|raw| {
                    let segs = snippet_segments(&raw);
                    let mut text = String::new();
                    let mut hl = Vec::new();
                    for seg in &segs {
                        let start = text.len();
                        text.push_str(&seg.text);
                        if seg.hit {
                            hl.push((start..text.len(), HighlightStyle { background_color: Some(crate::ui::alpha(p.accent, 0.3)), color: Some(p.ink), ..Default::default() }));
                        }
                    }
                    (text, hl)
                });
                let matched = row.hit.as_ref().map(|h| (h.matched, h.file.clone()));

                div()
                    .id(("row", ix))
                    .w_full()
                    .flex()
                    .flex_col()
                    .gap(u(4.))
                    .mb(px(1.))
                    .rounded(u(10.))
                    .px(u(12.))
                    .py(u(9.))
                    .bg(if selected { p.sel } else { gpui_kit::transparent_black() })
                    .when(!selected, |d| d.hover(move |s| s.bg(p.hover)))
                    .cursor_default()
                    .on_click(cx.listener(move |this, ev: &ClickEvent, window, cx| {
                        window.focus(&this.focus, cx);
                        let _ = ev;
                        this.select(&id, cx);
                    }))
                    .child(
                        div()
                            .flex()
                            .items_baseline()
                            .gap(u(12.))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .truncate()
                                    .text_size(u(13.))
                                    .text_color(match title.source {
                                        TitleSource::Name => p.ink,
                                        TitleSource::Prompt => p.soft,
                                        TitleSource::None => p.mute,
                                    })
                                    .when(title.source == TitleSource::Name, |d| d.font_weight(MEDIUM))
                                    .when(title.source == TitleSource::None, |d| d.italic())
                                    .child(title.text.clone()),
                            )
                            .child(div().flex_none().font_family(MONO).text_size(u(11.)).text_color(p.mute).child(time)),
                    )
                    .child(meta)
                    .when_some(snippet, |d, (text, hl)| {
                        let styled = StyledText::new(text).with_highlights(hl);
                        match matched {
                            Some(("file", Some(f))) => d.child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(u(6.))
                                    .min_w_0()
                                    .text_size(u(12.))
                                    .text_color(p.soft)
                                    .child(icon("file", 12., p.mute))
                                    .child(div().flex_none().child(if f.created { "Created" } else { "Edited" }))
                                    .child(div().min_w_0().truncate().font_family(MONO).text_size(u(11.5)).child(styled))
                                    .when(f.edits > 1, |d| d.child(div().flex_none().child(format!("{} times", f.edits)))),
                            ),
                            Some(("session", _)) => d.child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(u(6.))
                                    .min_w_0()
                                    .text_size(u(12.))
                                    .text_color(p.soft)
                                    .child(div().flex_none().child("Session"))
                                    .child(div().min_w_0().truncate().font_family(MONO).text_size(u(11.5)).child(styled)),
                            ),
                            _ => d.child(div().text_size(u(12.)).line_height(relative(1.5)).text_color(p.soft).line_clamp(2).child(styled)),
                        }
                    })
                    .into_any_element()
            }
        }
    }
}
