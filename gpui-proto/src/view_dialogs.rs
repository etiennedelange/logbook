//! The overlays: About, What's New, the Sources popover and the notice toast.

use crate::app::Logbook;
use crate::ui::*;
use crate::view_chrome::*;
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::base::TextView;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use logbook_gpui_proto::model::agents::{agent_info, source_rows, AGENTS};
use logbook_gpui_proto::model::format::plural;
use std::time::Duration;

/// A long path keeps its end, where the folders that tell them apart are: "…/projects/my-repo".
fn tail(path: &str, keep: usize) -> String {
    let n = path.chars().count();
    if n <= keep {
        return path.to_string();
    }
    let rest: String = path.chars().skip(n - keep).collect();
    format!("…{rest}")
}

/// "just now", "4 min ago", "2 h ago".
fn ago(d: Duration) -> String {
    let mins = d.as_secs() / 60;
    if mins < 1 {
        "just now".into()
    } else if mins < 60 {
        format!("{mins} min ago")
    } else {
        format!("{} h ago", mins / 60)
    }
}

impl Logbook {
    /// A modal over the whole window: a dim backdrop that closes it, and the panel.
    fn modal(&self, id: &'static str, width: f32, max_h: f32, panel: Div, on_close: impl Fn(&mut Logbook, &mut Context<Logbook>) + Clone + 'static, cx: &mut Context<Self>) -> AnyElement {
        let p = self.pal;
        let close = on_close.clone();
        div()
            .id(id)
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(gpui_kit::black().opacity(0.5))
            .occlude()
            .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| close(this, cx)))
            .child(
                panel
                    .flex()
                    .flex_col()
                    .w(u(width))
                    .max_w(relative(0.95))
                    .max_h(u(max_h))
                    .overflow_hidden()
                    .rounded(u(12.))
                    .border_1()
                    .border_color(p.line)
                    .bg(p.bg)
                    .text_color(p.ink)
                    .shadow_2xl()
                    // A click inside the panel isn't a click on the backdrop.
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation()),
            )
            .into_any_element()
    }

    fn dialog_header(&self, glyph: &'static str, title: &str, sub: impl IntoElement, close_id: &'static str, cx: &mut Context<Self>, on_close: impl Fn(&mut Logbook, &mut Context<Logbook>) + 'static) -> Div {
        let p = self.pal;
        div()
            .flex()
            .flex_none()
            .items_center()
            .gap(u(10.))
            .h(u(60.))
            .pl(u(20.))
            .pr(u(12.))
            .border_b_1()
            .border_color(p.line)
            .child(div().flex().items_center().justify_center().size(u(26.)).flex_none().rounded(u(7.)).bg(p.accent).child(icon(glyph, 15., p.accent_ink)))
            .child(div().flex().flex_col().flex_1().min_w_0().gap(px(1.)).child(div().text_size(u(14.)).font_weight(SEMIBOLD).child(title.to_string())).child(sub))
            .child(
                icon_button(close_id, "close", 14., 28., p, p.btn)
                    .tooltip_text("Close (Esc)")
                    .on_click(cx.listener(move |this, _, _, cx| on_close(this, cx))),
            )
    }

    pub fn about_dialog(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let p = self.pal;
        let sources = self.sources.clone();
        let checking = self.about_checking;
        let sub = div()
            .flex()
            .items_center()
            .gap(u(8.))
            .text_size(u(12.))
            .text_color(p.mute)
            .child(format!("Version {}", logbook_core::VERSION))
            .child(
                div()
                    .id("check-updates")
                    .rounded(u(4.))
                    .px(u(4.))
                    .text_color(if checking { p.mute } else { p.accent_text })
                    .when(!checking, |d| d.cursor_pointer().hover(|s| s.underline()))
                    .when(!checking, |d| d.on_click(cx.listener(|this, _, _, cx| this.check_update(true, cx))))
                    .child(if checking { "Checking…" } else { "Check for updates" }),
            );

        let section = move |title: &'static str, children: Vec<AnyElement>| {
            div()
                .flex()
                .flex_col()
                .gap(u(10.))
                .mt(u(24.))
                .text_size(u(13.))
                .line_height(relative(1.6))
                .text_color(p.soft)
                .child(div().text_size(u(12.)).font_weight(MEDIUM).text_color(p.mute).child(title))
                .children(children)
                .into_any_element()
        };
        let para = move |text: &str| div().child(text.to_string()).into_any_element();

        let mut where_items: Vec<AnyElement> = vec![para(
            "Each agent saves every session on your machine: the folder and repository you were in, and the whole conversation. Logbook reads those folders. It never changes or deletes anything in them, and nothing leaves your machine.",
        )];
        let paths: Vec<(&'static str, String, bool)> = match &sources {
            Some(s) => s.iter().map(|x| (agent_info(x.agent).name, x.dir.clone(), true)).collect(),
            None => AGENTS.iter().map(|a| (a.name, a.home.to_string(), false)).collect(),
        };
        for (i, (label, value, openable)) in paths.into_iter().enumerate() {
            where_items.push(self.path_row(("about-path", i), label, &value, openable, cx));
        }
        where_items.push(para(
            "Claude Code deletes sessions after 30 days (its cleanupPeriodDays setting). Logbook keeps its own copy of each conversation, so a deleted session stays searchable and readable here, marked “source deleted”. It just can’t be resumed.",
        ));
        let cache = logbook_core::paths::index_cache_path().display().to_string();
        let cache_parent = std::path::Path::new(&cache).parent().map(|p| p.display().to_string()).unwrap_or_default();
        let stay: Vec<AnyElement> = vec![
            para("To open instantly, Logbook keeps a small index of its own. Whenever you come back to the window, or press r, it looks for new and changed sessions and reads only those. The index is safe to delete: Logbook rebuilds it from your sessions."),
            {
                let el = self.path_row(("about-path", 99), "Index", &cache, true, cx);
                let _ = cache_parent;
                el
            },
        ];
        let body = div()
            .id("about-scroll")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .track_scroll(&self.about_scroll)
            .px(u(20.))
            .pb(u(24.))
            .pt(u(20.))
            .child(div().text_size(u(13.5)).line_height(relative(1.6)).text_color(p.ink).child(
                "Logbook keeps a searchable record of your coding-agent sessions from the GitHub Copilot CLI and Claude Code, so you can find an old conversation and pick it up right where you left off.",
            ))
            .child(section("Where sessions come from", where_items))
            .child(section("Staying up to date", stay))
            .child(section(
                "Searching",
                vec![para("Search looks through session names, repositories and full transcripts. A session has to contain every word you type, and words match from their start, so flak finds “flaky”. Matches are highlighted in the list and in the transcript. Narrow things down further with a time range, an agent or a repository from the sidebar.")],
            ))
            .child(section(
                "Resuming",
                vec![para("Resume opens Windows Terminal in the session’s folder running the agent’s own resume command (copilot --resume=<id> or claude --resume <id>), so the conversation carries on with all its context. Copy puts that command on your clipboard instead.")],
            ))
            .child(section("Keyboard", vec![self.shortcuts().into_any_element()]));

        let panel = div()
            .child(self.dialog_header("logbook", "About Logbook", sub, "about-close", cx, |this, cx| this.set_about(false, cx)))
            .child(div().relative().flex_1().min_h_0().flex().flex_col().child(body).vertical_scrollbar(&self.about_scroll));
        self.modal("about", 540., 720., panel, |this, cx| this.set_about(false, cx), cx)
    }

    /// A labelled path with an "open folder" button.
    fn path_row(&self, id: (&'static str, usize), label: &str, value: &str, openable: bool, cx: &mut Context<Self>) -> AnyElement {
        let p = self.pal;
        let v = value.to_string();
        // For a file (the index), open its folder.
        let folder = if label == "Index" { std::path::Path::new(value).parent().map(|x| x.display().to_string()).unwrap_or_default() } else { value.to_string() };
        div()
            .flex()
            .items_center()
            .gap(u(12.))
            .rounded(u(6.))
            .border_1()
            .border_color(p.line_soft)
            .bg(p.inset)
            .py(u(6.))
            .pl(u(12.))
            .pr(u(6.))
            .child(div().flex_none().text_size(u(11.)).text_color(p.mute).child(label.to_string()))
            .child(div().flex_1().min_w_0().truncate().font_family(MONO).text_size(u(11.5)).text_color(p.soft).child(v))
            .when(openable, |d| {
                d.child(
                    icon_button(id, "folder", 13., 24., p, p.btn)
                        .tooltip_text("Open folder")
                        .on_click(cx.listener(move |this, _, _, cx| this.open_folder(&folder, cx))),
                )
            })
            .into_any_element()
    }

    pub fn whats_new_dialog(&mut self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let p = self.pal;
        let w = self.whats_new.clone()?;
        let sub = div().text_size(u(12.)).text_color(p.mute).child(format!("Logbook {} is available", w.version));
        let notes: AnyElement = match &w.notes {
            Some(n) => TextView::markdown("whats-new-notes", n.clone())
                .style(crate::view_detail::md_style(&p, false))
                .selectable(true)
                .into_any_element(),
            None => div().child("This release has no notes here. The release page lists what changed.").into_any_element(),
        };
        let version = w.version.clone();
        let version2 = w.version.clone();
        let footer = div()
            .flex()
            .flex_none()
            .flex_wrap()
            .items_center()
            .gap(u(8.))
            .border_t_1()
            .border_color(p.line)
            .px(u(20.))
            .py(u(12.))
            .child(
                div()
                    .id("release-page")
                    .mr_auto()
                    .flex()
                    .items_center()
                    .gap(u(6.))
                    .rounded(u(6.))
                    .px(u(6.))
                    .py(u(5.))
                    .text_size(u(12.))
                    .text_color(p.mute)
                    .cursor_pointer()
                    .hover(move |s| s.bg(p.btn).text_color(p.soft))
                    .on_click(cx.listener(move |this, _, _, cx| this.open_release_page(&version, cx)))
                    .child(icon("external", 13., p.mute))
                    .child("Release page"),
            )
            .child(
                div()
                    .flex()
                    .gap(u(8.))
                    .child(plain_button("whats-later", "Later", p).on_click(cx.listener(|this, _, _, cx| this.whats_new_close(cx))))
                    .child(
                        accent_button("whats-get", "Get this version", p)
                            .tooltip_text("Opens the release page: this build checks for updates but doesn’t install them")
                            .on_click(cx.listener(move |this, _, _, cx| this.open_release_page(&version2, cx))),
                    ),
            );
        let panel = div()
            .child(self.dialog_header("update", "What’s New", sub, "whats-close", cx, |this, cx| this.whats_new_close(cx)))
            .child(
                div().relative().flex_1().min_h_0().flex().flex_col().child(
                    div()
                        .id("whats-scroll")
                        .flex_1()
                        .min_h_0()
                        .overflow_y_scroll()
                        .track_scroll(&self.notes_scroll)
                        .px(u(20.))
                        .py(u(20.))
                        .text_size(u(13.))
                        .line_height(relative(1.6))
                        .text_color(p.soft)
                        .child(notes),
                )
                .vertical_scrollbar(&self.notes_scroll),
            )
            .child(footer);
        Some(self.modal("whats-new", 520., 640., panel, |this, cx| this.whats_new_close(cx), cx))
    }

    /// The folders Logbook reads, one per agent, with its sessions and when it last read them.
    pub fn sources_popover(&self, rail: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let p = self.pal;
        let rows = source_rows(self.sources.as_deref().unwrap_or(&[]), &self.sessions);
        let items = rows.into_iter().enumerate().map(|(ri, r)| {
            let info = agent_info(r.agent);
            let found = r.found;
            div()
                .flex()
                .items_start()
                .gap(u(10.))
                .rounded(u(8.))
                .p(u(10.))
                .child(
                    div()
                        .mt(u(6.))
                        .size(u(7.))
                        .flex_none()
                        .rounded_full()
                        .when(found, |d| d.bg(p.cat[info.cat]))
                        .when(!found, |d| d.border_1().border_color(p.mute)),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .flex_1()
                        .min_w_0()
                        .gap(u(3.))
                        .child(
                            div()
                                .flex()
                                .items_baseline()
                                .gap(u(8.))
                                .text_size(u(13.))
                                .text_color(if found { p.ink } else { p.mute })
                                .child(div().flex_1().min_w_0().truncate().child(info.name))
                                .child(div().whitespace_nowrap().when(found, |d| d.font_family(MONO).text_size(u(11.))).when(!found, |d| d.text_size(u(11.5))).text_color(p.mute).child(if !found {
                                    "Not found".to_string()
                                } else if r.count > 0 {
                                    plural(r.count, "session", "sessions")
                                } else {
                                    "No sessions yet".to_string()
                                })),
                        )
                        .children(r.dirs.iter().enumerate().map(|(di, d)| {
                            let dir = d.dir.clone();
                            div()
                                .flex()
                                .items_center()
                                .gap(u(4.))
                                .min_w_0()
                                .child(div().flex_1().min_w_0().truncate().font_family(MONO).text_size(u(11.5)).text_color(if d.found { p.soft } else { p.mute }).tooltip_text_static(d.dir.clone()).child(tail(&d.short, 34)))
                                .when(d.found, |e| {
                                    e.child(
                                        icon_button(("src-open", ri * 10 + di), "folder", 13., 24., p, p.btn)
                                            .tooltip_text(format!("Open {}", d.dir))
                                            .on_click(cx.listener(move |this, _, _, cx| this.open_folder(&dir, cx))),
                                    )
                                })
                        }))
                        .when(r.kept > 0, |d| {
                            d.child(div().text_size(u(11.5)).text_color(p.mute).child(format!("{} kept after {} deleted {}", r.kept, info.name, if r.kept == 1 { "it" } else { "them" })))
                        }),
                )
        });
        let status = if self.status.indexing {
            div().flex().items_center().gap(u(8.)).child(spinner("refresh", 11., p.accent_text, true)).child("Reading…")
        } else {
            div()
                .flex()
                .items_center()
                .gap(u(8.))
                .child(div().size(u(6.)).flex_none().rounded_full().bg(p.ok))
                .child(format!("Up to date{}", self.read_at.map(|t| format!(" · read {}", ago(t.elapsed()))).unwrap_or_default()))
        };
        div()
            .id("sources-popover")
            .absolute()
            .left(u(if rail { 60. } else { 8. }))
            .bottom(u(if rail { 10. } else { 56. }))
            .flex()
            .flex_col()
            .w(u(320.))
            .max_h(relative(0.8))
            .overflow_hidden()
            .rounded(u(12.))
            .border_1()
            .border_color(p.line)
            .bg(p.raised)
            .shadow_lg()
            .occlude()
            .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                if this.sources_open {
                    this.sources_open = false;
                    this.sources_closed_at = Some(std::time::Instant::now());
                    cx.notify();
                }
            }))
            .child(
                div()
                    .flex()
                    .flex_none()
                    .flex_col()
                    .gap(u(3.))
                    .px(u(16.))
                    .pb(u(10.))
                    .pt(u(14.))
                    .child(div().text_size(u(13.)).font_weight(SEMIBOLD).text_color(p.ink).child("Sources"))
                    .child(div().text_size(u(12.)).line_height(relative(1.45)).text_color(p.mute).child("Logbook reads these folders and never changes them.")),
            )
            .child(div().id("sources-list").flex().flex_col().flex_1().min_h_0().overflow_y_scroll().track_scroll(&self.sources_scroll).px(u(6.)).pb(u(6.)).children(items))
            .child(div().flex_none().border_t_1().border_color(p.line).px(u(16.)).py(u(10.)).text_size(u(11.5)).text_color(p.mute).child(status))
    }

    /// A short-lived confirmation or error, pinned to the bottom of the window.
    pub fn notice_toast(&self) -> Option<AnyElement> {
        let p = self.pal;
        let n = self.notice.as_ref()?;
        Some(
            div()
                .absolute()
                .bottom(u(16.))
                .left_0()
                .right_0()
                .flex()
                .justify_center()
                .child(
                    div()
                        .max_w(relative(0.9))
                        .rounded(u(6.))
                        .border_1()
                        .border_color(if n.error { p.fail } else { p.line })
                        .bg(p.raised)
                        .px(u(12.))
                        .py(u(7.))
                        .text_size(u(12.))
                        .text_color(if n.error { p.fail } else { p.soft })
                        .shadow_lg()
                        .child(n.text.clone()),
                )
                .into_any_element(),
        )
    }
}

/// A tooltip that takes `&'static`-free text on a non-stateful element: a no-op where GPUI can't hold one.
trait StaticTip: Sized {
    fn tooltip_text_static(self, _text: String) -> Self {
        self
    }
}
impl StaticTip for Div {}
