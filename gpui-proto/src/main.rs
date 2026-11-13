// No console window behind the app in a release build on Windows.
#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

//! Logbook in GPUI: the whole app over `logbook-core`, no webview and no IPC. An experiment next to the
//! Tauri app (see "Experiment: a GPUI front end" in docs/PLAN.md); the web UI in `ui/` stays the shipping one.

mod app;
mod assets;
mod platform;
mod theme;
mod ui;
mod view_chrome;
mod view_detail;
mod view_dialogs;
mod view_list;
mod view_sidebar;
mod view_stats;

use app::{Logbook, View};
use gpui_kit::*;
use logbook_core::service::SessionService;
use logbook_gpui_proto::model::select::RANGES;
use std::sync::Arc;
use ui::*;

impl Logbook {
    fn on_key(&mut self, ev: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let key = ev.keystroke.key.as_str();
        let m = ev.keystroke.modifiers;
        let typing = self.input.read(cx).focus_handle(cx).is_focused(window) || self.repo_input.read(cx).focus_handle(cx).is_focused(window);

        if m.control && !m.alt && !m.platform && key == "0" {
            self.set_zoom(1.0, window, cx);
            cx.stop_propagation();
            return;
        }
        // The dialogs are modal: they close themselves on Esc, and nothing behind them reacts.
        if self.about || self.whats_new.is_some() {
            if key == "escape" {
                if self.about {
                    self.set_about(false, cx);
                } else {
                    self.whats_new_close(cx);
                }
            }
            return;
        }
        // Alt+1, Alt+2, ... pick an agent in sidebar order (again to clear it). Alt types nothing, so
        // this works in the search box too.
        if m.alt && !m.platform && !m.control {
            if let Some(n) = key.parse::<usize>().ok().filter(|n| (1..=9).contains(n)) {
                if let Some(agent) = self.present.get(n - 1).copied().filter(|_| self.present.len() > 1) {
                    let next = if self.agent == Some(agent) { None } else { Some(agent) };
                    self.set_agent(next, cx);
                    cx.stop_propagation();
                }
            }
            return;
        }
        if typing {
            match key {
                "escape" => {
                    let search = self.input.read(cx).focus_handle(cx).is_focused(window);
                    let (input, has_text) = if search { (self.input.clone(), !self.query.is_empty()) } else { (self.repo_input.clone(), !self.repo_query.is_empty()) };
                    if has_text {
                        input.update(cx, |i, cx| i.set_value("", window, cx));
                        if search {
                            self.set_query(String::new(), cx);
                        } else {
                            self.repo_query.clear();
                            cx.notify();
                        }
                    } else {
                        window.focus(&self.focus, cx);
                    }
                    cx.stop_propagation();
                }
                "down" if self.input.read(cx).focus_handle(cx).is_focused(window) => {
                    self.move_selection(1, cx);
                    cx.stop_propagation();
                }
                "up" if self.input.read(cx).focus_handle(cx).is_focused(window) => {
                    self.move_selection(-1, cx);
                    cx.stop_propagation();
                }
                _ => {}
            }
            return;
        }
        if m.control || m.alt || m.platform {
            return;
        }
        let ch = ev.keystroke.key_char.as_deref();
        match (key, ch) {
            (_, Some("?")) => self.set_about(true, cx),
            ("escape", _) if self.view == View::Stats => self.set_view(View::Sessions, cx),
            ("escape", _) if self.sources_open => {
                self.sources_open = false;
                cx.notify();
            }
            ("escape", _) => self.clear_selection(cx),
            ("/", _) => {
                if self.view == View::Stats {
                    self.set_view(View::Sessions, cx);
                }
                let input = self.input.clone();
                input.update(cx, |i, cx| {
                    i.focus(window, cx);
                    i.select_all(window, cx);
                });
                window.prevent_default();
                cx.stop_propagation();
            }
            ("5", _) => {
                let v = if self.view == View::Stats { View::Sessions } else { View::Stats };
                self.set_view(v, cx);
            }
            ("r", _) => self.reindex(cx),
            ("down", _) => self.move_selection(1, cx),
            ("up", _) => self.move_selection(-1, cx),
            ("home", _) => self.move_selection(i32::MIN, cx),
            ("end", _) => self.move_selection(i32::MAX, cx),
            ("enter", _) => {
                if let Some(id) = self.sel_id.clone() {
                    self.resume(&id, cx);
                }
            }
            (k, _) => {
                if let Some(r) = RANGES.iter().find(|r| k.chars().next() == Some(r.hotkey) && k.len() == 1) {
                    self.set_range(r.key, cx);
                }
            }
        }
    }
}

impl Render for Logbook {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = self.pal;
        // Layout runs in the web UI's CSS pixels: the window's size divided by the zoom.
        let vw = f32::from(window.viewport_size().width) / self.zoom.max(0.1);
        let rail = vw < view_sidebar::RAIL_BELOW;
        let avail = vw - if rail { 56.0 } else { 220.0 };
        // The list gives way before the detail pane does: 2fr against 3fr, never under 300px and 380px.
        let list_w = (avail * 0.4).max(300.0);

        let sidebar = self.sidebar(rail, window, cx);
        let panes: AnyElement = match self.view {
            View::Stats => self.stats_pane(window, cx).into_any_element(),
            View::Sessions => div()
                .flex()
                .flex_1()
                .min_w_0()
                .h_full()
                .child(div().w(u(list_w)).flex_none().h_full().child(self.list_pane(window, cx)))
                .child(div().flex_1().min_w_0().h_full().child(self.detail_pane(window, cx)))
                .into_any_element(),
        };
        let about = self.about.then(|| self.about_dialog(cx));
        let whats_new = self.whats_new_dialog(cx);
        let notice = self.notice_toast();

        div()
            .id("logbook")
            .key_context("Logbook")
            .track_focus(&self.focus)
            .relative()
            .size_full()
            .flex()
            .bg(p.bg)
            .text_color(p.ink)
            .font_family(SANS)
            .text_size(u(13.))
            .on_key_down(cx.listener(Self::on_key))
            // WebView2's own zoom hotkeys are off in the Tauri app, so Ctrl+wheel zooms here too.
            .on_scroll(cx.listener(|this, ev: &ScrollEvent, window, cx| {
                if ev.modifiers.control {
                    // As in the browser: scrolling up is a negative delta, which zooms in.
                    let dy = f32::from(ev.delta.pixel_delta(px(20.)).y);
                    this.set_zoom(platform::next_zoom(this.zoom, dy), window, cx);
                    cx.stop_propagation();
                }
            }))
            .child(sidebar)
            .child(panes)
            .children(about)
            .children(whats_new)
            .children(notice)
    }
}

fn main() -> anyhow::Result<()> {
    // Cached sessions show at once; the first re-scan runs in the background.
    let service = Arc::new(SessionService::open()?);
    gpui_kit::application().with_assets(assets::Assets).run(move |cx| {
        cx.text_system().add_fonts(assets::fonts()).expect("embedded fonts load");
        gpui_kit::init(cx);
        let bounds = Bounds::centered(None, size(px(1180.), px(760.)), cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            // No native title bar: the pane headers are the title bar, as in the Tauri window (`decorations: false`).
            titlebar: Some(TitlebarOptions { title: Some("Logbook".into()), appears_transparent: true, traffic_light_position: None }),
            window_min_size: Some(size(px(900.), px(480.))),
            app_owns_titlebar_drag: true,
            ..Default::default()
        };
        gpui_kit::open_window(options, cx, |window, cx| {
            let view = cx.new(|cx| Logbook::new(service, window, cx));
            let focus = view.read(cx).focus.clone();
            window.focus(&focus, cx);
            view
        })
        .expect("failed to open window");
        cx.activate(true);
    });
    Ok(())
}
