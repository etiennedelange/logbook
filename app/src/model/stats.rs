//! Everything the Stats view shows. A port of `stats.ts`.

use super::agents::present_agents;
use super::format::{plural, short_repo};
use super::Session;
use chrono::{DateTime, Datelike, Local, NaiveDate, Timelike};
use logbook_core::models::Agent;
use std::collections::HashMap;
use std::sync::Arc;

/// The Stats view's periods: the last 7 or 30 days, as in the sidebar, or the 26 weeks the activity map shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Period {
    Week,
    Month,
    Half,
}

pub struct PeriodInfo {
    pub key: Period,
    pub label: &'static str,
    pub days: i64,
}

pub const PERIODS: [PeriodInfo; 3] = [
    PeriodInfo { key: Period::Week, label: "Last 7 days", days: 7 },
    PeriodInfo { key: Period::Month, label: "Last 30 days", days: 30 },
    PeriodInfo { key: Period::Half, label: "26 weeks", days: 26 * 7 },
];

/// Weeks the activity map shows, Monday to Sunday, ending with this week.
pub const MAP_WEEKS: usize = 26;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Day {
    pub date: NaiveDate,
    /// Days before today: 0 is today, negative is later this week.
    pub ago: i64,
    pub count: usize,
}

pub struct Stats {
    /// The sessions in the period.
    pub sessions: usize,
    pub messages: i64,
    /// Days in the period with at least one session.
    pub active_days: usize,
    /// Days in a row with a session, up to today; a day without one yet today doesn't break it.
    pub streak: usize,
    /// The longest run of such days inside the period.
    pub longest_streak: usize,
    /// `MAP_WEEKS` weeks of 7 days, Monday first, oldest week first.
    pub weeks: Vec<Vec<Day>>,
    /// Sessions by the hour they started, 0 to 23.
    pub hours: [usize; 24],
    pub peak_hour: Option<usize>,
    /// Sessions started from 22:00 to 03:59.
    pub late_night: usize,
    pub weekend: usize,
    /// Sessions by weekday, Monday first.
    pub weekdays: [usize; 7],
    pub busiest_day: Option<Day>,
    pub agents: Vec<(Agent, usize)>,
    /// Repositories with a session in the period.
    pub repo_count: usize,
    /// The busiest repositories, at most five.
    pub repos: Vec<(String, usize)>,
    /// The session with the most messages.
    pub chattiest: Option<Arc<Session>>,
}

/// Monday 0 to Sunday 6.
pub fn weekday(d: NaiveDate) -> usize {
    d.weekday().num_days_from_monday() as usize
}

fn minus_days(d: NaiveDate, n: i64) -> NaiveDate {
    d - chrono::Duration::days(n)
}

/// Everything the Stats view shows for the period ending today. Sessions count on the day they were
/// last active, as the sidebar's ranges count them, so "Last 7 days" here matches its badge.
pub fn compute_stats(all: &[Arc<Session>], period: Period, now: DateTime<Local>) -> Stats {
    let today = now.date_naive();
    let days = PERIODS.iter().find(|p| p.key == period).unwrap().days;
    let map_start = minus_days(today, weekday(today) as i64 + (MAP_WEEKS as i64 - 1) * 7);

    let mut per_day: HashMap<i64, usize> = HashMap::new();
    let mut in_period: Vec<(&Arc<Session>, DateTime<Local>)> = Vec::new();
    for s in all {
        let Some(at) = s.updated.or(s.created) else { continue };
        let ago = (today - at.date_naive()).num_days();
        if ago < 0 {
            continue; // a clock that ran ahead
        }
        *per_day.entry(ago).or_default() += 1;
        if ago < days {
            in_period.push((s, at));
        }
    }

    let weeks: Vec<Vec<Day>> = (0..MAP_WEEKS as i64)
        .map(|w| {
            (0..7)
                .map(|d| {
                    let date = map_start + chrono::Duration::days(w * 7 + d);
                    let ago = (today - date).num_days();
                    Day { date, ago, count: if ago < 0 { 0 } else { per_day.get(&ago).copied().unwrap_or(0) } }
                })
                .collect()
        })
        .collect();

    let mut active_days = 0;
    let mut longest_streak = 0;
    let mut run = 0;
    let mut busiest_day: Option<Day> = None;
    for ago in (0..days).rev() {
        let count = per_day.get(&ago).copied().unwrap_or(0);
        if count > 0 {
            active_days += 1;
        }
        run = if count > 0 { run + 1 } else { 0 };
        longest_streak = longest_streak.max(run);
        if count > 0 && busiest_day.is_none_or(|b| count > b.count) {
            busiest_day = Some(Day { date: minus_days(today, ago), ago, count });
        }
    }
    let mut streak = 0;
    let mut ago = if per_day.contains_key(&0) { 0 } else { 1 };
    while per_day.contains_key(&ago) {
        streak += 1;
        ago += 1;
    }

    let mut hours = [0usize; 24];
    let mut weekdays = [0usize; 7];
    let mut messages = 0i64;
    let mut chattiest: Option<&Arc<Session>> = None;
    let mut agent_counts: HashMap<Agent, usize> = HashMap::new();
    let mut repo_counts: HashMap<&str, usize> = HashMap::new();
    for (s, at) in &in_period {
        messages += s.message_count;
        if chattiest.is_none_or(|c| s.message_count > c.message_count) {
            chattiest = Some(s);
        }
        if let Some(start) = s.created.or(s.updated) {
            hours[start.hour() as usize] += 1;
        }
        weekdays[weekday(at.date_naive())] += 1;
        *agent_counts.entry(s.agent).or_default() += 1;
        if let Some(r) = s.repo.as_deref() {
            *repo_counts.entry(r).or_default() += 1;
        }
    }
    let peak = hours.iter().copied().max().unwrap_or(0);

    let mut repos: Vec<(String, usize)> = repo_counts.iter().map(|(n, c)| (n.to_string(), *c)).collect();
    repos.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.to_lowercase().cmp(&b.0.to_lowercase())));
    let repo_count = repos.len();
    repos.truncate(5);

    Stats {
        sessions: in_period.len(),
        messages,
        active_days,
        streak,
        longest_streak,
        weeks,
        hours,
        peak_hour: (peak > 0).then(|| hours.iter().position(|&h| h == peak).unwrap()),
        late_night: [22, 23, 0, 1, 2, 3].iter().map(|&h| hours[h]).sum(),
        weekend: weekdays[5] + weekdays[6],
        weekdays,
        busiest_day,
        // Every agent that has sessions at all keeps its row, in the sidebar's order.
        agents: present_agents(all).into_iter().map(|a| (a, agent_counts.get(&a).copied().unwrap_or(0))).collect(),
        repo_count,
        repos,
        chattiest: chattiest.cloned(),
    }
}

/// The activity map's shade for a day's count: 0 for none, then 1 to 4.
pub fn level(count: usize) -> usize {
    match count {
        0 => 0,
        1 => 1,
        2..=3 => 2,
        4..=5 => 3,
        _ => 4,
    }
}

pub fn hh(h: usize) -> String {
    format!("{h:02}:00")
}

fn pct(n: usize, of: usize) -> usize {
    (n * 100 + of.max(1) / 2) / of.max(1)
}

/// 1234567 → "1,234,567".
pub fn thousands(n: i64) -> String {
    let digits = n.abs().to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    if n < 0 { format!("-{out}") } else { out }
}

/// The fun-fact strip: short lines worked out from the stats, only the ones the data supports.
pub fn facts(s: &Stats, agent_name: impl Fn(Agent) -> String) -> Vec<String> {
    if s.sessions == 0 {
        return Vec::new();
    }
    let mut out = Vec::new();
    out.push(format!(
        "You and your agents traded {} {} in this period.",
        thousands(s.messages),
        if s.messages == 1 { "message" } else { "messages" }
    ));
    if let Some(peak) = s.peak_hour {
        let afternoon = (12..18).contains(&peak);
        out.push(format!(
            "Your sessions peak around {}.{}",
            hh(peak),
            if afternoon { " Afternoon slump? Not here." } else { "" }
        ));
    }
    match (s.repos.first(), s.repos.get(1), s.repos.get(2)) {
        (Some(r0), Some(r1), Some(r2)) if r0.1 > r1.1 + r2.1 => out.push(format!(
            "{} got more sessions than {} and {} combined.",
            short_repo(&r0.0),
            short_repo(&r1.0),
            short_repo(&r2.0)
        )),
        (Some(r0), _, _) => out.push(format!(
            "{} is your busiest repository, with {}.",
            short_repo(&r0.0),
            plural(r0.1, "session", "sessions")
        )),
        _ => {}
    }
    if s.sessions >= 5 {
        let w = pct(s.weekend, s.sessions);
        out.push(if w <= 15 {
            format!("Only {w}% of sessions were at the weekend. Healthy.")
        } else {
            format!("{w}% of sessions were at the weekend.")
        });
    }
    if s.streak > 1 {
        out.push(format!("You’re on a {}-day streak. One session tomorrow keeps it going.", s.streak));
    }
    let used: Vec<_> = s.agents.iter().filter(|a| a.1 > 0).collect();
    if used.len() > 1 {
        let top = used.iter().max_by_key(|a| a.1).unwrap();
        out.push(format!("{} ran {}% of sessions in this period.", agent_name(top.0), pct(top.1, s.sessions)));
    }
    if let Some(c) = &s.chattiest {
        if c.message_count > 1 {
            out.push(format!("Chattiest session: “{}”, with {} messages.", c.title.text, c.message_count));
        }
    }
    out
}

/// The greeting at the top of the Stats view.
pub fn greeting(now: DateTime<Local>) -> &'static str {
    match now.hour() {
        0..=4 => "Up late",
        5..=11 => "Good morning",
        12..=17 => "Good afternoon",
        _ => "Good evening",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::test_support::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    // Tuesday, 3 Nov 2026, mid-afternoon.
    fn now() -> DateTime<Local> {
        local(2026, 11, 3, 15, 30)
    }
    fn at(days: u32, hour: u32) -> String {
        iso(local(2026, 11, 3, hour, 0) - chrono::Duration::days(days as i64))
    }
    fn s(days: u32, f: impl FnOnce(&mut logbook_core::dto::SessionDto)) -> Arc<Session> {
        static N: AtomicUsize = AtomicUsize::new(0);
        let id = format!("s{}", N.fetch_add(1, Ordering::SeqCst));
        session(&id, |d| {
            d.updated_at = Some(at(days, 10));
            d.created_at = Some(at(days, 10));
            f(d);
        })
    }
    fn plain(days: u32) -> Arc<Session> {
        s(days, |_| {})
    }

    #[test]
    fn counts_the_period_by_last_activity_as_the_sidebar_does() {
        let all = [plain(0), plain(0), plain(3), plain(6), plain(7), plain(29), plain(40)];
        assert_eq!(compute_stats(&all, Period::Week, now()).sessions, 4);
        assert_eq!(compute_stats(&all, Period::Month, now()).sessions, 6);
        assert_eq!(compute_stats(&all, Period::Half, now()).sessions, 7);
    }

    #[test]
    fn adds_up_messages_and_active_days() {
        let all = [s(0, |d| d.message_count = 10), s(0, |d| d.message_count = 4), s(2, |d| d.message_count = 6)];
        let st = compute_stats(&all, Period::Week, now());
        assert_eq!((st.messages, st.active_days), (20, 2));
        assert_eq!(st.chattiest.unwrap().message_count, 10);
    }

    #[test]
    fn the_streak_survives_a_day_without_a_session_yet() {
        let run = || vec![plain(1), plain(2), plain(3), plain(5)];
        assert_eq!(compute_stats(&run(), Period::Month, now()).streak, 3);
        let with_today: Vec<_> = std::iter::once(plain(0)).chain(run()).collect();
        assert_eq!(compute_stats(&with_today, Period::Month, now()).streak, 4);
        assert_eq!(compute_stats(&[plain(2), plain(3)], Period::Month, now()).streak, 0);
        assert_eq!(compute_stats(&with_today, Period::Month, now()).longest_streak, 4);
    }

    #[test]
    fn lays_out_26_monday_first_weeks_ending_this_week() {
        let st = compute_stats(&[plain(0), plain(0), plain(1)], Period::Week, now());
        assert_eq!(st.weeks.len(), MAP_WEEKS);
        let last = st.weeks.last().unwrap();
        assert_eq!(last[0].date, NaiveDate::from_ymd_opt(2026, 11, 2).unwrap());
        assert_eq!((last[1].ago, last[1].count), (0, 2));
        assert_eq!((last[0].ago, last[0].count), (1, 1));
        assert_eq!(last[2].ago, -1);
        let b = st.busiest_day.unwrap();
        assert_eq!((b.ago, b.count), (0, 2));
    }

    #[test]
    fn charts_the_hour_each_session_started_and_the_late_ones() {
        let h = |days, hour| s(days, move |d| d.created_at = Some(at(days, hour)));
        let st = compute_stats(&[h(0, 9), h(1, 15), h(2, 15), h(3, 23)], Period::Week, now());
        assert_eq!((st.hours[15], st.peak_hour, st.late_night), (2, Some(15), 1));
    }

    #[test]
    fn ranks_repositories_and_keeps_every_agent_in_sidebar_order() {
        let r = |days, repo: &str, agent| s(days, move |d| {
            d.repository = Some(repo.into());
            d.agent = agent;
        });
        let all = [r(0, "me/a", Agent::Copilot), r(1, "me/b", Agent::Copilot), r(2, "me/b", Agent::Copilot), r(40, "me/c", Agent::ClaudeCode)];
        let st = compute_stats(&all, Period::Week, now());
        assert_eq!(st.repos, [("me/b".to_string(), 2), ("me/a".to_string(), 1)]);
        assert_eq!(st.repo_count, 2);
        assert_eq!(st.agents, [(Agent::Copilot, 3), (Agent::ClaudeCode, 0)]);
    }

    #[test]
    fn is_empty_not_broken_with_no_sessions() {
        let st = compute_stats(&[], Period::Month, now());
        assert_eq!((st.sessions, st.streak, st.peak_hour), (0, 0, None));
        assert!(st.busiest_day.is_none() && st.chattiest.is_none());
        assert!(facts(&st, |a| format!("{a:?}")).is_empty());
    }

    #[test]
    fn facts_only_say_what_the_stats_support() {
        let one = compute_stats(&[s(0, |d| d.message_count = 1)], Period::Week, now());
        let lines = facts(&one, |_| String::new());
        assert_eq!(lines[0], "You and your agents traded 1 message in this period.");
        assert!(!lines.iter().any(|l| l.contains("weekend") || l.contains("streak")));
        let st = compute_stats(&[plain(0), plain(1), s(2, |d| d.agent = Agent::ClaudeCode)], Period::Week, now());
        let name = |a| if a == Agent::ClaudeCode { "Claude Code".to_string() } else { "Copilot CLI".to_string() };
        assert!(facts(&st, name).contains(&"Copilot CLI ran 67% of sessions in this period.".to_string()));
    }

    #[test]
    fn levels_greetings_and_thousands() {
        assert_eq!([0, 1, 2, 3, 4, 5, 6, 30].map(level), [0, 1, 2, 2, 3, 3, 4, 4]);
        let g = |h| greeting(local(2026, 1, 1, h, 0));
        assert_eq!((g(2), g(9), g(15), g(20)), ("Up late", "Good morning", "Good afternoon", "Good evening"));
        assert_eq!((thousands(736), thousands(1234), thousands(1234567)), ("736".into(), "1,234".into(), "1,234,567".into()));
    }
}
