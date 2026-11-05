import { describe, expect, it } from 'vite-plus/test';
import { hit, session } from '../test/fixtures';
import { agentCounts, rangeStart, repos, visibleRows } from './select';

const now = new Date(2026, 9, 13, 12, 0);
const at = (days: number) => new Date(now.getTime() - days * 86_400_000).toISOString();

const a = session({ id: 'a', repository: 'org/a', updatedAt: at(0) });
const b = session({ id: 'b', repository: 'org/b', updatedAt: at(3) });
const c = session({ id: 'c', repository: 'org/a', updatedAt: at(20) });

describe('visibleRows', () => {
  it('filters sessions by range and repository', () => {
    const ids = (r: ReturnType<typeof visibleRows>) => r.map((x) => x.session.id);
    expect(ids(visibleRows([a, b, c], null, 'all', null, now))).toEqual(['a', 'b', 'c']);
    expect(ids(visibleRows([a, b, c], null, 'today', null, now))).toEqual(['a']);
    expect(ids(visibleRows([a, b, c], null, 'week', null, now))).toEqual(['a', 'b']);
    expect(ids(visibleRows([a, b, c], null, 'all', 'org/a', now))).toEqual(['a', 'c']);
  });

  it('shows search hits, with their snippets, instead of all sessions', () => {
    const h = hit(c, 'x \u0002y\u0003');
    const rows = visibleRows([a, b, c], [h], 'all', null, now);
    expect(rows).toEqual([{ session: c, snippet: 'x \u0002y\u0003', hit: h }]);
    expect(visibleRows([a, b, c], [hit(c, null)], 'week', null, now)).toEqual([]);
  });
});

describe('repos', () => {
  it('counts sessions per repository in range, busiest first', () => {
    expect(repos([a, b, c], 'all', now)).toEqual([
      { name: 'org/a', count: 2 },
      { name: 'org/b', count: 1 },
    ]);
    expect(repos([a, b, c], 'today', now)).toEqual([{ name: 'org/a', count: 1 }]);
  });
});

describe('rangeStart', () => {
  it('starts at local midnight', () => {
    expect(rangeStart('today', now)).toEqual(new Date(2026, 9, 13));
    expect(rangeStart('week', now)).toEqual(new Date(2026, 9, 7));
    expect(rangeStart('all', now)).toBeNull();
  });
});

describe('agents', () => {
  const x = session({ id: 'claude:x', agent: 'claude', repository: 'org/a', updatedAt: at(1) });
  const old = session({ id: 'claude:old', agent: 'claude', repository: 'org/b', updatedAt: at(40) });
  const all = [a, b, c, x, old];

  it('filters rows by agent, combined with range and repository', () => {
    const ids = (r: ReturnType<typeof visibleRows>) => r.map((x) => x.session.id);
    expect(ids(visibleRows(all, null, 'all', null, now, 'claude'))).toEqual(['claude:x', 'claude:old']);
    expect(ids(visibleRows(all, null, 'week', 'org/a', now, 'claude'))).toEqual(['claude:x']);
    expect(ids(visibleRows(all, null, 'all', 'org/a', now, 'copilot'))).toEqual(['a', 'c']);
  });

  it('counts each agent in the range and repository, keeping agents with none', () => {
    expect(agentCounts(all, ['copilot', 'claude'], 'all', null, now)).toEqual([
      { agent: 'copilot', count: 3 },
      { agent: 'claude', count: 2 },
    ]);
    expect(agentCounts(all, ['copilot', 'claude'], 'today', 'org/a', now)).toEqual([
      { agent: 'copilot', count: 1 },
      { agent: 'claude', count: 0 },
    ]);
  });

  it("lists only the picked agent's repositories", () => {
    expect(repos(all, 'all', now, 'claude').map((r) => r.name)).toEqual(['org/a', 'org/b']);
    expect(repos(all, 'all', now, 'copilot')).toEqual([
      { name: 'org/a', count: 2 },
      { name: 'org/b', count: 1 },
    ]);
  });
});
