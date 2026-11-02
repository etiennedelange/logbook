import { describe, expect, it } from 'vite-plus/test';
import { session } from '../test/fixtures';
import {
  basename,
  dayGroup,
  displayName,
  highlightWords,
  matchMode,
  queryTokens,
  relTime,
  repoOf,
  sessionTitle,
  shortRepo,
  snippetSegments,
} from './format';

describe('snippetSegments', () => {
  it('splits on STX/ETX markers', () => {
    expect(snippetSegments('... build the \u0002dashboard\u0003 env \u0002config\u0003')).toEqual([
      { text: '... build the ', hit: false },
      { text: 'dashboard', hit: true },
      { text: ' env ', hit: false },
      { text: 'config', hit: true },
    ]);
  });

  it('keeps text without markers whole', () => {
    expect(snippetSegments('plain')).toEqual([{ text: 'plain', hit: false }]);
  });
});

describe('relTime', () => {
  const now = new Date('2026-10-13T12:00:00Z');
  it('is short within a day', () => {
    expect(relTime('2026-10-13T11:59:30Z', now)).toBe('now');
    expect(relTime('2026-10-13T11:15:00Z', now)).toBe('45m');
    expect(relTime('2026-10-13T02:00:00Z', now)).toBe('10h');
  });
  it('shows a date once older than a week', () => {
    expect(relTime('2026-09-01T12:00:00Z', now)).toBe('Sep 1');
    expect(relTime('2025-09-01T12:00:00Z', now)).toBe('Sep 1, 2025');
  });
  it('is blank without a date', () => {
    expect(relTime(null, now)).toBe('');
  });
});

describe('session names', () => {
  it('falls back to the first prompt, then "Untitled session"', () => {
    expect(sessionTitle(session({ name: 'Named' }))).toEqual({ text: 'Named', source: 'name' });
    expect(sessionTitle(session({ name: '  ', firstPrompt: 'Why is CI red?' }))).toEqual({
      text: 'Why is CI red?',
      source: 'prompt',
    });
    expect(displayName(session({ name: null, firstPrompt: null }))).toBe('Untitled session');
  });
  it('groups by repository, else the folder name', () => {
    expect(repoOf(session({ repository: 'org/app' }))).toBe('org/app');
    expect(repoOf(session({ repository: null, gitRoot: null, cwd: 'C:\\Repos\\thing\\' }))).toBe('thing');
    expect(repoOf(session({ repository: null, gitRoot: null, cwd: null }))).toBeNull();
    expect(basename('/home/me/proj')).toBe('proj');
  });
});

describe('highlightWords', () => {
  it('marks words that start with a search token', () => {
    const segs = highlightWords('Build the dashboard, then dash off', queryTokens('DASH'));
    expect(segs.filter((s) => s.hit).map((s) => s.text)).toEqual(['dashboard', 'dash']);
    expect(segs.map((s) => s.text).join('')).toBe('Build the dashboard, then dash off');
  });
  it('leaves text alone without tokens', () => {
    expect(highlightWords('plain text', [])).toEqual([{ text: 'plain text', hit: false }]);
  });
});

describe('phrase search', () => {
  const tokens = queryTokens('What gets sched');
  const text = 'So what now? **What gets scheduled:** only a key. It gets what it needs.';

  it('prefers the phrase, then whole words, then prefixes', () => {
    expect(matchMode(text, tokens)).toBe('phrase');
    expect(matchMode('what is scheduled, it gets done', queryTokens('what gets scheduled'))).toBe('words');
    expect(matchMode('what getSpecification scheduled', queryTokens('what gets scheduled'))).toBe('prefix');
    expect(matchMode('nothing here', tokens)).toBeNull();
  });

  it('marks only the phrase, separators included, in phrase mode', () => {
    const hits = highlightWords(text, tokens, 'phrase').filter((s) => s.hit);
    expect(hits.map((s) => s.text)).toEqual(['What gets scheduled']);
  });

  it('cleans markdown out of snippets and joins a marked phrase', () => {
    expect(snippetSegments('... **\u0002What\u0003 \u0002gets\u0003 \u0002scheduled\u0003:** only `a` key')).toEqual([
      { text: '... ', hit: false },
      { text: 'What gets scheduled', hit: true },
      { text: ': only a key', hit: false },
    ]);
  });
});

describe('dayGroup', () => {
  const now = new Date(2026, 9, 13, 12, 0);
  it('names recent days and falls back to the month', () => {
    expect(dayGroup(new Date(2026, 9, 13, 1).toISOString(), now)).toBe('Today');
    expect(dayGroup(new Date(2026, 9, 12, 23).toISOString(), now)).toBe('Yesterday');
    expect(dayGroup(new Date(2026, 9, 9).toISOString(), now)).toBe('Friday');
    expect(dayGroup(new Date(2026, 7, 2).toISOString(), now)).toBe('August 2026');
  });
});

it('shortRepo keeps the last path segment', () => {
  expect(shortRepo('Org/Project/Repo.Name')).toBe('Repo.Name');
  expect(shortRepo('solo')).toBe('solo');
});

it('strips markdown from a prompt used as a title', () => {
  expect(sessionTitle(session({ name: null, firstPrompt: 'What does `git rebase` do?' })).text).toBe(
    'What does git rebase do?',
  );
});
