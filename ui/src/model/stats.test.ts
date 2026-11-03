import { describe, expect, it } from 'vite-plus/test';
import { session } from '../test/fixtures';
import { MAP_WEEKS, computeStats, facts, greeting, level } from './stats';

// Tuesday, 3 Nov 2026, mid-afternoon.
const now = new Date(2026, 10, 3, 15, 30);
/** A local time `days` before today at `hour`. */
const at = (days: number, hour = 10) => new Date(2026, 10, 3 - days, hour, 0).toISOString();

let n = 0;
const s = (days: number, over: Parameters<typeof session>[0] = {}) =>
  session({ id: `s${n++}`, updatedAt: at(days), createdAt: at(days), ...over });

describe('computeStats', () => {
  it('counts the period by last activity, as the sidebar does', () => {
    const all = [s(0), s(0), s(3), s(6), s(7), s(29), s(40)];
    expect(computeStats(all, 'week', now).sessions).toBe(4);
    expect(computeStats(all, 'month', now).sessions).toBe(6);
    expect(computeStats(all, 'half', now).sessions).toBe(7);
  });

  it('adds up messages and active days', () => {
    const st = computeStats(
      [s(0, { messageCount: 10 }), s(0, { messageCount: 4 }), s(2, { messageCount: 6 })],
      'week',
      now,
    );
    expect(st.messages).toBe(20);
    expect(st.activeDays).toBe(2);
    expect(st.chattiest?.messageCount).toBe(10);
  });

  it('keeps the streak going when today has no session yet', () => {
    const run = [s(1), s(2), s(3), s(5)];
    expect(computeStats(run, 'month', now).streak).toBe(3);
    expect(computeStats([s(0), ...run], 'month', now).streak).toBe(4);
    expect(computeStats([s(2), s(3)], 'month', now).streak).toBe(0);
    expect(computeStats([s(0), ...run], 'month', now).longestStreak).toBe(4);
  });

  it('lays out 26 Monday-first weeks ending this week', () => {
    const st = computeStats([s(0), s(0), s(1)], 'week', now);
    expect(st.weeks).toHaveLength(MAP_WEEKS);
    const last = st.weeks.at(-1)!;
    expect(last[0]!.date).toEqual(new Date(2026, 10, 2)); // Monday
    expect(last[1]).toMatchObject({ ago: 0, count: 2 });
    expect(last[0]).toMatchObject({ ago: 1, count: 1 });
    expect(last[2]!.ago).toBe(-1); // tomorrow: drawn empty
    expect(st.busiestDay).toMatchObject({ ago: 0, count: 2 });
  });

  it('charts the hour each session started, and the late ones', () => {
    const st = computeStats(
      [
        s(0, { createdAt: at(0, 9) }),
        s(1, { createdAt: at(1, 15) }),
        s(2, { createdAt: at(2, 15) }),
        s(3, { createdAt: at(3, 23) }),
      ],
      'week',
      now,
    );
    expect(st.hours[15]).toBe(2);
    expect(st.peakHour).toBe(15);
    expect(st.lateNight).toBe(1);
  });

  it('ranks repositories and keeps every agent in sidebar order', () => {
    const st = computeStats(
      [
        s(0, { repository: 'me/a' }),
        s(1, { repository: 'me/b' }),
        s(2, { repository: 'me/b' }),
        s(40, { repository: 'me/c', agent: 'claude' }),
      ],
      'week',
      now,
    );
    expect(st.repos).toEqual([
      { name: 'me/b', count: 2 },
      { name: 'me/a', count: 1 },
    ]);
    expect(st.repoCount).toBe(2);
    expect(st.agents).toEqual([
      { agent: 'copilot', count: 3 },
      { agent: 'claude', count: 0 },
    ]);
  });

  it('is empty, not broken, with no sessions', () => {
    const st = computeStats([], 'month', now);
    expect(st).toMatchObject({ sessions: 0, streak: 0, peakHour: null, busiestDay: null, chattiest: null });
    expect(facts(st, String)).toEqual([]);
  });
});

describe('facts', () => {
  it('only says what the stats support', () => {
    const one = computeStats([s(0, { messageCount: 1 })], 'week', now);
    const lines = facts(one, String);
    expect(lines[0]).toBe('You and your agents traded 1 message in this period.');
    expect(lines.some((l) => l.includes('weekend'))).toBe(false); // too few to say
    expect(lines.some((l) => l.includes('streak'))).toBe(false);
  });

  it('names the busiest agent when there are two', () => {
    const st = computeStats([s(0), s(1), s(2, { agent: 'claude' })], 'week', now);
    expect(facts(st, (a) => (a === 'claude' ? 'Claude Code' : 'Copilot CLI'))).toContain(
      'Copilot CLI ran 67% of sessions in this period.',
    );
  });
});

describe('level and greeting', () => {
  it('shades by count', () => {
    expect([0, 1, 2, 3, 4, 5, 6, 30].map(level)).toEqual([0, 1, 2, 2, 3, 3, 4, 4]);
  });
  it('greets by the clock', () => {
    expect(greeting(new Date(2026, 0, 1, 2))).toBe('Up late');
    expect(greeting(new Date(2026, 0, 1, 9))).toBe('Good morning');
    expect(greeting(new Date(2026, 0, 1, 15))).toBe('Good afternoon');
    expect(greeting(new Date(2026, 0, 1, 20))).toBe('Good evening');
  });
});
