//! The app's colours: the same tokens as `ui/src/theme.css` (OKLCH), as GPUI colours, for both themes.

use gpui_kit::{App, Global, Hsla, Rgba};

/// A colour in OKLCH, kept so `color-mix(in oklch, ...)` can be reproduced exactly.
#[derive(Clone, Copy)]
struct Lch {
    l: f32,
    c: f32,
    h: f32,
}

const fn lch(l: f32, c: f32, h: f32) -> Lch {
    Lch { l, c, h }
}

impl Lch {
    fn rgba(self) -> Rgba {
        let (a, b) = (self.c * self.h.to_radians().cos(), self.c * self.h.to_radians().sin());
        let l_ = self.l + 0.396_337_78 * a + 0.215_803_76 * b;
        let m_ = self.l - 0.105_561_346 * a - 0.063_854_17 * b;
        let s_ = self.l - 0.089_484_18 * a - 1.291_485_5 * b;
        let (l, m, s) = (l_.powi(3), m_.powi(3), s_.powi(3));
        let r = 4.076_741_7 * l - 3.307_711_6 * m + 0.230_969_93 * s;
        let g = -1.268_438 * l + 2.609_757_4 * m - 0.341_319_4 * s;
        let bl = -0.004_196_086 * l - 0.703_418_6 * m + 1.707_614_7 * s;
        let enc = |c: f32| {
            let c = c.clamp(0.0, 1.0);
            if c <= 0.003_130_8 { 12.92 * c } else { 1.055 * c.powf(1.0 / 2.4) - 0.055 }
        };
        Rgba { r: enc(r), g: enc(g), b: enc(bl), a: 1.0 }
    }

    fn hsla(self) -> Hsla {
        self.rgba().into()
    }

    /// `color-mix(in oklch, self pct%, other)`: lightness and chroma linearly, hue by the shorter arc.
    fn mix(self, pct: f32, other: Lch) -> Lch {
        let t = pct / 100.0;
        let mut dh = other.h - self.h;
        if dh > 180.0 {
            dh -= 360.0;
        } else if dh < -180.0 {
            dh += 360.0;
        }
        Lch {
            l: self.l * t + other.l * (1.0 - t),
            c: self.c * t + other.c * (1.0 - t),
            h: (self.h + dh * (1.0 - t)).rem_euclid(360.0),
        }
    }
}

fn hex(v: u32) -> Hsla {
    gpui_kit::rgb(v).into()
}

/// Every colour the UI uses. `Copy`, so a render can take the whole set.
#[derive(Clone, Copy)]
pub struct Palette {
    pub dark: bool,
    /// Three steps of shade across the window: sidebar, then session list, then transcript.
    pub side: Hsla,
    pub side_hover: Hsla,
    pub side_sel: Hsla,
    pub list: Hsla,
    pub bg: Hsla,
    pub raised: Hsla,
    pub inset: Hsla,
    pub line: Hsla,
    pub line_soft: Hsla,
    pub ink: Hsla,
    pub soft: Hsla,
    pub mute: Hsla,
    pub hover: Hsla,
    pub sel: Hsla,
    pub btn: Hsla,
    pub btn_hover: Hsla,
    pub accent: Hsla,
    pub accent_ink: Hsla,
    pub accent_text: Hsla,
    pub ok: Hsla,
    pub fail: Hsla,
    /// One categorical colour per agent.
    pub cat: [Hsla; 4],
    /// The activity map's five shades, from empty to busiest.
    pub heat: [Hsla; 5],
    /// `color-mix(in oklch, accent 50%, raised)` and 55%: the hour chart's and repository bars.
    pub bar: Hsla,
    pub bar_repo: Hsla,
}

impl Palette {
    pub fn dark() -> Self {
        let accent = lch(0.74, 0.14, 295.0);
        let line_soft = lch(0.25, 0.012, 265.0);
        let raised = lch(0.24, 0.014, 265.0);
        Self {
            dark: true,
            side: lch(0.255, 0.013, 265.0).hsla(),
            side_hover: lch(0.29, 0.014, 265.0).hsla(),
            side_sel: lch(0.335, 0.016, 265.0).hsla(),
            list: lch(0.215, 0.012, 265.0).hsla(),
            bg: lch(0.195, 0.011, 265.0).hsla(),
            raised: raised.hsla(),
            inset: lch(0.17, 0.012, 265.0).hsla(),
            line: lch(0.29, 0.012, 265.0).hsla(),
            line_soft: line_soft.hsla(),
            ink: lch(0.96, 0.005, 265.0).hsla(),
            soft: lch(0.85, 0.01, 265.0).hsla(),
            mute: lch(0.72, 0.01, 265.0).hsla(),
            hover: lch(0.245, 0.012, 265.0).hsla(),
            sel: lch(0.285, 0.016, 265.0).hsla(),
            btn: lch(0.27, 0.014, 265.0).hsla(),
            btn_hover: lch(0.31, 0.014, 265.0).hsla(),
            accent: accent.hsla(),
            accent_ink: lch(0.18, 0.04, 295.0).hsla(),
            accent_text: lch(0.8, 0.11, 295.0).hsla(),
            ok: lch(0.76, 0.15, 155.0).hsla(),
            fail: lch(0.7, 0.17, 25.0).hsla(),
            cat: [hex(0x4eb9ad), hex(0x7bd2f3), hex(0x86a4e4), hex(0xd1b6f3)],
            heat: [
                line_soft.hsla(),
                accent.mix(32.0, line_soft).hsla(),
                accent.mix(55.0, line_soft).hsla(),
                accent.mix(78.0, line_soft).hsla(),
                accent.hsla(),
            ],
            bar: accent.mix(50.0, raised).hsla(),
            bar_repo: accent.mix(55.0, raised).hsla(),
        }
    }

    pub fn light() -> Self {
        let accent = lch(0.54, 0.19, 293.0);
        let line_soft = lch(0.935, 0.005, 265.0);
        let raised = lch(0.96, 0.005, 265.0);
        Self {
            dark: false,
            side: lch(0.935, 0.006, 265.0).hsla(),
            side_hover: lch(0.91, 0.008, 265.0).hsla(),
            side_sel: lch(0.885, 0.01, 265.0).hsla(),
            list: lch(0.975, 0.003, 265.0).hsla(),
            bg: lch(0.995, 0.001, 265.0).hsla(),
            raised: raised.hsla(),
            inset: hex(0xffffff),
            line: lch(0.9, 0.006, 265.0).hsla(),
            line_soft: line_soft.hsla(),
            ink: lch(0.22, 0.015, 265.0).hsla(),
            soft: lch(0.35, 0.012, 265.0).hsla(),
            mute: lch(0.47, 0.012, 265.0).hsla(),
            hover: lch(0.95, 0.005, 265.0).hsla(),
            sel: lch(0.925, 0.01, 265.0).hsla(),
            btn: lch(0.94, 0.005, 265.0).hsla(),
            btn_hover: lch(0.91, 0.006, 265.0).hsla(),
            accent: accent.hsla(),
            accent_ink: lch(0.99, 0.005, 293.0).hsla(),
            accent_text: lch(0.5, 0.17, 293.0).hsla(),
            ok: lch(0.55, 0.13, 155.0).hsla(),
            fail: lch(0.53, 0.18, 25.0).hsla(),
            cat: [hex(0x368a81), hex(0x28667c), hex(0x5a78bb), hex(0x6c4c8e)],
            heat: [
                line_soft.hsla(),
                accent.mix(32.0, line_soft).hsla(),
                accent.mix(55.0, line_soft).hsla(),
                accent.mix(78.0, line_soft).hsla(),
                accent.hsla(),
            ],
            bar: accent.mix(50.0, raised).hsla(),
            bar_repo: accent.mix(55.0, raised).hsla(),
        }
    }

    pub fn of(dark: bool) -> Self {
        if dark { Self::dark() } else { Self::light() }
    }
}

/// The current palette, readable from any `App` (list rows render with only that in hand).
pub struct PaletteGlobal(pub Palette);

impl Global for PaletteGlobal {}

pub fn pal(cx: &App) -> Palette {
    cx.global::<PaletteGlobal>().0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oklch_converts_to_srgb() {
        // White and black are exact; mid-grey L=0.5 is about #636363.
        let w = lch(1.0, 0.0, 0.0).rgba();
        assert!((w.r - 1.0).abs() < 1e-3 && (w.g - 1.0).abs() < 1e-3 && (w.b - 1.0).abs() < 1e-3);
        let g = lch(0.5, 0.0, 0.0).rgba();
        assert!((g.r * 255.0 - 99.0).abs() < 1.5, "{}", g.r * 255.0);
        // oklch(0.74 0.14 295) is the dark theme's violet: blue-ish, red above green.
        let a = lch(0.74, 0.14, 295.0).rgba();
        assert!(a.b > a.r && a.r > a.g);
    }

    #[test]
    fn mixing_takes_the_shorter_hue_arc() {
        let m = lch(0.5, 0.1, 350.0).mix(50.0, lch(0.5, 0.1, 10.0));
        assert!(m.h > 355.0 || m.h < 5.0, "{}", m.h);
    }
}
