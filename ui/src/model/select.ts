import type { SearchHit, Session } from '../types';
import { repoOf } from './format';

export type Range = 'all' | 'today' | 'week' | 'month';

export const RANGES: { key: Range; label: string; hotkey: string }[] = [
  { key: 'all', label: 'All sessions', hotkey: '1' },
  { key: 'today', label: 'Today', hotkey: '2' },
  { key: 'week', label: 'Last 7 days', hotkey: '3' },
  { key: 'month', label: 'Last 30 days', hotkey: '4' },
];

/** Start of the range in local time, or null for all time. */
export function rangeStart(range: Range, now: Date): Date | null {
  if (range === 'all') return null;
  const d = new Date(now.getFullYear(), now.getMonth(), now.getDate());
  if (range === 'week') d.setDate(d.getDate() - 6);
  if (range === 'month') d.setDate(d.getDate() - 29);
  return d;
}

// Sessions are immutable once fetched, so each one's timestamp and repository are worked
// out once rather than on every filter click and render.
const times = new WeakMap<Session, number | null>();
const repoKeys = new WeakMap<Session, string | null>();

/** When the session was last active, in ms since the epoch. */
export function timeOf(s: Session): number | null {
  let t = times.get(s);
  if (t === undefined) {
    const at = s.updatedAt ?? s.createdAt;
    t = at === null ? null : new Date(at).getTime();
    times.set(s, t);
  }
  return t;
}

/** `repoOf`, remembered per session. */
export function repoKey(s: Session): string | null {
  let r = repoKeys.get(s);
  if (r === undefined) {
    r = repoOf(s);
    repoKeys.set(s, r);
  }
  return r;
}

const indexes = new WeakMap<Session[], Map<string, Session>>();

/** Sessions by id, built once per fetched list. */
export function sessionsById(sessions: Session[]): Map<string, Session> {
  let byId = indexes.get(sessions);
  if (!byId) {
    byId = new Map(sessions.map((s) => [s.id, s]));
    indexes.set(sessions, byId);
  }
  return byId;
}

/** A test for "is this session in the range", with the range's start worked out once. */
function rangeTest(range: Range, now: Date): (s: Session) => boolean {
  const start = rangeStart(range, now)?.getTime();
  if (start === undefined) return () => true;
  return (s) => {
    const t = timeOf(s);
    return t !== null && t >= start;
  };
}

export const inRange = (s: Session, range: Range, now: Date): boolean => rangeTest(range, now)(s);

export interface Row {
  session: Session;
  snippet: string | null;
}

/** What the list shows: search hits when there is a query, otherwise every session; then the range and repo filters. */
export function visibleRows(
  sessions: Session[],
  hits: SearchHit[] | null,
  range: Range,
  repo: string | null,
  now: Date,
): Row[] {
  let rows: Row[];
  if (hits) {
    const byId = sessionsById(sessions);
    rows = [];
    for (const h of hits) {
      const session = byId.get(h.id);
      // A hit for a session that isn't loaded yet; `refresh` re-runs the search once it is.
      if (session) rows.push({ session, snippet: h.snippet });
    }
  } else {
    rows = sessions.map((session) => ({ session, snippet: null }));
  }
  const ok = rangeTest(range, now);
  return rows.filter((r) => ok(r.session) && (repo === null || repoKey(r.session) === repo));
}

export interface RepoCount {
  name: string;
  count: number;
}

/** Repositories in the given range, busiest first. */
export function repos(sessions: Session[], range: Range, now: Date): RepoCount[] {
  const counts = new Map<string, number>();
  const ok = rangeTest(range, now);
  for (const s of sessions) {
    const name = repoKey(s);
    if (name && ok(s)) counts.set(name, (counts.get(name) ?? 0) + 1);
  }
  return [...counts]
    .map(([name, count]) => ({ name, count }))
    .sort((a, b) => b.count - a.count || a.name.localeCompare(b.name));
}

/** How many sessions each time range holds, for the sidebar. */
export function rangeCounts(sessions: Session[], now: Date): Record<Range, number> {
  const counts = {} as Record<Range, number>;
  for (const r of RANGES) {
    const ok = rangeTest(r.key, now);
    counts[r.key] = sessions.reduce((n, s) => (ok(s) ? n + 1 : n), 0);
  }
  return counts;
}
