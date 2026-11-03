import { describe, expect, it } from 'vite-plus/test';
import { session } from '../test/fixtures';
import { presentAgents, shownAgents, sourceRows } from './agents';

describe('presentAgents', () => {
  it('lists agents with sessions, most sessions first, ties in the usual order', () => {
    const copilot = session({ id: 'copilot:a' });
    const claude = (id: string) => session({ id: `claude:${id}`, agent: 'claude' });
    expect(presentAgents([copilot])).toEqual(['copilot']);
    expect(presentAgents([copilot, claude('x')])).toEqual(['copilot', 'claude']);
    expect(presentAgents([copilot, claude('x'), claude('y')])).toEqual(['claude', 'copilot']);
  });
});

describe('shownAgents', () => {
  const six = ['a', 'b', 'c', 'd', 'e', 'f'];

  it('shows the first four until expanded', () => {
    expect(shownAgents(six, null, false)).toEqual(['a', 'b', 'c', 'd']);
    expect(shownAgents(six, null, true)).toEqual(six);
    expect(shownAgents(six.slice(0, 4), null, false)).toHaveLength(4);
  });

  it('never hides the picked agent', () => {
    expect(shownAgents(six, 'f', false)).toEqual(['a', 'b', 'c', 'd', 'f']);
    expect(shownAgents(six, 'b', false)).toEqual(['a', 'b', 'c', 'd']);
  });
});

describe('sourceRows', () => {
  const dir = (agent: 'copilot' | 'claude', found = true) => ({ agent, dir: `/${agent}`, short: `~/${agent}`, found });

  it('groups folders by agent with their session and kept counts, most sessions first', () => {
    const sessions = [
      session({ id: 'copilot:a' }),
      session({ id: 'claude:x', agent: 'claude' }),
      session({ id: 'claude:y', agent: 'claude', sourceDeletedAt: '2026-10-24T00:00:00Z' }),
    ];
    expect(sourceRows([dir('copilot'), dir('claude')], sessions)).toEqual([
      { agent: 'claude', dirs: [dir('claude')], found: true, count: 2, kept: 1 },
      { agent: 'copilot', dirs: [dir('copilot')], found: true, count: 1, kept: 0 },
    ]);
  });

  it('puts agents whose folder does not exist last', () => {
    const rows = sourceRows([dir('copilot', false), dir('claude')], []);
    expect(rows.map((r) => [r.agent, r.found])).toEqual([
      ['claude', true],
      ['copilot', false],
    ]);
  });
});
