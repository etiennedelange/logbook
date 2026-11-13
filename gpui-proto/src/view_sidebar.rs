//! The sidebar: time ranges, Stats, agents, repositories, and the footer's Sources, re-index and theme buttons.

use crate::app::{Logbook, View};
use crate::ui::*;
use crate::view_chrome::*;
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use logbook_gpui_proto::model::agents::{agent_info, source_rows, AGENT_ROWS};
use logbook_gpui_proto::model::format::{plural, short_repo};
use logbook_gpui_proto::model::select::{Range, RANGES};
use logbook_core::models::Agent;

/// Below this width the sidebar folds to an icon rail.
pub const RAIL_BELOW: f32 = 960.0;

fn range_icon(r: Range) -> &'static str {
    match r {
        Range::All => "sessions",
        Range::Today => "today",
        Range::Week => "week",
        Range::Month => "month",
    }
}

impl Logbook {
    pub fn sidebar(&mut self, rail: bool, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = self.pal;
        let stats = self.view == View::Stats;
        let width = if rail { 56.0 } else { 220.0 };

        let nav_item = move |id: &'static str, icon_name: &'static str, label: &'static str, hotkey: char, count: Option<usize>, on: bool| {
            div()
                .id(id)
                .flex()
                .items_center()
                .gap(u(9.))
                .w_full()
                .rounded(u(8.))
                .when(rail, |d| d.justify_center().py(u(9.)))
                .when(!rail, |d| d.px(u(10.)).py(u(7.)))
                .text_size(u(13.))
                .font_weight(MEDIUM)
                .text_color(if on { p.ink } else { p.soft })
                .bg(if on { p.side_sel } else { gpui_kit::transparent_black() })
                .cursor_pointer()
                .when(!on, |d| d.hover(move |s| s.bg(p.side_hover)))
                .tooltip_text(format!("{label} ({hotkey})"))
                .child(icon(icon_name, 15., if on { p.ink } else { p.mute }))
                .when(!rail, |d| d.child(div().flex_1().child(label)))
                .when(!rail, |d| {
                    d.children(count.map(|c| div().font_family(MONO).text_size(u(11.)).text_color(p.mute).child(c.to_string())))
                })
        };

        let mut ranges = div().flex().flex_none().flex_col().gap(u(2.)).px(u(8.)).pb(u(8.)).pt(u(4.));
        for r in RANGES.iter() {
            let (key, count) = (r.key, self.range_counts.get(&r.key).copied().unwrap_or(0));
            ranges = ranges.child(
                nav_item(r.label, range_icon(key), r.label, r.hotkey, Some(count), !stats && self.range == key)
                    .on_click(cx.listener(move |this, _, _, cx| this.set_range(key, cx))),
            );
        }
        ranges = ranges.child(
            nav_item("nav-stats", "stats", "Stats", '5', None, stats).on_click(cx.listener(move |this, _, _, cx| {
                let v = if this.view == View::Stats { View::Sessions } else { View::Stats };
                this.set_view(v, cx);
            })),
        );

        // Agents: only when there is more than one to pick between.
        let agents = if self.agent_counts.is_empty() {
            None
        } else {
            let mut list = div().flex().flex_col().gap(px(1.)).px(u(8.)).when(rail, |d| d.border_t_1().border_color(p.line).pt(u(8.)));
            for (agent, count) in self.agent_counts.clone() {
                let info = agent_info(agent);
                let on = self.agent == Some(agent);
                let hotkey = self.present.iter().position(|a| *a == agent).filter(|i| *i < 9).map(|i| format!("Alt+{}", i + 1));
                list = list.child(
                    div()
                        .id(("agent", info.cat))
                        .flex()
                        .flex_none()
                        .items_center()
                        .gap(u(8.))
                        .w_full()
                        .rounded(u(6.))
                        .when(rail, |d| d.justify_center().py(u(9.)))
                        .when(!rail, |d| d.px(u(10.)).py(u(6.)))
                        .text_size(u(13.))
                        .text_color(if on { p.ink } else { p.soft })
                        .bg(if on { p.btn } else { gpui_kit::transparent_black() })
                        .cursor_pointer()
                        .when(!on, |d| d.hover(move |s| s.bg(p.hover)))
                        .tooltip_text(match hotkey {
                            Some(h) => format!("{} ({h})", info.name),
                            None => info.name.to_string(),
                        })
                        .on_click(cx.listener(move |this, _, _, cx| {
                            let next = if this.agent == Some(agent) { None } else { Some(agent) };
                            this.set_agent(next, cx);
                        }))
                        .child(div().size(u(7.)).flex_none().rounded_full().bg(p.cat[info.cat]))
                        .when(!rail, |d| d.child(div().flex_1().min_w_0().truncate().child(info.name)))
                        .when(!rail, |d| d.child(div().font_family(MONO).text_size(u(11.)).text_color(p.mute).child(count.to_string()))),
                );
            }
            if self.present.len() > AGENT_ROWS && !rail {
                let all = self.all_agents;
                list = list.child(
                    div()
                        .id("agents-all")
                        .rounded(u(6.))
                        .px(u(10.))
                        .py(u(5.))
                        .text_size(u(12.))
                        .text_color(p.mute)
                        .cursor_pointer()
                        .hover(move |s| s.bg(p.hover).text_color(p.soft))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.all_agents = !this.all_agents;
                            this.refilter(false);
                            cx.notify();
                        }))
                        .child(if all { "Show fewer".to_string() } else { format!("Show all {}", self.present.len()) }),
                );
            }
            Some(
                div()
                    .flex()
                    .flex_none()
                    .flex_col()
                    .when(!rail, |d| d.child(div().px(u(18.)).pb(u(6.)).pt(u(14.)).text_size(u(12.)).font_weight(MEDIUM).text_color(p.mute).child("Agents")))
                    .when(rail, |d| d.pt(u(8.)))
                    .child(list),
            )
        };

        // Repositories (hidden in the rail, where there's no room for names).
        let q = self.repo_query.trim().to_lowercase();
        let shown: Vec<_> = self.repos.iter().filter(|r| r.name.to_lowercase().contains(&q)).cloned().collect();
        let repos = (!rail).then(|| {
            let mut head = div().flex().flex_none().items_center().px(u(18.)).pb(u(6.)).pt(u(14.)).child(
                div().flex_1().text_size(u(12.)).font_weight(MEDIUM).text_color(p.mute).child("Repositories"),
            );
            if self.repo.is_some() {
                head = head.child(
                    div()
                        .id("repo-clear")
                        .rounded(u(6.))
                        .px(u(6.))
                        .text_size(u(12.))
                        .line_height(u(18.))
                        .text_color(p.mute)
                        .cursor_pointer()
                        .hover(move |s| s.bg(p.side_hover).text_color(p.soft))
                        .on_click(cx.listener(|this, _, _, cx| this.set_repo(None, cx)))
                        .child("Clear"),
                );
            }
            let mut rows = div().id("repos").flex().flex_col().gap(px(1.)).px(u(8.)).pb(u(8.)).flex_1().min_h_0().overflow_y_scroll().track_scroll(&self.repos_scroll);
            for (i, r) in shown.iter().enumerate() {
                let on = self.repo.as_deref() == Some(r.name.as_str());
                let name = r.name.clone();
                rows = rows.child(
                    div()
                        .id(("repo", i))
                        .flex()
                        .flex_none()
                        .items_center()
                        .gap(u(9.))
                        .w_full()
                        .rounded(u(8.))
                        .px(u(10.))
                        .py(u(6.))
                        .text_size(u(13.))
                        .text_color(if on { p.ink } else { p.soft })
                        .bg(if on { p.side_sel } else { gpui_kit::transparent_black() })
                        .cursor_pointer()
                        .when(!on, |d| d.hover(move |s| s.bg(p.side_hover)))
                        .tooltip_text(r.name.clone())
                        .on_click(cx.listener(move |this, _, _, cx| {
                            let next = if this.repo.as_deref() == Some(name.as_str()) { None } else { Some(name.clone()) };
                            this.set_repo(next, cx);
                        }))
                        .child(icon("folder", 15., if on { p.ink } else { p.mute }))
                        .child(div().flex_1().min_w_0().truncate().child(short_repo(&r.name).to_string()))
                        .child(div().font_family(MONO).text_size(u(11.)).text_color(p.mute).child(r.count.to_string())),
                );
            }
            if !q.is_empty() && shown.is_empty() {
                rows = rows.child(div().px(u(10.)).py(u(8.)).text_size(u(12.)).text_color(p.mute).child("No repositories match."));
            }
            if !self.sessions.is_empty() && self.repos.is_empty() {
                rows = rows.child(div().px(u(10.)).py(u(8.)).text_size(u(12.)).text_color(p.mute).child("No repositories in this range."));
            }
            div()
                .flex()
                .flex_col()
                .flex_1()
                .min_h_0()
                .child(head)
                .when(self.repos.len() > 6, |d| {
                    d.child(div().flex_none().px(u(8.)).pb(u(4.)).child(self.search_box(&self.repo_input.clone(), "clear-repo-filter", &self.repo_query, _window, cx)))
                })
                .child(div().flex().flex_col().flex_1().min_h_0().relative().overflow_hidden().child(rows.vertical_scrollbar(&self.repos_scroll)))
        });

        // Footer: Sources, re-index, theme.
        let indexing = self.status.indexing;
        let theme_name = if p.dark { "sun" } else { "moon" };
        let sources_found = self.sources.as_ref().map(|s| s.iter().filter(|s| s.found).count());
        let footer = div()
            .flex()
            .flex_none()
            .items_center()
            .justify_end()
            .gap(u(4.))
            .when(rail, |d| d.flex_col().px(u(8.)).py(u(10.)))
            .when(!rail, |d| d.px(u(10.)).py(u(10.)))
            .when_some(sources_found, |d, found| {
                let rows = source_rows(self.sources.as_deref().unwrap_or(&[]), &self.sessions);
                let found_rows: Vec<_> = rows.iter().filter(|r| r.found).collect();
                let open = self.sources_open;
                d.child(self.sources_trigger(found_rows.iter().map(|r| r.agent).collect(), found, open, rail, cx))
            })
            .child(
                icon_button_el("reindex", spinner("refresh", 14., p.mute, indexing), 28., p, p.side_hover)
                    .tooltip_text("Re-index sessions (r)")
                    .on_click(cx.listener(|this, _, _, cx| this.reindex(cx))),
            )
            .child(
                icon_button("theme", theme_name, 14., 28., p, p.side_hover)
                    .tooltip_text(if p.dark { "Switch to light theme" } else { "Switch to dark theme" })
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_theme(cx))),
            );

        div()
            .relative()
            .flex()
            .flex_none()
            .flex_col()
            .w(u(width))
            .h_full()
            .min_h_0()
            .bg(p.side)
            .child(
                div()
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap(u(9.))
                    .h(u(HEADER_H))
                    .when(rail, |d| d.justify_center())
                    .when(!rail, |d| d.px(u(18.)))
                    .child(icon("logbook", 19., p.accent_text))
                    .when(!rail, |d| {
                        d.child(
                            drag_region(div().id("brand")).flex_1().h_full().flex().items_center().child(
                                div().text_size(u(15.)).font_weight(SEMIBOLD).child("logbook"),
                            ),
                        )
                    }),
            )
            .child(ranges)
            .children(agents)
            .children(repos)
            .when(rail, |d| d.child(div().flex_1()))
            .child(footer)
            .when(self.sources_open && sources_found.is_some(), |d| d.child(deferred(self.sources_popover(rail, cx)).with_priority(1)))
    }
}

impl Logbook {
    fn sources_trigger(&self, agents: Vec<Agent>, found: usize, open: bool, rail: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let p = self.pal;
        let label = if found > 0 { plural(found, "source", "sources") } else { "No sources".to_string() };
        div()
            .id("sources")
            .flex()
            .items_center()
            .gap(u(8.))
            .h(u(28.))
            .rounded(u(6.))
            .when(!rail, |d| d.mr_auto().pl(u(8.)).pr(u(10.)))
            .when(rail, |d| d.w(u(28.)).justify_center())
            .text_size(u(12.))
            .text_color(if open { p.ink } else { p.mute })
            .bg(if open { p.side_sel } else { gpui_kit::transparent_black() })
            .cursor_pointer()
            .hover(move |s| s.bg(if open { p.side_sel } else { p.side_hover }).text_color(if open { p.ink } else { p.soft }))
            .tooltip_text("Folders Logbook reads")
            .on_click(cx.listener(|this, _, _, cx| {
                let just_closed = this.sources_closed_at.is_some_and(|t| t.elapsed() < std::time::Duration::from_millis(250));
                this.sources_open = !this.sources_open && !just_closed;
                cx.notify();
            }))
            .child(if agents.is_empty() {
                div().size(u(9.)).rounded_full().border_1().border_color(p.mute).into_any_element()
            } else {
                // The agents' colours, overlapping a little, each ringed in the button's colour.
                let ring = if open { p.side_sel } else { p.side };
                div()
                    .flex()
                    .items_center()
                    .children(agents.iter().take(4).enumerate().map(|(i, a)| {
                        div().size(u(9.)).rounded_full().bg(p.cat[agent_info(*a).cat]).border_2().border_color(ring).when(i > 0, |d| d.ml(u(-3.)))
                    }))
                    .into_any_element()
            })
            .when(!rail, |d| d.child(label))
    }
}
