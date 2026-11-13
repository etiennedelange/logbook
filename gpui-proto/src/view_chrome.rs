//! Pieces every pane shares: the 32px header that doubles as the title bar, the window's caption
//! buttons, plain buttons, and the search box.

use crate::app::Logbook;
use crate::theme::Palette;
use crate::ui::*;
use gpui_kit::component::input::Input;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

/// Pane top bar height. Every pane uses the same height so their bottom borders line up.
pub const HEADER_H: f32 = 32.0;

/// Makes an element move the window when dragged. On Windows the system does it from the element's hit
/// region; elsewhere GPUI starts the move itself.
pub fn drag_region(el: Stateful<Div>) -> Stateful<Div> {
    let el = el.window_control_area(WindowControlArea::Drag);
    #[cfg(not(target_os = "windows"))]
    let el = el
        .on_mouse_down(MouseButton::Left, |ev, window, _| {
            if ev.click_count == 2 {
                window.zoom_window();
            } else {
                window.start_window_move();
            }
        });
    el
}

impl Logbook {
    /// A pane's header: its title (a drag region) and whatever `end` puts after it.
    pub fn pane_header(&self, id: &'static str, title: String, pl: f32, pr: f32, end: impl IntoElement) -> Div {
        let p = self.pal;
        div()
            .flex()
            .flex_none()
            .items_center()
            .gap(u(10.))
            .h(u(HEADER_H))
            .pl(u(pl))
            .pr(u(pr))
            .border_b_1()
            .border_color(p.line)
            .child(
                drag_region(div().id(id))
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .flex()
                    .items_center()
                    .child(div().truncate().text_size(u(14.)).font_weight(SEMIBOLD).line_height(relative(1.2)).child(title)),
            )
            .child(end)
    }

    /// Minimise, maximise and close. The window has no native title bar; the pane headers are the title
    /// bar, so these sit inline at the end of the last one, as Windows 11's caption buttons do.
    pub fn window_controls(&self, window: &Window) -> impl IntoElement {
        let p = self.pal;
        let maximized = window.is_maximized();
        let button = move |id: &'static str, glyph: &'static str, area: WindowControlArea, close: bool| {
            let el = div()
                .id(id)
                .w(u(46.))
                .h_full()
                .flex()
                .flex_none()
                .items_center()
                .justify_center()
                .text_color(p.soft)
                .hover(move |s| if close { s.bg(rgb(0xc42b1c)).text_color(white()) } else { s.bg(p.btn).text_color(p.ink) })
                .window_control_area(area)
                .child(svg().path(SharedString::from(format!("icons/{glyph}.svg"))).size(u(10.)).text_color(p.soft));
            // Off Windows nothing handles the hit region, so the buttons act on click.
            #[cfg(not(target_os = "windows"))]
            let el = el.on_click(move |_, window, _| match area {
                WindowControlArea::Min => window.minimize_window(),
                WindowControlArea::Max => window.zoom_window(),
                _ => window.remove_window(),
            });
            el
        };
        div()
            .flex()
            .flex_none()
            .self_stretch()
            .ml(u(4.))
            .mr(u(-12.))
            .child(button("win-min", "win-min", WindowControlArea::Min, false))
            .child(button("win-max", if maximized { "win-restore" } else { "win-max" }, WindowControlArea::Max, false))
            .child(button("win-close", "win-close", WindowControlArea::Close, true))
    }
}

/// A small square icon button (the About `?`, copy, folder, the sidebar footer's).
pub fn icon_button(id: impl Into<ElementId>, name: &'static str, size: f32, box_px: f32, p: Palette, hover_bg: Hsla) -> Stateful<Div> {
    icon_button_el(id, icon(name, size, p.mute).into_any_element(), box_px, p, hover_bg)
}

pub fn icon_button_el(id: impl Into<ElementId>, glyph: AnyElement, box_px: f32, p: Palette, hover_bg: Hsla) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .size(u(box_px))
        .rounded(u(6.))
        .text_color(p.mute)
        .cursor_pointer()
        .hover(move |s| s.bg(hover_bg).text_color(p.soft))
        .child(glyph)
}

/// The button every dialog and empty state uses: `bg-btn`, soft text.
pub fn plain_button(id: impl Into<ElementId>, label: &str, p: Palette) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .items_center()
        .gap(u(6.))
        .rounded(u(6.))
        .bg(p.btn)
        .px(u(11.))
        .py(u(5.))
        .text_size(u(12.))
        .text_color(p.soft)
        .cursor_pointer()
        .hover(move |s| s.bg(p.btn_hover).text_color(p.ink))
        .child(label.to_string())
}

pub fn accent_button(id: impl Into<ElementId>, label: &str, p: Palette) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .items_center()
        .gap(u(6.))
        .rounded(u(6.))
        .bg(p.accent)
        .px(u(11.))
        .py(u(5.))
        .text_size(u(12.))
        .font_weight(SEMIBOLD)
        .text_color(p.accent_ink)
        .cursor_pointer()
        .hover(|s| s.opacity(0.92))
        .child(label.to_string())
}

impl Logbook {
    /// The filter box: Esc or × clears it. `searchable` is the one `/` focuses from anywhere.
    pub fn search_box(&self, input: &Entity<gpui_kit::component::input::InputState>, id: &'static str, value: &str, window: &Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = self.pal;
        let focused = input.read(cx).focus_handle(cx).is_focused(window);
        let input_clone = input.clone();
        let input_focus = input.clone();
        div()
            .relative()
            .w_full()
            .min_w_0()
            .child(
                div()
                    .rounded(u(8.))
                    .border_1()
                    .border_color(if focused { p.accent } else { p.line })
                    .bg(p.bg)
                    .py(u(1.))
                    .pl(u(4.))
                    .pr(u(26.))
                    .text_size(u(12.5))
                    .text_color(p.ink)
                    .cursor_text()
                    // A click anywhere in the box focuses it, not just on the text.
                    .on_mouse_down(MouseButton::Left, cx.listener(move |_, _, window, cx| {
                        input_focus.update(cx, |i, cx| i.focus(window, cx));
                    }))
                    .child(Input::new(input).appearance(false).cleanable(false)),
            )
            .when(!value.is_empty(), move |this| {
                this.child(
                    div()
                        .id(id)
                        .absolute()
                        .right(u(3.))
                        .top(u(7.))
                        .size(u(20.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(u(4.))
                        .text_color(p.mute)
                        .text_size(u(14.))
                        .cursor_pointer()
                        .hover(move |s| s.bg(p.btn).text_color(p.soft))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            input_clone.update(cx, |i, cx| i.set_value("", window, cx));
                            if id == "clear-search" {
                                this.set_query(String::new(), cx);
                            } else {
                                this.repo_query.clear();
                                cx.notify();
                            }
                        }))
                        .child("×"),
                )
            })
    }
}
