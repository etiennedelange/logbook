//! The agents Logbook reads: names, resume commands, and which are present. A port of `agents.ts`.

use super::Session;
use logbook_core::models::Agent;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

pub struct AgentInfo {
    pub key: Agent,
    pub name: &'static str,
    /// The muted tag on list rows: short and lowercase, so a row reads the same with two agents or ten.
    pub short: &'static str,
    /// Index into the theme's categorical colours. A secondary cue: the name identifies an agent.
    pub cat: usize,
    /// Where its sessions live, when the backend hasn't said.
    pub home: &'static str,
    /// What resumes a session, as About and tooltips describe it.
    pub resume: &'static str,
    /// How its replies are labelled in a transcript.
    pub speaker: &'static str,
    /// For an agent that deletes old sessions: how long it keeps them, and the setting that says so.
    pub retention: Option<(u32, &'static str)>,
}

/// Every agent Logbook reads, in sidebar order (and `Alt+1`, `Alt+2`, ... order).
pub const AGENTS: [AgentInfo; 2] = [
    AgentInfo {
        key: Agent::Copilot,
        name: "Copilot CLI",
        short: "copilot",
        cat: 0,
        home: "~/.copilot/session-state",
        resume: "copilot --resume=<id>",
        speaker: "Copilot",
        retention: None,
    },
    AgentInfo {
        key: Agent::ClaudeCode,
        name: "Claude Code",
        short: "claude",
        cat: 2,
        home: "~/.claude/projects",
        resume: "claude --resume <id>",
        speaker: "Claude",
        retention: Some((30, "cleanupPeriodDays")),
    },
];

pub fn agent_info(key: Agent) -> &'static AgentInfo {
    AGENTS.iter().find(|a| a.key == key).unwrap_or(&AGENTS[0])
}

/// Agents that have at least one session, most sessions first (ties in `AGENTS` order). The sidebar's
/// order and the `Alt+1`, `Alt+2`, ... order, so it uses every session rather than the current
/// filters: it would jump around otherwise.
pub fn present_agents(sessions: &[Arc<Session>]) -> Vec<Agent> {
    let mut counts: HashMap<Agent, usize> = HashMap::new();
    for s in sessions {
        *counts.entry(s.agent).or_default() += 1;
    }
    let mut present: Vec<Agent> = AGENTS.iter().map(|a| a.key).filter(|k| counts.contains_key(k)).collect();
    present.sort_by_key(|k| std::cmp::Reverse(counts[k]));
    present
}

/// Agents the sidebar lists before "Show all".
pub const AGENT_ROWS: usize = 4;

/// Which of `present` the sidebar shows: the first `AGENT_ROWS`, or all when expanded. The picked
/// agent always shows, so the active filter is never hidden.
pub fn shown_agents<T: PartialEq + Copy>(present: &[T], active: Option<T>, expanded: bool) -> Vec<T> {
    if expanded || present.len() <= AGENT_ROWS {
        return present.to_vec();
    }
    let mut top = present[..AGENT_ROWS].to_vec();
    if let Some(a) = active {
        if !top.contains(&a) && present.contains(&a) {
            top.push(a);
        }
    }
    top
}

/// An agent's session folder, which Logbook reads and never writes to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceInfo {
    pub agent: Agent,
    pub dir: String,
    /// `dir` with the home folder written as `~`, for showing.
    pub short: String,
    /// The folder exists: the agent is installed and has run.
    pub found: bool,
}

pub fn source_infos(sources: Vec<(Agent, Vec<PathBuf>)>) -> Vec<SourceInfo> {
    sources
        .into_iter()
        .flat_map(|(agent, roots)| {
            roots.into_iter().map(move |r| SourceInfo {
                agent,
                dir: r.display().to_string(),
                short: logbook_core::paths::tilde(&r),
                found: r.is_dir(),
            })
        })
        .collect()
}

/// One agent in the sidebar's Sources popover: its folders and what Logbook has from them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceRow {
    pub agent: Agent,
    pub dirs: Vec<SourceInfo>,
    /// At least one of its folders exists.
    pub found: bool,
    /// Every session of this agent in the index, whatever the filters.
    pub count: usize,
    /// Of those, the ones whose file the agent deleted and Logbook kept.
    pub kept: usize,
}

/// The backend's folders grouped by agent: agents with folders first, most sessions first (ties in
/// `AGENTS` order, as in the sidebar), then the ones whose folders don't exist yet.
pub fn source_rows(sources: &[SourceInfo], sessions: &[Arc<Session>]) -> Vec<SourceRow> {
    let mut rows: Vec<SourceRow> = AGENTS
        .iter()
        .filter_map(|a| {
            let dirs: Vec<SourceInfo> = sources.iter().filter(|s| s.agent == a.key).cloned().collect();
            (!dirs.is_empty()).then(|| SourceRow {
                agent: a.key,
                found: dirs.iter().any(|d| d.found),
                dirs,
                count: 0,
                kept: 0,
            })
        })
        .collect();
    for s in sessions {
        if let Some(r) = rows.iter_mut().find(|r| r.agent == s.agent) {
            r.count += 1;
            if s.source_deleted_at.is_some() {
                r.kept += 1;
            }
        }
    }
    rows.sort_by(|a, b| b.found.cmp(&a.found).then(b.count.cmp(&a.count)));
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::test_support::*;

    fn claude(id: &str) -> Arc<Session> {
        session(&format!("claude:{id}"), |s| s.agent = Agent::ClaudeCode)
    }

    #[test]
    fn present_agents_most_sessions_first_ties_in_the_usual_order() {
        let copilot = session("copilot:a", |_| {});
        assert_eq!(present_agents(&[copilot.clone()]), [Agent::Copilot]);
        assert_eq!(present_agents(&[copilot.clone(), claude("x")]), [Agent::Copilot, Agent::ClaudeCode]);
        assert_eq!(present_agents(&[copilot, claude("x"), claude("y")]), [Agent::ClaudeCode, Agent::Copilot]);
    }

    #[test]
    fn shown_agents_never_hides_the_picked_one() {
        let six = ['a', 'b', 'c', 'd', 'e', 'f'];
        assert_eq!(shown_agents(&six, None, false), ['a', 'b', 'c', 'd']);
        assert_eq!(shown_agents(&six, None, true), six);
        assert_eq!(shown_agents(&six[..4], None, false).len(), 4);
        assert_eq!(shown_agents(&six, Some('f'), false), ['a', 'b', 'c', 'd', 'f']);
        assert_eq!(shown_agents(&six, Some('b'), false), ['a', 'b', 'c', 'd']);
    }

    fn dir(agent: Agent, found: bool) -> SourceInfo {
        SourceInfo { agent, dir: "/x".into(), short: "~/x".into(), found }
    }

    #[test]
    fn source_rows_group_folders_by_agent_with_counts() {
        let sessions = [
            session("copilot:a", |_| {}),
            claude("x"),
            session("claude:y", |s| {
                s.agent = Agent::ClaudeCode;
                s.source_deleted_at = Some("2026-10-24T00:00:00Z".into());
            }),
        ];
        let rows = source_rows(&[dir(Agent::Copilot, true), dir(Agent::ClaudeCode, true)], &sessions);
        assert_eq!(rows.iter().map(|r| (r.agent, r.count, r.kept)).collect::<Vec<_>>(), [
            (Agent::ClaudeCode, 2, 1),
            (Agent::Copilot, 1, 0),
        ]);
        let rows = source_rows(&[dir(Agent::Copilot, false), dir(Agent::ClaudeCode, true)], &[]);
        assert_eq!(rows.iter().map(|r| (r.agent, r.found)).collect::<Vec<_>>(), [
            (Agent::ClaudeCode, true),
            (Agent::Copilot, false),
        ]);
    }
}
