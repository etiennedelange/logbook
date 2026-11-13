//! The detail pane: the selected session's facts, transcript and files, and the resume dock.

use crate::app::{DItem, Logbook, MdEntry, Tab};
use crate::theme::Palette;
use crate::ui::*;
use crate::view_chrome::*;
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::base::{RangeHighlight, TextView, TextViewState, TextViewStyle};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use logbook_gpui_proto::model::agents::agent_info;
use logbook_gpui_proto::model::format::{self, clock, date_only, date_time, highlight_words, plural, short_repo};
use logbook_gpui_proto::diff::DiffItem;
use logbook_gpui_proto::transcript::{self, folded};
use logbook_gpui_proto::model::Session;
/// The reading column's width.
const COLUMN: f32 = 760.0;

/// How a session's changed files read in the facts: "2 created, 4 edited".
pub fn files_summary(s: &Session) -> Option<String> {
    let parts: Vec<String> = [(s.files_created, "created"), (s.files_edited, "edited")]
        .into_iter()
        .filter(|(n, _)| *n > 0)
        .map(|(n, w)| format!("{n} {w}"))
        .collect();
    (!parts.is_empty()).then(|| parts.join(", "))
}

/// Rendered markdown styled like the web UI's `.chat-md`.
pub fn md_style(p: &Palette, user: bool) -> TextViewStyle {
    let p = *p;
    let sz = |level: u8| match level {
        1 => 1.2,
        2 => 1.1,
        _ => 1.0,
    };
    TextViewStyle::default()
        .with_foreground(if user { p.ink } else { p.soft })
        .with_muted_foreground(p.mute)
        .with_link(p.accent_text)
        .with_selection(alpha(p.accent, 0.34))
        .with_code_background(p.inset)
        .with_border(p.line_soft)
        .with_paragraph_gap(rems(0.75))
        .with_heading(move |level| StyleRefinement::default().text_color(p.ink).font_weight(SEMIBOLD).text_size(rems(sz(level) * 13.5 / 16.0)))
        .with_inline_code_style(StyleRefinement::default().font_family(MONO).text_color(p.ink).bg(p.raised).rounded(u(4.)).px(u(5.)).py(px(1.)))
        .with_code_block(
            StyleRefinement::default()
                .bg(p.inset)
                .border_1()
                .border_color(p.line_soft)
                .rounded(u(6.))
                .px(u(12.))
                .py(u(10.))
                .font_family(MONO)
                .text_size(u(12.))
                .line_height(relative(1.55))
                .text_color(p.soft),
        )
        .with_blockquote_style(StyleRefinement::default().border_l_1().border_color(p.line).pl(u(12.)).text_color(p.mute))
        .with_table(StyleRefinement::default().bg(gpui_kit::transparent_black()).border_color(p.line_soft))
        .with_table_cell(StyleRefinement::default().bg(gpui_kit::transparent_black()).border_color(p.line_soft).text_size(u(12.5)))
        .with_table_row_hover(p.hover)
        .with_table_head(StyleRefinement::default().bg(p.raised).text_color(p.ink).font_weight(SEMIBOLD))
}

impl Logbook {
    pub fn detail_pane(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = self.pal;
        let wide = window.viewport_size().width / self.zoom.max(0.1) >= px(1400.);
        let end = self.header_end(None, wide, window, cx);
        let Some(detail) = &self.detail else { return self.nothing_selected(end, p, cx).into_any_element() };
        let Some(session) = self.by_id.get(&detail.id).cloned() else { return self.nothing_selected(end, p, cx).into_any_element() };

        let title = session.title.text.clone();
        let where_ = session.branch.clone().or_else(|| session.repo.as_deref().map(|r| short_repo(r).to_string())).unwrap_or_default();
        let pill_title = [session.repo.clone(), session.branch.clone()].into_iter().flatten().collect::<Vec<_>>().join(" · ");
        let end = self.header_end(Some((where_, pill_title)), wide, window, cx);

        // The latest of your messages that has scrolled out of view, pinned above the replies to it.
        let pinned = match (&detail.messages, detail.tab) {
            (Some(m), Tab::Transcript) if !m.is_empty() => {
                let top = detail.list.logical_scroll_top().item_ix;
                top.checked_sub(detail.msg_base).and_then(|t| transcript::pinned_prompt(m, t.max(1)).filter(|_| t > 0))
            }
            _ => None,
        };
        let pinned_bar = pinned.map(|ix| self.pinned_bar(ix, cx));

        let list_el = list(detail.list.clone(), cx.processor(|this, ix, window, cx| this.detail_item(ix, window, cx))).size_full();
        let scroll_list = detail.list.clone();

        div()
            .flex()
            .flex_col()
            .size_full()
            .min_w_0()
            .min_h_0()
            .child(self.pane_header("detail-title", title, 20., 12., end))
            .child(
                div()
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .child(list_el)
                    .children(pinned_bar)
                    .vertical_scrollbar(&scroll_list),
            )
            .child(self.resume_dock(&session, cx))
            .into_any_element()
    }

    /// The header's right end: the branch pill, the update pill, About and the window controls.
    pub fn header_end(&self, pill: Option<(String, String)>, wide: bool, window: &Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = self.pal;
        div()
            .flex()
            .flex_none()
            .items_center()
            .gap(u(10.))
            .self_stretch()
            .children(pill.filter(|(w, _)| !w.is_empty()).map(|(w, t)| {
                div()
                    .id("branch-pill")
                    .max_w(relative(0.4))
                    .flex_none()
                    .truncate()
                    .rounded_full()
                    .bg(p.raised)
                    .px(u(10.))
                    .py(u(2.))
                    .font_family(MONO)
                    .text_size(u(11.))
                    .text_color(p.soft)
                    .tooltip_text(t)
                    .child(w)
            }))
            .children(self.update_pill(wide, cx))
            .child(
                icon_button("about", "help", 15., 28., p, p.btn)
                    .tooltip_text("About Logbook (?)")
                    .on_click(cx.listener(|this, _, _, cx| this.set_about(true, cx))),
            )
            .child(self.window_controls(window))
    }

    /// In the toolbar once GitHub has a newer release; opens What's New. Nothing shows otherwise.
    pub fn update_pill(&self, wide: bool, cx: &mut Context<Self>) -> Option<impl IntoElement> {
        let p = self.pal;
        let u_ = self.update.clone()?;
        let version = u_.version.clone();
        Some(
            div()
                .id("update-pill")
                .flex()
                .flex_none()
                .items_center()
                .gap(u(6.))
                .h(u(22.))
                .rounded_full()
                .border_1()
                .border_color(alpha(p.accent, 0.4))
                .bg(alpha(p.accent, 0.12))
                .px(u(10.))
                .text_size(u(12.))
                .font_weight(MEDIUM)
                .text_color(p.accent_text)
                .cursor_pointer()
                .hover(move |s| s.bg(alpha(p.accent, 0.2)))
                .tooltip_text(format!("Logbook {version} is available"))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.whats_new = Some(crate::app::WhatsNew { version: u_.version.clone(), notes: u_.notes.clone() });
                    cx.notify();
                }))
                .child(icon("update", 13., p.accent_text))
                .child(if wide { format!("Update to {}", self.update.as_ref().map_or("", |u| u.version.as_str())) } else { "Update".to_string() }),
        )
    }

    fn nothing_selected(&self, end: impl IntoElement, p: Palette, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .child(self.pane_header("detail-title", "No session selected".into(), 20., 12., end))
            .child(
                div().flex().flex_1().items_center().justify_center().px(u(32.)).child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(u(16.))
                        .w_full()
                        .max_w(u(300.))
                        .child(div().text_size(u(13.)).line_height(relative(1.55)).text_color(p.soft).child(
                            "Search across every Copilot CLI and Claude Code conversation, then resume it right where you left off.",
                        ))
                        .child(self.shortcuts()),
                ),
            )
    }

    /// The keyboard shortcuts handled in `main.rs`, as a key / description list.
    pub fn shortcuts(&self) -> impl IntoElement {
        let p = self.pal;
        let multi = self.present.len() > 1;
        let mut rows: Vec<(Vec<&str>, &str, bool)> = vec![
            (vec!["/"], "Search names and transcripts", false),
            (vec!["↑", "↓"], "Move through sessions", false),
            (vec!["Enter"], "Resume the selected session", false),
            (vec!["1", "4"], "Pick a time range", true),
            (vec!["5"], "Your stats", false),
        ];
        if multi {
            rows.push((vec!["Alt+1", "Alt+2"], "Pick an agent", false));
        }
        rows.push((vec!["r"], "Re-index sessions", false));
        rows.push((vec!["Esc"], "Close the session", false));
        rows.push((vec!["?"], "About Logbook", false));
        div().flex().flex_col().gap(u(9.)).text_size(u(12.)).children(rows.into_iter().map(move |(keys, what, run)| {
            div()
                .flex()
                .items_center()
                .gap(u(16.))
                .child(
                    div().flex().items_center().gap(u(4.)).w(u(110.)).flex_none().children(keys.into_iter().enumerate().map(move |(i, k)| {
                        div()
                            .flex()
                            .items_center()
                            .gap(u(4.))
                            .when(i > 0, |d| d.child(div().text_color(p.mute).child(if run { "–" } else { "" })))
                            .child(
                                div()
                                    .min_w(u(20.))
                                    .rounded(u(4.))
                                    .border_1()
                                    .border_color(p.line)
                                    .bg(p.raised)
                                    .px(u(5.))
                                    .py(px(1.))
                                    .text_center()
                                    .font_family(MONO)
                                    .text_size(u(11.))
                                    .text_color(p.soft)
                                    .child(k.to_string()),
                            )
                    })),
                )
                .child(div().text_color(p.mute).child(what.to_string()))
        }))
    }

    // ---- The list's entries ------------------------------------------------------------------------

    fn detail_item(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let p = self.pal;
        let Some(d) = &self.detail else { return div().into_any_element() };
        let Some(item) = d.items.get(ix).cloned() else { return div().into_any_element() };
        let Some(session) = self.by_id.get(&d.id).cloned() else { return div().into_any_element() };
        let column = |el: AnyElement, max: f32| div().w_full().flex().justify_center().child(div().w_full().max_w(u(max)).px(u(20.)).child(el)).into_any_element();
        let el = match item {
            DItem::Facts => self.facts(&session, window, cx),
            DItem::Note => self.deleted_note(&session),
            DItem::Tabs => self.tabs(&session, cx),
            DItem::Skeleton => column(
                div()
                    .flex()
                    .flex_col()
                    .gap(u(24.))
                    .pt(u(20.))
                    .children((0..3).map(|_| {
                        div()
                            .flex()
                            .flex_col()
                            .gap(u(8.))
                            .child(div().h(u(8.)).w(u(60.)).rounded(u(4.)).bg(p.raised))
                            .child(div().h(u(10.)).w(relative(0.92)).rounded(u(4.)).bg(p.raised))
                            .child(div().h(u(10.)).w(relative(0.78)).rounded(u(4.)).bg(p.raised))
                            .child(div().h(u(10.)).w(relative(0.44)).rounded(u(4.)).bg(p.raised))
                    }))
                    .into_any_element(),
                COLUMN,
            ),
            DItem::Message(text) => {
                let error = text.starts_with("Couldn");
                div()
                    .px(u(20.))
                    .py(u(if error { 24. } else { 32. }))
                    .text_size(u(12.))
                    .text_color(if error { p.fail } else { p.mute })
                    .when(!error, |d| d.text_center())
                    .child(text)
                    .into_any_element()
            }
            DItem::MarksBanner => {
                let m = d.marks.matches;
                column(
                    div()
                        .pt(u(16.))
                        .pb(u(12.))
                        .text_size(u(11.))
                        .text_color(p.mute)
                        .child(if m > 0 {
                            format!("{} your search", plural(m, "message matches", "messages match"))
                        } else {
                            "No messages match your search; the match is in the session’s name or details.".to_string()
                        })
                        .into_any_element(),
                    COLUMN,
                )
            }
            DItem::Msg(mi) => self.message(mi, &session, window, cx),
            DItem::Pad => div().h(u(40.)).into_any_element(),
            DItem::Heading(label) => column(div().pt(u(20.)).mb(u(8.)).ml(u(10.)).text_size(u(11.)).font_weight(MEDIUM).text_color(p.mute).child(label).into_any_element(), 800.),
            DItem::FileRow(fi) => self.file_row(fi, cx),
            DItem::NoFiles => column(
                div().pt(u(16.)).mx(u(10.)).text_size(u(12.5)).text_color(p.mute).child(format!("{} didn’t change any files with its file tools.", agent_info(session.agent).name)).into_any_element(),
                800.,
            ),
            DItem::Commands => self.commands_card(),
            DItem::Footnote => column(
                div()
                    .pt(u(20.))
                    .mx(u(10.))
                    .text_size(u(11.5))
                    .line_height(relative(1.55))
                    .text_color(p.mute)
                    .child(format!("From {}’s edit records. This is what changed during the session, not the files as they are now.", agent_info(session.agent).name))
                    .into_any_element(),
                800.,
            ),
            DItem::EditsHead => self.edits_head(cx),
            DItem::Diff(di) => self.diff_line(di),
        };
        // Entries are laid out with only the list's width to go on, so each takes all of it.
        div().w_full().child(el).into_any_element()
    }

    fn facts(&self, s: &Session, _window: &mut Window, _cx: &mut Context<Self>) -> AnyElement {
        let p = self.pal;
        let files = files_summary(s);
        let mut top = vec![
            ("Last active", date_time(s.updated_at.as_deref())),
            ("Started", date_time(s.created_at.as_deref())),
            ("Messages", s.message_count.to_string()),
        ];
        if let Some(f) = files {
            top.push(("Files", f));
        }
        let folder = s.cwd.clone().or_else(|| s.git_root.clone());
        let mut paths: Vec<(&str, String)> = Vec::new();
        if let Some(f) = &folder {
            paths.push(("Folder", f.clone()));
        }
        paths.push(("Session", s.native_id.clone()));
        let id_match = self.hits.as_ref().is_some_and(|h| h.iter().any(|h| h.id == s.id && h.matched == "session"));
        let multi = self.present.len() > 1;
        let info = agent_info(s.agent);

        div()
            .flex()
            .flex_col()
            .gap(u(12.))
            .border_b_1()
            .border_color(p.line)
            .px(u(20.))
            .py(u(16.))
            .child(
                div().flex().flex_wrap().gap_x(u(40.)).gap_y(u(12.)).children(top.into_iter().map(|(label, value)| {
                    div()
                        .flex()
                        .flex_col()
                        .gap(u(3.))
                        .min_w_0()
                        .child(div().text_size(u(11.)).text_color(p.mute).child(label))
                        .child(div().truncate().text_size(u(12.5)).text_color(p.soft).child(value))
                })),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(u(5.))
                    .when(multi, |d| {
                        d.child(
                            div()
                                .flex()
                                .items_center()
                                .gap(u(16.))
                                .child(div().w(u(48.)).flex_none().text_size(u(11.)).text_color(p.mute).child("Agent"))
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(u(5.))
                                        .text_size(u(12.))
                                        .text_color(p.soft)
                                        .child(div().size(u(6.)).flex_none().rounded_full().bg(p.cat[info.cat]))
                                        .child(info.name),
                                ),
                        )
                    })
                    .children(paths.into_iter().map(|(label, value)| {
                        let marked = id_match && label == "Session";
                        div()
                            .flex()
                            .items_baseline()
                            .gap(u(16.))
                            .child(div().w(u(48.)).flex_none().text_size(u(11.)).text_color(p.mute).child(label))
                            .child(
                                div()
                                    .min_w_0()
                                    .truncate()
                                    .font_family(MONO)
                                    .text_size(u(11.5))
                                    .text_color(p.soft)
                                    .when(marked, |d| d.bg(alpha(p.accent, 0.3)).text_color(p.ink).rounded(u(2.)))
                                    .child(value),
                            )
                    })),
            )
            .into_any_element()
    }

    /// Why a session whose file is gone can still be read but not resumed.
    fn deleted_note(&self, s: &Session) -> AnyElement {
        let p = self.pal;
        let info = agent_info(s.agent);
        let when = s.source_deleted_at.as_deref().map(date_only);
        let lead = format!("{} deleted this session’s file{}.", info.name, when.map(|w| format!(" by {w}")).unwrap_or_default());
        let mut text = format!("{lead} Logbook kept the messages, so you can still find and read them, but the session can’t be resumed.");
        if let Some((days, setting)) = info.retention {
            text.push_str(&format!(" {} keeps sessions for {days} days unless {setting} in its settings says otherwise.", info.name));
        }
        let styled = StyledText::new(text).with_highlights(vec![(0..lead.len(), HighlightStyle { color: Some(p.ink), font_weight: Some(MEDIUM), ..Default::default() })]);
        div()
            .mx(u(20.))
            .mt(u(16.))
            .rounded(u(6.))
            .border_1()
            .border_color(p.line)
            .bg(p.raised)
            .px(u(12.))
            .py(u(10.))
            .text_size(u(12.5))
            .line_height(relative(1.55))
            .text_color(p.soft)
            .child(styled)
            .into_any_element()
    }

    /// Transcript | Files, under the session's facts. Shown only for a session that changed files.
    fn tabs(&self, s: &Session, cx: &mut Context<Self>) -> AnyElement {
        let p = self.pal;
        let tab = self.detail.as_ref().map_or(Tab::Transcript, |d| d.tab);
        let n = s.files_created + s.files_edited;
        let item = |id: &'static str, label: &'static str, key: Tab, count: Option<usize>, cx: &mut Context<Self>| {
            let on = tab == key;
            div()
                .id(id)
                .flex()
                .items_center()
                .gap(u(6.))
                .h(u(38.))
                .px(u(10.))
                .border_b_2()
                .border_color(if on { p.accent } else { gpui_kit::transparent_black() })
                .text_size(u(12.5))
                .text_color(if on { p.ink } else { p.mute })
                .when(on, |d| d.font_weight(MEDIUM))
                .cursor_pointer()
                .when(!on, |d| d.hover(move |s| s.text_color(p.soft)))
                .on_click(cx.listener(move |this, _, _, cx| this.set_detail_tab(key, cx)))
                .child(label)
                .children(count.map(|c| div().rounded_full().bg(p.btn).px(u(6.)).text_size(u(11.)).text_color(p.soft).child(c.to_string())))
        };
        div()
            .flex()
            .gap(u(4.))
            .border_b_1()
            .border_color(p.line)
            .px(u(16.))
            .child(item("tab-transcript", "Transcript", Tab::Transcript, None, cx))
            .child(item("tab-files", "Files", Tab::Files, Some(n), cx))
            .into_any_element()
    }

    // ---- Messages ------------------------------------------------------------------------------------

    fn message(&mut self, mi: usize, s: &Session, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let p = self.pal;
        let weak = cx.entity().downgrade();
        let Some(d) = self.detail.as_mut() else { return div().into_any_element() };
        let Some(messages) = d.messages.clone() else { return div().into_any_element() };
        let Some(m) = messages.get(mi) else { return div().into_any_element() };
        let long = m.long;
        let has_hit = d.marks.hit_at.get(mi).copied().unwrap_or(false);
        // Follows the search (a match unfolds it) until the user folds or unfolds it themselves.
        let open = d.folds.get(&mi).copied().unwrap_or(!long || has_hit);
        // The text is only built when the view is made or the message folds or unfolds, not on every frame.
        let shown = |open: bool| if open { m.text.clone() } else { folded(&m.text) };
        let entry = d.mds.entry(mi).or_insert_with(|| MdEntry { state: cx.new(|cx| TextViewState::markdown(&shown(open), cx)), open, marked: None });
        if entry.open != open {
            let text = shown(open);
            entry.state.update(cx, |st, cx| st.set_text(&text, cx));
            entry.open = open;
            entry.marked = None;
        }
        // Messages without a match render unmarked, so a new search leaves them alone.
        let want = if has_hit { format!("{}|{}", d.marks.key, p.dark) } else { String::new() };
        let rendered = entry.state.read(cx).rendered_text();
        if entry.marked.as_ref().map(|(k, r)| (k.as_str(), r)) != Some((want.as_str(), &rendered)) {
            let had = entry.marked.as_ref().is_some_and(|(k, _)| !k.is_empty());
            if has_hit {
                let mut at = 0;
                let mut ranges = Vec::new();
                for seg in highlight_words(rendered.as_str(), &d.marks.tokens, d.marks.mode) {
                    if seg.hit {
                        ranges.push(RangeHighlight::new(at..at + seg.text.len(), alpha(p.accent, 0.3)));
                    }
                    at += seg.text.len();
                }
                entry.state.update(cx, |st, cx| {
                    let _ = st.set_range_highlights(ranges, cx);
                });
            } else if had {
                entry.state.update(cx, |st, cx| st.clear_range_highlights(cx));
            }
            entry.marked = Some((want, rendered));
        }
        let state = entry.state.clone();

        let user = m.user;
        let speaker = agent_info(s.agent).speaker;
        let ts = m.ts.clone();
        let when = clock(ts.as_deref());
        let day = m.day.clone();
        let chars = m.chars;

        let view = TextView::new(&state)
            .selectable(true)
            .style(md_style(&p, user))
            .on_link_click(move |url, _ev, _window, cx| {
                let url = url.to_string();
                weak.update(cx, |this, cx| this.open_link(&url, cx)).ok();
            });

        let body = div()
            .min_w_0()
            .text_size(u(13.5))
            .line_height(relative(1.6))
            .text_color(if user { p.ink } else { p.soft })
            .when(user, |d| d.max_w(relative(0.85)).self_end().rounded(u(18.)).bg(p.sel).px(u(16.)).py(u(10.)))
            .child(view);

        let fold_button = long.then(|| {
            div()
                .id(("fold", mi))
                .rounded(u(6.))
                .px(u(6.))
                .py(u(2.))
                .text_size(u(12.))
                .font_weight(MEDIUM)
                .text_color(p.accent_text)
                .cursor_pointer()
                .hover(move |s| s.bg(p.hover))
                .when(user, |d| d.self_end())
                .when(!user, |d| d.self_start())
                .on_click(cx.listener(move |this, _, _, cx| this.toggle_fold(mi, open, cx)))
                .child(if open { "Show less".to_string() } else { format!("Show full message ({}k characters)", (chars as f32 / 1000.0).round() as usize) })
        });

        div()
            .w_full()
            .flex()
            .justify_center()
            .px(u(20.))
            .when(mi == 0, |d| d.pt(u(16.)))
            .pb(u(20.))
            .child(
                div()
                    .w_full()
                    .max_w(u(COLUMN))
                    .flex()
                    .flex_col()
                    .gap(u(6.))
                    .children(day.map(|label| {
                        div()
                            .flex()
                            .items_center()
                            .gap(u(12.))
                            .pb(u(4.))
                            .pt(u(8.))
                            .text_size(u(11.))
                            .text_color(p.mute)
                            .child(div().h(px(1.)).flex_1().bg(p.line_soft))
                            .child(label)
                            .child(div().h(px(1.)).flex_1().bg(p.line_soft))
                    }))
                    .child(
                        div()
                            .flex()
                            .items_baseline()
                            .gap(u(8.))
                            .text_size(u(11.))
                            .when(user, |d| d.justify_end())
                            .when(!user, |d| d.child(div().font_weight(SEMIBOLD).text_color(p.soft).child(speaker)))
                            .child(div().font_family(MONO).text_color(p.mute).child(when)),
                    )
                    .child(body)
                    .children(fold_button),
            )
            .into_any_element()
    }

    /// Your last message above the fold, as one line across the top of the transcript. Click it to read
    /// more of it, with a button that jumps back to the message itself.
    fn pinned_bar(&self, ix: usize, cx: &mut Context<Self>) -> AnyElement {
        let p = self.pal;
        let Some(d) = &self.detail else { return div().into_any_element() };
        let Some(m) = d.messages.as_ref().and_then(|m| m.get(ix)).cloned() else { return div().into_any_element() };
        let open = d.pinned_open;
        let one_line: String = m.text.split_whitespace().collect::<Vec<_>>().join(" ");
        div()
            .absolute()
            .top_0()
            .left_0()
            .right_0()
            .border_b_1()
            .border_color(p.line)
            .bg(p.raised)
            .when(open, |d| d.shadow_lg())
            .child(
                div()
                    .id("pinned")
                    .flex()
                    .items_center()
                    .gap(u(8.))
                    .h(u(36.))
                    .w_full()
                    .px(u(20.))
                    .text_color(p.mute)
                    .cursor_pointer()
                    .hover(move |s| s.text_color(p.soft))
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_pinned(cx)))
                    .child(icon("up", 13., p.mute))
                    .child(div().flex_none().text_size(u(11.)).font_weight(SEMIBOLD).text_color(p.soft).child("You"))
                    .child(div().flex_1().min_w_0().truncate().text_size(u(12.5)).text_color(p.ink).child(if open { String::new() } else { one_line }))
                    .child(div().flex_none().font_family(MONO).text_size(u(11.)).child(clock(m.ts.as_deref())))
                    .child(icon(if open { "chevron-up" } else { "chevron-down" }, 13., p.mute)),
            )
            .when(open, |d| {
                d.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(u(8.))
                        .px(u(20.))
                        .pb(u(12.))
                        .child(div().text_size(u(13.)).line_height(relative(1.55)).text_color(p.ink).line_clamp(4).child(m.text.clone()))
                        .child(
                            div()
                                .id("pinned-jump")
                                .h(u(30.))
                                .self_end()
                                .flex()
                                .items_center()
                                .rounded_full()
                                .border_1()
                                .border_color(p.line)
                                .bg(p.btn)
                                .px(u(12.))
                                .text_size(u(12.))
                                .font_weight(MEDIUM)
                                .text_color(p.accent_text)
                                .cursor_pointer()
                                .hover(move |s| s.bg(p.btn_hover))
                                .on_click(cx.listener(move |this, _, _, cx| this.jump_to_message(ix, cx)))
                                .child("Jump to message"),
                        ),
                )
            })
            .into_any_element()
    }

    // ---- Files --------------------------------------------------------------------------------------

    fn counts(&self, added: u32, removed: u32) -> impl IntoElement {
        let p = self.pal;
        div()
            .flex()
            .flex_none()
            .items_center()
            .child(div().w(u(44.)).text_right().font_family(MONO).text_size(u(11.5)).text_color(p.ok).child(if added > 0 { format!("+{added}") } else { String::new() }))
            .child(div().w(u(44.)).text_right().font_family(MONO).text_size(u(11.5)).text_color(p.fail).child(if removed > 0 { format!("−{removed}") } else { String::new() }))
    }

    fn file_row(&mut self, fi: usize, cx: &mut Context<Self>) -> AnyElement {
        let p = self.pal;
        let Some(f) = self.detail.as_ref().and_then(|d| d.files.as_ref()).and_then(|f| f.files.get(fi)).cloned() else { return div().into_any_element() };
        let tokens = format::query_tokens(&if self.hits.is_some() { self.hits_query.clone() } else { String::new() });
        let mode = format::match_mode(&f.path, &tokens);
        let cut = f.path.rfind(['/', '\\']).map_or(0, |i| i + 1);
        let path_el: AnyElement = {
            let segs = highlight_words(&f.path, &tokens, mode);
            if mode.is_some() && segs.iter().any(|s| s.hit) {
                let mut text = String::new();
                let mut hl = Vec::new();
                for s in &segs {
                    let start = text.len();
                    text.push_str(&s.text);
                    if s.hit {
                        hl.push((start..text.len(), HighlightStyle { background_color: Some(alpha(p.accent, 0.3)), color: Some(p.ink), ..Default::default() }));
                    }
                }
                div().text_color(p.soft).child(StyledText::new(text).with_highlights(hl)).into_any_element()
            } else {
                let dir = f.path[..cut].to_string();
                let name = f.path[cut..].to_string();
                let styled = StyledText::new(format!("{dir}{name}")).with_highlights(vec![
                    (0..dir.len(), HighlightStyle { color: Some(p.mute), ..Default::default() }),
                    (dir.len()..dir.len() + name.len(), HighlightStyle { color: Some(p.ink), ..Default::default() }),
                ]);
                div().child(styled).into_any_element()
            }
        };
        let path = f.path.clone();
        let edits = if f.edits > 1 { plural(f.edits as usize, "edit", "edits") } else { String::new() };
        div()
            .w_full()
            .flex()
            .justify_center()
            .px(u(20.))
            .child(
                div()
                    .id(("file", fi))
                    .w_full()
                    .max_w(u(800.))
                    .flex()
                    .items_center()
                    .gap(u(12.))
                    .rounded(u(8.))
                    .px(u(10.))
                    .py(u(8.))
                    .cursor_pointer()
                    .hover(move |s| s.bg(p.hover))
                    .tooltip_text(format!("Show {}’s edits", f.path))
                    .on_click(cx.listener(move |this, _, _, cx| this.open_file(Some(path.clone()), cx)))
                    .child(icon("file", 14., p.mute))
                    .child(div().flex_1().min_w_0().truncate().font_family(MONO).text_size(u(12.)).child(path_el))
                    .child(div().flex_none().whitespace_nowrap().text_size(u(11.)).text_color(p.mute).child(edits))
                    .child(self.counts(f.added, f.removed)),
            )
            .into_any_element()
    }

    fn commands_card(&self) -> AnyElement {
        let p = self.pal;
        let Some(f) = self.detail.as_ref().and_then(|d| d.files.as_ref()) else { return div().into_any_element() };
        div()
            .w_full()
            .flex()
            .justify_center()
            .px(u(20.))
            .pt(u(20.))
            .child(
                div()
                    .w_full()
                    .max_w(u(800.))
                    .flex()
                    .flex_col()
                    .gap(u(6.))
                    .child(div().ml(u(10.)).text_size(u(11.)).font_weight(MEDIUM).text_color(p.mute).child("Commands that may have changed files"))
                    .child(
                        div()
                            .mx(u(10.))
                            .flex()
                            .flex_col()
                            .gap(u(6.))
                            .rounded(u(8.))
                            .border_1()
                            .border_color(p.line)
                            .bg(p.raised)
                            .px(u(12.))
                            .py(u(10.))
                            .children(f.commands.iter().map(|c| div().font_family(MONO).text_size(u(11.5)).text_color(p.soft).child(c.clone())))
                            .child(div().mt(u(4.)).text_size(u(11.5)).text_color(p.mute).child("Logbook sees these commands, but not which files they changed.")),
                    ),
            )
            .into_any_element()
    }

    fn edits_head(&self, cx: &mut Context<Self>) -> AnyElement {
        let p = self.pal;
        let Some(d) = &self.detail else { return div().into_any_element() };
        let Some(f) = d.file.as_ref().and_then(|path| d.files.as_ref().and_then(|f| f.files.iter().find(|x| &x.path == path))).cloned() else { return div().into_any_element() };
        div()
            .w_full()
            .flex()
            .justify_center()
            .px(u(20.))
            .pt(u(14.))
            .pb(u(14.))
            .child(
                div()
                    .w_full()
                    .max_w(u(840.))
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(u(10.))
                    .child(
                        div()
                            .id("edits-back")
                            .flex()
                            .items_center()
                            .gap(u(4.))
                            .h(u(32.))
                            .rounded(u(8.))
                            .bg(p.btn)
                            .pl(u(6.))
                            .pr(u(10.))
                            .text_size(u(12.5))
                            .text_color(p.soft)
                            .cursor_pointer()
                            .hover(move |s| s.bg(p.btn_hover))
                            .on_click(cx.listener(|this, _, _, cx| this.open_file(None, cx)))
                            .child(icon("chevron-left", 14., p.soft))
                            .child("Files"),
                    )
                    .child(div().flex_1().min_w_0().font_family(MONO).text_size(u(12.5)).text_color(p.ink).child(f.path.clone()))
                    .child(self.counts(f.added, f.removed)),
            )
            .into_any_element()
    }

    /// One line of a file's edit cards: the head, a numbered +/- line, a gap between hunks, or the bottom edge.
    fn diff_line(&self, di: DiffItem) -> AnyElement {
        let p = self.pal;
        let card = |el: Div| div().w_full().flex().justify_center().px(u(20.)).child(el.w_full().max_w(u(840.)));
        match di {
            DiffItem::Head { label, ts } => card(
                div()
                    .flex()
                    .items_center()
                    .gap(u(10.))
                    .rounded_t(u(10.))
                    .border_1()
                    .border_color(p.line)
                    .bg(p.raised)
                    .px(u(12.))
                    .py(u(8.))
                    .child(div().text_size(u(12.)).font_weight(MEDIUM).text_color(p.ink).child(label))
                    .children(ts.map(|t| div().font_family(MONO).text_size(u(11.)).text_color(p.mute).child(clock(Some(&t))))),
            )
            .into_any_element(),
            DiffItem::Gap => card(div().border_x_1().border_color(p.line).px(u(12.)).font_family(MONO).text_size(u(11.)).line_height(relative(1.7)).text_color(p.mute).child("⋯")).into_any_element(),
            DiffItem::Note(t) => card(div().border_x_1().border_color(p.line).px(u(12.)).py(u(4.)).text_size(u(12.)).text_color(p.mute).child(t)).into_any_element(),
            DiffItem::End => card(div().h(u(6.)).rounded_b(u(10.)).border_x_1().border_b_1().border_color(p.line)).pb(u(14.)).into_any_element(),
            DiffItem::Line { no, op, text } => {
                let bg = match op {
                    '+' => alpha(p.ok, 0.13),
                    '-' => alpha(p.fail, 0.13),
                    _ => gpui_kit::transparent_black(),
                };
                let opc = match op {
                    '+' => p.ok,
                    '-' => p.fail,
                    _ => p.mute,
                };
                card(
                    div()
                        .flex()
                        .overflow_hidden()
                        .border_x_1()
                        .border_color(p.line)
                        .bg(bg)
                        .whitespace_nowrap()
                        .font_family(MONO)
                        .text_size(u(11.5))
                        .line_height(relative(1.7))
                        .child(div().w(u(44.)).flex_none().pr(u(8.)).text_right().text_color(p.mute).child(if no > 0 { no.to_string() } else { String::new() }))
                        .child(div().w(u(18.)).flex_none().text_color(opc).child(match op {
                            '+' => "+",
                            '-' => "−",
                            _ => "",
                        }))
                        .child(div().flex_1().min_w_0().text_color(p.soft).child(text)),
                )
                .into_any_element()
            }
        }
    }

    // ---- The resume dock -------------------------------------------------------------------------------

    fn resume_dock(&self, s: &Session, cx: &mut Context<Self>) -> impl IntoElement {
        let p = self.pal;
        let info = agent_info(s.agent);
        let deleted = s.source_deleted_at.is_some();
        let folder = s.cwd.clone().or_else(|| s.git_root.clone());
        let command = info.resume.replace("<id>", &s.native_id);
        let line = match &folder {
            Some(f) => format!("{f} › {command}"),
            None => command.clone(),
        };
        let cmd_start = line.len() - command.len();
        let styled = StyledText::new(line.clone()).with_highlights(vec![(cmd_start..line.len(), HighlightStyle { color: Some(p.soft), ..Default::default() })]);
        let (id1, id2) = (s.id.clone(), s.id.clone());
        let folder2 = folder.clone();
        let round = move |id: &'static str, name: &'static str| {
            div()
                .id(id)
                .flex()
                .flex_none()
                .items_center()
                .justify_center()
                .size(u(32.))
                .rounded_full()
                .border_1()
                .border_color(p.line)
                .text_color(p.soft)
                .cursor_pointer()
                .hover(move |s| s.bg(p.btn_hover).text_color(p.ink))
                .child(icon(name, 14., p.soft))
        };
        div().flex_none().px(u(20.)).pb(u(16.)).pt(u(8.)).child(
            div().w_full().flex().justify_center().child(
                div()
                    .w_full()
                    .max_w(u(COLUMN))
                    .flex()
                    .items_center()
                    .gap(u(8.))
                    .rounded(u(16.))
                    .border_1()
                    .border_color(p.line)
                    .bg(p.raised)
                    .py(u(7.))
                    .pl(u(16.))
                    .pr(u(7.))
                    .child(div().flex_1().min_w_0().truncate().font_family(MONO).text_size(u(11.5)).text_color(p.mute).child(styled))
                    .child(round("copy-resume", "copy").tooltip_text("Copy resume command").on_click(cx.listener(move |this, _, _, cx| this.copy_resume(&id1, cx))))
                    .children(folder2.map(|f| {
                        let f2 = f.clone();
                        round("open-folder", "folder").tooltip_text(format!("Open {f}")).on_click(cx.listener(move |this, _, _, cx| this.open_folder(&f2, cx)))
                    }))
                    .child(
                        div()
                            .id("resume")
                            .flex()
                            .flex_none()
                            .items_center()
                            .gap(u(6.))
                            .h(u(32.))
                            .rounded_full()
                            .px(u(14.))
                            .text_size(u(12.5))
                            .font_weight(SEMIBOLD)
                            .when(!deleted, |d| d.bg(p.accent).text_color(p.accent_ink).cursor_pointer().hover(|s| s.opacity(0.92)))
                            .when(deleted, |d| d.bg(p.btn).text_color(p.mute))
                            .tooltip_text(if deleted {
                                format!("{} deleted this session, so it can’t be resumed", info.name)
                            } else {
                                format!("Open a terminal here running {} (Enter on the row)", info.resume)
                            })
                            .when(!deleted, |d| d.on_click(cx.listener(move |this, _, _, cx| this.resume(&id2, cx))))
                            .child(icon("terminal", 13., if deleted { p.mute } else { p.accent_ink }))
                            .child("Resume"),
                    ),
            ),
        )
    }
}
