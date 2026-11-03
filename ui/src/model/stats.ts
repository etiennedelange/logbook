import type { Agent, Session } from '../types';
import { presentAgents } from './agents';
import { repoKey, timeOf } from './select';
import { plural, sessionTitle, shortRepo } from './format';

/** The Stats view's periods: the last 7 or 30 days, as in the sidebar, or the 26 weeks the activity map shows. */
export type Period = 'week' | 'month' | 'half';

export const PERIODS: { key: Period; label: string; days: number }[] = [
  { key: 'week', label: 'Last 7 days', days: 7 },
  { key: 'month', label: 'Last 30 days', days: 30 },
  { key: 'half', label: '26 weeks', days: 26 * 7 },
];

/** Weeks the activity map shows, Monday to Sunday, ending with this week. */
export const MAP_WEEKS = 26;

export interface Day {
  /** Local midnight. */
  date: Date;
  /** Days before today: 0 is today, negative is later this week. */
  ago: number;
  count: number;
}

export interface Stats {
  /** The sessions in the period. */
  sessions: number;
  messages: number;
  /** Days in the period with at least one session. */
  activeDays: number;
  /** Days in a row with a session, up to today; a day without one yet today doesn't break it. */
  streak: number;
  /** The longest run of such days inside the period. */
  longestStreak: number;
  /** `MAP_WEEKS` weeks of 7 days, Monday first, oldest week first. */
  weeks: Day[][];
  /** Sessions by the hour they started, 0 to 23. */
  hours: number[];
  peakHour: number | null;
  /** Sessions started from 22:00 to 03:59. */
  lateNight: number;
  weekend: number;
  /** Sessions by weekday, Monday first. */
  weekdays: number[];
  busiestDay: Day | null;
  agents: { agent: Agent; count: number }[];
  /** Repositories with a session in the period. */
  repoCount: number;
  /** The busiest repositories, at most five. */
  repos: { name: string; count: number }[];
  /** The session with the most messages. */
  chattiest: Session | null;
}

const DAY_MS = 86_400_000;

const midnight = (d: Date) => new Date(d.getFullYear(), d.getMonth(), d.getDate());
/** Whole days between two local midnights, safe across a daylight-saving change. */
const daysBetween = (a: Date, b: Date) => Math.round((a.getTime() - b.getTime()) / DAY_MS);
/** Monday 0 to Sunday 6. */
export const weekday = (d: Date) => (d.getDay() + 6) % 7;

/** When the session started, for the hour chart; its last activity if it has no start. */
function startOf(s: Session): Date | null {
  const at = s.createdAt ?? s.updatedAt;
  return at === null ? null : new Date(at);
}

/**
 * Everything the Stats view shows for the period ending today. Sessions count on the day they were
 * last active, as the sidebar's ranges count them, so "Last 7 days" here matches its badge.
 */
export function computeStats(all: Session[], period: Period, now: Date): Stats {
  const today = midnight(now);
  const days = PERIODS.find((p) => p.key === period)!.days;
  const mapStart = new Date(
    today.getFullYear(),
    today.getMonth(),
    today.getDate() - weekday(today) - (MAP_WEEKS - 1) * 7,
  );

  const perDay = new Map<number, number>();
  const inPeriod: Session[] = [];
  for (const s of all) {
    const t = timeOf(s);
    if (t === null) continue;
    const ago = daysBetween(today, midnight(new Date(t)));
    if (ago < 0) continue; // a clock that ran ahead
    perDay.set(ago, (perDay.get(ago) ?? 0) + 1);
    if (ago < days) inPeriod.push(s);
  }

  const weeks: Day[][] = [];
  for (let w = 0; w < MAP_WEEKS; w++) {
    const week: Day[] = [];
    for (let d = 0; d < 7; d++) {
      const date = new Date(mapStart.getFullYear(), mapStart.getMonth(), mapStart.getDate() + w * 7 + d);
      const ago = daysBetween(today, date);
      week.push({ date, ago, count: ago < 0 ? 0 : (perDay.get(ago) ?? 0) });
    }
    weeks.push(week);
  }

  let activeDays = 0;
  let longestStreak = 0;
  let run = 0;
  let busiestDay: Day | null = null;
  for (let ago = days - 1; ago >= 0; ago--) {
    const count = perDay.get(ago) ?? 0;
    if (count > 0) activeDays++;
    run = count > 0 ? run + 1 : 0;
    longestStreak = Math.max(longestStreak, run);
    if (count > 0 && (!busiestDay || count > busiestDay.count)) {
      busiestDay = { date: new Date(today.getFullYear(), today.getMonth(), today.getDate() - ago), ago, count };
    }
  }
  let streak = 0;
  for (let ago = perDay.has(0) ? 0 : 1; perDay.has(ago); ago++) streak++;

  const hours = Array<number>(24).fill(0);
  const weekdays = Array<number>(7).fill(0);
  let messages = 0;
  let chattiest: Session | null = null;
  const agentCounts = new Map<Agent, number>();
  const repoCounts = new Map<string, number>();
  for (const s of inPeriod) {
    messages += s.messageCount;
    if (!chattiest || s.messageCount > chattiest.messageCount) chattiest = s;
    const start = startOf(s);
    if (start) hours[start.getHours()]!++;
    weekdays[weekday(new Date(timeOf(s)!))]!++;
    agentCounts.set(s.agent, (agentCounts.get(s.agent) ?? 0) + 1);
    const repo = repoKey(s);
    if (repo) repoCounts.set(repo, (repoCounts.get(repo) ?? 0) + 1);
  }
  const peak = Math.max(...hours);

  return {
    sessions: inPeriod.length,
    messages,
    activeDays,
    streak,
    longestStreak,
    weeks,
    hours,
    peakHour: peak > 0 ? hours.indexOf(peak) : null,
    lateNight: [22, 23, 0, 1, 2, 3].reduce((n, h) => n + hours[h]!, 0),
    weekend: weekdays[5]! + weekdays[6]!,
    weekdays,
    busiestDay,
    // Every agent that has sessions at all keeps its row, in the sidebar's order.
    agents: presentAgents(all).map((agent) => ({ agent, count: agentCounts.get(agent) ?? 0 })),
    repoCount: repoCounts.size,
    repos: [...repoCounts]
      .map(([name, count]) => ({ name, count }))
      .sort((a, b) => b.count - a.count || a.name.localeCompare(b.name))
      .slice(0, 5),
    chattiest,
  };
}

/** The activity map's shade for a day's count: 0 for none, then 1 to 4. */
export const level = (count: number): 0 | 1 | 2 | 3 | 4 =>
  count === 0 ? 0 : count === 1 ? 1 : count <= 3 ? 2 : count <= 5 ? 3 : 4;

export const hh = (h: number) => `${String(h).padStart(2, '0')}:00`;

const pct = (n: number, of: number) => Math.round((n * 100) / Math.max(1, of));

/** The fun-fact strip: short lines worked out from the stats, only the ones the data supports. */
export function facts(s: Stats, agentName: (a: Agent) => string): string[] {
  if (s.sessions === 0) return [];
  const out: string[] = [];
  out.push(
    `You and your agents traded ${s.messages.toLocaleString('en-US')} ${s.messages === 1 ? 'message' : 'messages'} in this period.`,
  );
  if (s.peakHour !== null) {
    const afternoon = s.peakHour >= 12 && s.peakHour < 18;
    out.push(`Your sessions peak around ${hh(s.peakHour)}.${afternoon ? ' Afternoon slump? Not here.' : ''}`);
  }
  const [r0, r1, r2] = s.repos;
  if (r0 && r1 && r2 && r0.count > r1.count + r2.count) {
    out.push(`${shortRepo(r0.name)} got more sessions than ${shortRepo(r1.name)} and ${shortRepo(r2.name)} combined.`);
  } else if (r0) {
    out.push(`${shortRepo(r0.name)} is your busiest repository, with ${plural(r0.count, 'session', 'sessions')}.`);
  }
  if (s.sessions >= 5) {
    const w = pct(s.weekend, s.sessions);
    out.push(
      w <= 15 ? `Only ${w}% of sessions were at the weekend. Healthy.` : `${w}% of sessions were at the weekend.`,
    );
  }
  if (s.streak > 1) out.push(`You’re on a ${s.streak}-day streak. One session tomorrow keeps it going.`);
  const used = s.agents.filter((a) => a.count > 0);
  if (used.length > 1) {
    const top = [...used].sort((a, b) => b.count - a.count)[0]!;
    out.push(`${agentName(top.agent)} ran ${pct(top.count, s.sessions)}% of sessions in this period.`);
  }
  if (s.chattiest && s.chattiest.messageCount > 1) {
    out.push(`Chattiest session: “${sessionTitle(s.chattiest).text}”, with ${s.chattiest.messageCount} messages.`);
  }
  return out;
}

/** The greeting at the top of the Stats view. */
export function greeting(now: Date): string {
  const h = now.getHours();
  return h < 5 ? 'Up late' : h < 12 ? 'Good morning' : h < 18 ? 'Good afternoon' : 'Good evening';
}
