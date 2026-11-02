import { describe, expect, it } from 'vite-plus/test';
import { session } from '../test/fixtures';
import { presentAgents, shownAgents } from './agents';

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
