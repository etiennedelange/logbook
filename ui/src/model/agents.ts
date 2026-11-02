import type { Agent, Session } from '../types';

export interface AgentInfo {
  key: Agent;
  name: string;
  /** The muted tag on list rows: short and lowercase, so a row reads the same with two agents or ten. */
  short: string;
  /**
   * Its sidebar dot. A secondary cue only: there are a handful of colours that read as different, so
   * the name, not the colour, is what identifies an agent.
   */
  color: string;
  /** Where its sessions live, when About has no answer from the backend. */
  home: string;
  /** What resumes a session, as About and tooltips describe it. */
  resume: string;
  /** How its replies are labelled in a transcript. */
  speaker: string;
  /** For an agent that deletes old sessions: how long it keeps them, and the setting that says so. */
  retention?: { days: number; setting: string };
}

/** Every agent Logbook reads, in sidebar order (and `Alt+1`, `Alt+2`, ... order). */
export const AGENTS: AgentInfo[] = [
  {
    key: 'copilot',
    name: 'Copilot CLI',
    short: 'copilot',
    color: 'var(--cat-1)',
    home: '~/.copilot/session-state',
    resume: 'copilot --resume=<id>',
    speaker: 'Copilot',
  },
  {
    key: 'claude',
    name: 'Claude Code',
    short: 'claude',
    color: 'var(--cat-3)',
    home: '~/.claude/projects',
    resume: 'claude --resume <id>',
    speaker: 'Claude',
    retention: { days: 30, setting: 'cleanupPeriodDays' },
  },
];

const byKey = new Map(AGENTS.map((a) => [a.key, a]));

export const agentInfo = (key: Agent): AgentInfo => byKey.get(key) ?? AGENTS[0]!;

const presentCache = new WeakMap<Session[], Agent[]>();

/**
 * Agents that have at least one session, most sessions first (ties in `AGENTS` order). This is
 * the sidebar's order and the `Alt+1`, `Alt+2`, ... order, so it uses every session rather than
 * the current filters: it would jump around otherwise. The agent filter and tags only appear
 * when there are two or more.
 */
export function presentAgents(sessions: Session[]): Agent[] {
  let present = presentCache.get(sessions);
  if (!present) {
    const counts = new Map<Agent, number>();
    for (const s of sessions) counts.set(s.agent, (counts.get(s.agent) ?? 0) + 1);
    present = AGENTS.filter((a) => counts.has(a.key))
      .sort((a, b) => counts.get(b.key)! - counts.get(a.key)!)
      .map((a) => a.key);
    presentCache.set(sessions, present);
  }
  return present;
}

/** Agents the sidebar lists before "Show all". */
export const AGENT_ROWS = 4;

/**
 * Which of `present` the sidebar shows: the first `AGENT_ROWS`, or all of them when expanded.
 * The picked agent always shows, so the active filter is never hidden.
 */
export function shownAgents<T extends string>(present: T[], active: T | null, expanded: boolean): T[] {
  if (expanded || present.length <= AGENT_ROWS) return present;
  const top = present.slice(0, AGENT_ROWS);
  return active !== null && !top.includes(active) && present.includes(active) ? [...top, active] : top;
}
