//! The Stats view: totals, the 26-week activity map, when you start sessions, agents, repositories, highlights.

use crate::app::{Logbook, View};
use crate::theme::Palette;
use crate::ui::*;
use chrono::Datelike;
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use logbook_gpui_proto::model::agents::agent_info;
use logbook_gpui_proto::model::format::{plural, short_repo};
use logbook_gpui_proto::model::stats::{facts, greeting, hh, level, thousands, Day, Period, Stats, MAP_WEEKS, PERIODS};
use chrono::Timelike;

const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
const WEEKDAYS: [&str; 7] = ["Mondays", "Tuesdays", "Wednesdays", "Thursdays", "Fridays", "Saturdays", "Sundays"];

fn day_label(d: chrono::NaiveDate) -> String {
    d.format("%a, %b %-d").to_string()
}

impl Logbook {
    fn card(&self, title: &str, aside: Option<String>, body: impl IntoElement) -> Div {
        let p = self.pal;
        div()
            .flex()
            .flex_col()
            .gap(u(12.))
            .min_w_0()
            .rounded(u(12.))
            .border_1()
            .border_color(p.line)
            .bg(p.list)
            .px(u(20.))
            .py(u(16.))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_baseline()
                    .gap_x(u(16.))
                    .gap_y(u(4.))
                    .child(div().flex_1().text_size(u(13.)).font_weight(SEMIBOLD).child(title.to_string()))
                    .children(aside.map(|a| div().text_size(u(12.)).text_color(p.mute).child(a))),
            )
            .child(body)
    }

    pub fn stats_pane(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = self.pal;
        let wide = window.viewport_size().width / self.zoom.max(0.1) >= px(1400.);
        let two_cols = window.viewport_size().width / self.zoom.max(0.1) < px(1100.);
        let end = self.header_end(None, wide, window, cx);
        let stats = self.stats();
        let stats = &*stats;
        let days = PERIODS.iter().find(|x| x.key == self.stats_period).unwrap();
        let sub = format!(
            "{} · {}{}, {}",
            greeting(self.now),
            plural(stats.sessions, "session", "sessions"),
            if stats.repo_count > 0 { format!(" across {}", plural(stats.repo_count, "repository", "repositories")) } else { String::new() },
            days.label.to_lowercase()
        );
        let found = self.sources.as_ref().map(|s| s.iter().filter(|x| x.found).count()).unwrap_or(0);

        let period_group = div()
            .flex()
            .flex_none()
            .gap(px(1.))
            .rounded(u(8.))
            .border_1()
            .border_color(p.line)
            .bg(p.inset)
            .p(u(2.))
            .children(PERIODS.iter().map(|pi| {
                let on = self.stats_period == pi.key;
                let key = pi.key;
                div()
                    .id(pi.label)
                    .rounded(u(6.))
                    .px(u(10.))
                    .py(u(4.))
                    .text_size(u(12.))
                    .font_weight(MEDIUM)
                    .text_color(if on { p.ink } else { p.mute })
                    .bg(if on { p.sel } else { gpui_kit::transparent_black() })
                    .cursor_pointer()
                    .when(!on, |d| d.hover(move |s| s.text_color(p.soft)))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.stats_period = key;
                        cx.notify();
                    }))
                    .child(pi.label)
            }));
        let live = (found > 0).then(|| {
            div()
                .flex()
                .items_center()
                .gap(u(8.))
                .whitespace_nowrap()
                .text_size(u(12.))
                .text_color(p.mute)
                .child(div().size(u(7.)).rounded_full().bg(p.ok))
                .child(if self.status.indexing { "Reading sessions…".to_string() } else { format!("Watching {}", plural(found, "source", "sources")) })
        });

        let body: AnyElement = if self.loaded && self.sessions.is_empty() {
            div().py(u(40.)).text_center().text_size(u(13.)).text_color(p.mute).child("No sessions yet. Once Copilot CLI or Claude Code has run, your stats show up here.").into_any_element()
        } else {
            div()
                .flex()
                .flex_col()
                .gap(u(12.))
                .child(self.fact_strip(stats, cx))
                .child(self.totals(stats, days.days, two_cols))
                .child(self.activity_map(stats, days.days))
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .gap(u(12.))
                        .child(self.hours(stats).flex_1().min_w(u(300.)))
                        .child(self.agents_card(stats, cx).flex_1().min_w(u(300.))),
                )
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .gap(u(12.))
                        .child(self.repos_card(stats, cx).flex_1().min_w(u(300.)))
                        .child(self.highlights(stats, cx).flex_1().min_w(u(300.))),
                )
                .into_any_element()
        };

        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .h_full()
            .bg(p.bg)
            .child(self.pane_header("stats-title", "Stats".into(), 24., 12., end))
            .child(
                div().relative().flex_1().min_h_0().child(
                    div()
                        .id("stats-scroll")
                        .size_full()
                        .overflow_y_scroll()
                        .track_scroll(&self.stats_scroll)
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(u(12.))
                                .px(u(24.))
                                .pb(u(24.))
                                .pt(u(16.))
                                .child(div().text_size(u(13.)).text_color(p.mute).child(sub))
                                .when(!self.sessions.is_empty(), |d| {
                                    d.child(div().flex().flex_wrap().items_center().gap_x(u(16.)).gap_y(u(8.)).child(period_group).child(div().ml_auto().children(live)))
                                })
                                .child(body),
                        )
                        .vertical_scrollbar(&self.stats_scroll),
                ),
            )
    }

    /// One line at a time, worked out from the stats. Click for the next; it moves on by itself unless you're on it.
    fn fact_strip(&self, stats: &Stats, cx: &mut Context<Self>) -> AnyElement {
        let p = self.pal;
        let list = facts(stats, |a| agent_info(a).name.to_string());
        if list.is_empty() {
            return div().into_any_element();
        }
        let at = self.fact_i % list.len();
        let n = list.len();
        div()
            .id("fact-strip")
            .flex()
            .flex_none()
            .items_center()
            .gap(u(12.))
            .min_h(u(40.))
            .rounded(u(8.))
            .border_1()
            .border_color(p.line_soft)
            .bg(p.list)
            .px(u(16.))
            .py(u(8.))
            .text_size(u(13.))
            .text_color(p.soft)
            .cursor_pointer()
            .hover(move |s| s.bg(p.hover))
            .tooltip_text("Show another")
            .on_hover(cx.listener(|this, hovered: &bool, _, cx| {
                this.fact_held = *hovered;
                cx.notify();
            }))
            .on_click(cx.listener(|this, _, _, cx| {
                this.fact_i += 1;
                cx.notify();
            }))
            .child(icon("sparkle", 15., p.accent_text))
            .child(div().flex_1().min_w_0().child(list[at].clone()))
            .child(div().flex_none().font_family(MONO).text_size(u(11.)).text_color(p.mute).child(format!("{}/{}", at + 1, n)))
            .into_any_element()
    }

    fn totals(&self, stats: &Stats, days: i64, two_cols: bool) -> AnyElement {
        let p = self.pal;
        let tile = move |label: &'static str, value: String, sub: String, flame: bool| {
            div()
                .flex()
                .flex_col()
                .gap(u(2.))
                .rounded(u(12.))
                .border_1()
                .border_color(p.line)
                .bg(p.list)
                .px(u(16.))
                .py(u(12.))
                .child(div().text_size(u(12.)).text_color(p.mute).child(label))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(u(8.))
                        .text_size(u(22.))
                        .font_weight(SEMIBOLD)
                        .child(div().flex_1().child(value))
                        .when(flame, |d| d.child(icon("flame", 22., p.accent_text))),
                )
                .child(div().text_size(u(11.)).text_color(p.mute).child(sub))
        };
        div()
            .grid()
            .grid_cols(if two_cols { 2 } else { 4 })
            .gap(u(12.))
            .child(tile("Sessions", stats.sessions.to_string(), format!("{:.1} a day on average", stats.sessions as f64 / days as f64), false))
            .child(tile(
                "Messages",
                thousands(stats.messages),
                if stats.sessions > 0 { format!("{} per session", (stats.messages as f64 / stats.sessions as f64).round()) } else { "None yet".into() },
                false,
            ))
            .child(tile("Active days", stats.active_days.to_string(), format!("of {days}"), false))
            .child(tile("Current streak", plural(stats.streak, "day", "days"), format!("Longest in period: {}", plural(stats.longest_streak, "day", "days")), true))
            .into_any_element()
    }

    fn activity_map(&self, stats: &Stats, days: i64) -> AnyElement {
        let p = self.pal;
        let busiest_wd = stats.weekdays.iter().enumerate().max_by_key(|(i, c)| (**c, std::cmp::Reverse(*i))).map_or(0, |(i, _)| i);
        let months: Vec<&str> = stats
            .weeks
            .iter()
            .enumerate()
            .map(|(i, w)| {
                if let Some(first) = w.iter().find(|d| d.date.day() == 1) {
                    return MONTHS[first.date.month0() as usize];
                }
                // The first column names its month too, unless a new month starts within two weeks.
                if i == 0 && !stats.weeks[1..3].iter().any(|x| x.iter().any(|d| d.date.day() == 1)) {
                    return MONTHS[w[0].date.month0() as usize];
                }
                ""
            })
            .collect();
        let active = stats.weeks.iter().flatten().filter(|d| d.count > 0).count();
        let _ = active;
        let square = move |d: &Day, in_period: bool| -> AnyElement {
            if d.ago < 0 {
                return div().size(u(13.)).into_any_element();
            }
            div()
                .id(SharedString::from(format!("d{}", d.ago)))
                .size(u(13.))
                .rounded(u(3.))
                .bg(p.heat[level(d.count)])
                .when(!in_period, |s| s.opacity(0.35))
                .tooltip_text(format!("{}: {}", day_label(d.date), plural(d.count, "session", "sessions")))
                .into_any_element()
        };
        let weekday_col = div().flex().flex_col().gap(u(3.)).mr(u(4.)).pt(u(17.)).font_family(MONO).text_size(u(11.)).text_color(p.mute).children(
            ["Mon", "", "Wed", "", "Fri", "", "Sun"].map(|d| div().h(u(13.)).line_height(u(13.)).child(d)),
        );
        let grid = div().flex().gap(u(3.)).child(weekday_col).children(stats.weeks.iter().enumerate().map(|(wi, w)| {
            div()
                .flex()
                .flex_col()
                .gap(u(3.))
                .child(div().h(u(14.)).w(u(13.)).whitespace_nowrap().text_size(u(11.)).text_color(p.mute).child(months[wi]))
                .children(w.iter().map(|d| square(d, d.ago < days)))
        }));
        let fact = |label: &'static str, value: String| {
            div().child(div().text_size(u(11.)).text_color(p.mute).child(label)).child(div().text_size(u(14.)).font_weight(SEMIBOLD).child(value))
        };
        let legend = div()
            .mt(u(12.))
            .flex()
            .items_center()
            .gap(u(3.))
            .text_size(u(11.))
            .text_color(p.mute)
            .child(div().mr(u(4.)).child("Less"))
            .children((0..5).map(|l| div().size(u(11.)).rounded(u(3.)).bg(p.heat[l])))
            .child(div().ml(u(4.)).child("More"));
        let _ = MAP_WEEKS;
        self.card(
            "Activity",
            Some("Each square is a day; the highlighted ones are in the period".into()),
            div()
                .flex()
                .flex_wrap()
                .items_start()
                .gap_x(u(32.))
                .gap_y(u(16.))
                .child(div().max_w_full().child(grid).child(legend))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(u(12.))
                        .min_w(u(160.))
                        .flex_1()
                        .child(fact("Busiest day", stats.busiest_day.map_or("–".to_string(), |d| format!("{} · {}", day_label(d.date), d.count))))
                        .child(fact("Favourite day", if stats.sessions > 0 { WEEKDAYS[busiest_wd].to_string() } else { "–".into() }))
                        .child(fact("Days off", plural((days - stats.active_days as i64).max(0) as usize, "day", "days"))),
                ),
        )
        .into_any_element()
    }

    fn hours(&self, stats: &Stats) -> Div {
        let p = self.pal;
        let max = stats.hours.iter().copied().max().unwrap_or(0).max(1);
        let h = self.now.hour() as usize;
        let aside = format!("{}now {}", stats.peak_hour.map(|k| format!("Peak around {} · ", hh(k))).unwrap_or_default(), hh(h));
        self.card(
            "When you start sessions",
            Some(aside),
            div()
                .child(
                    div().flex().h(u(110.)).items_end().gap(u(3.)).border_b_1().border_color(p.line).children(stats.hours.iter().enumerate().map(|(i, c)| {
                        div()
                            .id(("hour", i))
                            .flex_1()
                            .h(u((104.0 * *c as f32 / max as f32).round().max(3.0)))
                            .rounded_t(u(3.))
                            .bg(if Some(i) == stats.peak_hour { p.accent } else { p.bar })
                            .when(i == h, |d| d.border_1().border_color(alpha(p.ink, 0.45)))
                            .tooltip_text(format!("{}: {}", hh(i), plural(*c, "session", "sessions")))
                    })),
                )
                .child(div().mt(u(5.)).flex().gap(u(3.)).font_family(MONO).text_size(u(11.)).text_color(p.mute).children((0..24).map(|i| {
                    div()
                        .flex_1()
                        .when(i == h, |d| d.text_color(p.accent_text))
                        .child(if i == h {
                            "now".to_string()
                        } else if i % 6 == 0 && (i as i64 - h as i64).abs() > 1 {
                            format!("{i:02}")
                        } else {
                            String::new()
                        })
                }))),
        )
    }

    fn agents_card(&self, stats: &Stats, cx: &mut Context<Self>) -> Div {
        let p = self.pal;
        let total: usize = stats.agents.iter().map(|a| a.1).sum();
        let body: AnyElement = if total == 0 {
            div().text_size(u(12.)).text_color(p.mute).child("No sessions in this period.").into_any_element()
        } else {
            div()
                .flex()
                .flex_col()
                .gap(u(12.))
                .child(div().flex().h(u(12.)).gap(u(2.)).overflow_hidden().rounded_full().children(stats.agents.iter().filter(|a| a.1 > 0).map(|(a, c)| {
                    div().h_full().w(relative(*c as f32 / total as f32)).bg(p.cat[agent_info(*a).cat])
                })))
                .child(div().flex().flex_col().gap(px(1.)).children(stats.agents.iter().map(|(a, c)| {
                    let a = *a;
                    let info = agent_info(a);
                    div()
                        .id(("agent-row", info.cat))
                        .flex()
                        .items_center()
                        .gap(u(10.))
                        .w_full()
                        .rounded(u(6.))
                        .px(u(8.))
                        .py(u(6.))
                        .text_size(u(13.))
                        .cursor_pointer()
                        .hover(move |s| s.bg(p.hover))
                        .tooltip_text(format!("Show {} sessions", info.name))
                        .on_click(cx.listener(move |this, _, _, cx| this.set_agent(Some(a), cx)))
                        .child(div().size(u(9.)).flex_none().rounded_full().bg(p.cat[info.cat]))
                        .child(div().flex_1().child(info.name))
                        .child(div().font_family(MONO).text_size(u(12.)).text_color(p.soft).child(c.to_string()))
                        .child(div().w(u(40.)).text_right().font_family(MONO).text_size(u(12.)).text_color(p.mute).child(format!("{}%", (c * 100 + total / 2) / total.max(1))))
                })))
                .into_any_element()
        };
        self.card("Agents", None, body)
    }

    fn repos_card(&self, stats: &Stats, cx: &mut Context<Self>) -> Div {
        let p = self.pal;
        let max = stats.repos.first().map_or(1, |r| r.1).max(1);
        let body: AnyElement = if stats.repos.is_empty() {
            div().text_size(u(12.)).text_color(p.mute).child("No sessions in this period.").into_any_element()
        } else {
            div()
                .flex()
                .flex_col()
                .gap(px(1.))
                .children(stats.repos.iter().enumerate().map(|(i, (name, count))| {
                    let n = name.clone();
                    div()
                        .id(("repo-row", i))
                        .flex()
                        .items_center()
                        .gap(u(12.))
                        .w_full()
                        .rounded(u(6.))
                        .px(u(8.))
                        .py(u(6.))
                        .text_size(u(13.))
                        .cursor_pointer()
                        .hover(move |s| s.bg(p.hover))
                        .tooltip_text(format!("Show sessions in {name}"))
                        .on_click(cx.listener(move |this, _, _, cx| this.set_repo(Some(n.clone()), cx)))
                        .child(div().w(u(110.)).flex_none().truncate().text_color(p.soft).child(short_repo(name).to_string()))
                        .child(
                            div().flex_1().h(u(8.)).rounded_full().bg(p.raised).child(
                                div().h_full().w(relative(*count as f32 / max as f32)).rounded_full().bg(if i == 0 { p.accent } else { p.bar_repo }),
                            ),
                        )
                        .child(div().w(u(36.)).flex_none().text_right().font_family(MONO).text_size(u(12.)).text_color(p.mute).child(count.to_string()))
                }))
                .into_any_element()
        };
        self.card("Top repositories", None, body)
    }

    fn highlights(&self, stats: &Stats, cx: &mut Context<Self>) -> Div {
        let p = self.pal;
        let late = (stats.late_night * 100 + stats.sessions / 2) / stats.sessions.max(1);
        struct Row {
            icon: &'static str,
            label: &'static str,
            value: String,
            detail: String,
            open: Option<String>,
        }
        let mut rows = vec![Row {
            icon: "flame",
            label: "Longest streak",
            value: format!("{} in a row", plural(stats.longest_streak, "day", "days")),
            detail: format!("Current run: {}", plural(stats.streak, "day", "days")),
            open: None,
        }];
        if let Some(c) = &stats.chattiest {
            rows.push(Row {
                icon: "chat",
                label: "Chattiest session",
                value: c.title.text.clone(),
                detail: [Some(plural(c.message_count as usize, "message", "messages")), c.repository.as_deref().map(|r| short_repo(r).to_string()), c.branch.clone()]
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>()
                    .join(" · "),
                open: Some(c.id.clone()),
            });
        }
        rows.push(Row { icon: "moon", label: "Night owl", value: format!("{late}% after 22:00"), detail: format!("{} started late", plural(stats.late_night, "session", "sessions")), open: None });
        let list = div().flex().flex_col().children(rows.into_iter().enumerate().map(|(i, r)| {
            let body = div()
                .flex()
                .items_start()
                .gap(u(12.))
                .w_full()
                .rounded(u(6.))
                .px(u(8.))
                .py(u(8.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .size(u(32.))
                        .flex_none()
                        .rounded(u(8.))
                        .bg(alpha(p.accent, 0.16))
                        .child(icon(r.icon, 15., p.accent_text)),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .flex_1()
                        .min_w_0()
                        .child(div().text_size(u(11.)).text_color(p.mute).child(r.label))
                        .child(div().truncate().text_size(u(13.)).font_weight(SEMIBOLD).child(r.value))
                        .child(div().truncate().text_size(u(12.)).text_color(p.mute).child(r.detail)),
                );
            let wrap = div().when(i > 0, |d| d.border_t_1().border_color(p.line_soft));
            match r.open {
                Some(id) => wrap.child(
                    body.id(("hl", i))
                        .cursor_pointer()
                        .hover(move |s| s.bg(p.hover))
                        .tooltip_text("Open this session")
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.view = View::Sessions;
                            this.select(&id, cx);
                        })),
                ),
                None => wrap.child(body),
            }
        }));
        self.card("Highlights", None, list)
    }
}

#[allow(unused)]
fn _unused(_: Period, _: Palette) {}
