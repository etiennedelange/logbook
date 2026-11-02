import type { SearchHit, Session, TimelineEntry } from '../types';

export const ago = (min: number, now = Date.now()) => new Date(now - min * 60_000).toISOString();

export function session(over: Partial<Session> = {}): Session {
  const id = over.id ?? 'copilot:a1b2c3d4-0000-4000-8000-000000000001';
  const nativeId = over.nativeId ?? id.replace(/^[a-z-]+:/, '');
  return {
    id,
    agent: 'copilot',
    nativeId,
    name: 'Fix the flaky overdue-loans test',
    userNamed: true,
    cwd: 'C:\\Repos\\bookshelf',
    gitRoot: 'C:\\Repos\\bookshelf',
    repository: 'me/bookshelf',
    branch: 'fix/overdue-flake',
    clientName: 'copilot-cli',
    createdAt: ago(90),
    updatedAt: ago(5),
    messageCount: 12,
    firstPrompt: 'The overdue-loans test fails about one run in five on CI.',
    path: `C:\\Users\\me\\.copilot\\session-state\\${nativeId}`,
    sourceDeletedAt: null,
    ...over,
  };
}

export const hit = (s: Session, snippet: string | null): SearchHit => ({ id: s.id, snippet });

export const entry = (role: TimelineEntry['role'], text: string, min = 5): TimelineEntry => ({
  role,
  text,
  timestamp: ago(min),
});
