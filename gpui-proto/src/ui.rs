//! Small helpers shared by every view.

use gpui_kit::*;

pub const SANS: &str = "IBM Plex Sans";
pub const MONO: &str = "JetBrains Mono";

/// A length in the web UI's CSS pixels. Everything is in rems, and the window's rem size is 16px times the
/// zoom, so Ctrl+wheel zooms the whole interface (layout reflowing) like the webview's zoom did.
pub fn u(v: f32) -> Rems {
    rems(v / 16.0)
}

/// A stroke icon from `assets/icons`, drawn in `color`.
pub fn icon(name: &'static str, size: f32, color: Hsla) -> Svg {
    svg().path(SharedString::from(format!("icons/{name}.svg"))).size(u(size)).flex_none().text_color(color)
}

pub const MEDIUM: FontWeight = FontWeight::MEDIUM;
pub const SEMIBOLD: FontWeight = FontWeight::SEMIBOLD;

/// `color` at `alpha` of its opacity (`bg-accent/12` and the like).
pub fn alpha(color: Hsla, a: f32) -> Hsla {
    Hsla { a: color.a * a, ..color }
}

/// A tooltip drawn with the app's palette (gpui-component's own reads a theme this app doesn't fill in).
struct Tip {
    text: SharedString,
    p: crate::theme::Palette,
}

impl Render for Tip {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let p = self.p;
        div()
            .max_w(u(360.))
            .rounded(u(6.))
            .border_1()
            .border_color(p.line)
            .bg(p.raised)
            .px(u(10.))
            .py(u(5.))
            .text_size(u(12.))
            .text_color(p.soft)
            .shadow_md()
            .child(self.text.clone())
    }
}

/// `.tooltip_text("...")` on any element that can show a tooltip.
pub trait TooltipExt: StatefulInteractiveElement + Sized {
    fn tooltip_text(self, text: impl Into<SharedString>) -> Self {
        let text: SharedString = text.into();
        self.tooltip(move |_window, cx| {
            let p = crate::theme::pal(cx);
            let text = text.clone();
            cx.new(|_| Tip { text, p }).into()
        })
    }
}

impl<T: StatefulInteractiveElement> TooltipExt for T {}

/// A refresh glyph that turns while `spinning`.
pub fn spinner(name: &'static str, size: f32, color: Hsla, spinning: bool) -> AnyElement {
    let el = icon(name, size, color);
    if spinning {
        el.with_animation(
            "spin",
            Animation::new(std::time::Duration::from_millis(1000)).repeat(),
            |el, delta| el.with_transformation(Transformation::rotate(percentage(delta))),
        )
        .into_any_element()
    } else {
        el.into_any_element()
    }
}
