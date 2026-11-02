import { act, fireEvent, render, screen, within } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vite-plus/test';

vi.mock('./api', () => ({
  api: {
    appInfo: vi.fn(),
    indexStatus: vi.fn(),
    reindex: vi.fn().mockResolvedValue(undefined),
    recentSessions: vi.fn(),
    searchSessions: vi.fn(),
    transcript: vi.fn(),
    resumeCommand: vi.fn(),
    resumeSession: vi.fn().mockResolvedValue(undefined),
    openFolder: vi.fn().mockResolvedValue(undefined),
  },
  onIndexUpdated: vi.fn().mockResolvedValue(() => undefined),
}));

import { api } from './api';
import App from './App';
import { useApp } from './state/store';
import type { SearchHit } from './types';
import { entry, hit, session } from './test/fixtures';

const one = session({ id: 'one', name: 'Fix the flaky test', repository: 'org/app' });
const two = session({ id: 'two', name: 'Dashboard config', repository: 'org/dash' });

beforeEach(() => {
  vi.clearAllMocks();
  useApp.setState(useApp.getInitialState(), true);
  vi.mocked(api.recentSessions).mockResolvedValue([one, two]);
  vi.mocked(api.indexStatus).mockResolvedValue({ indexing: false, total: 2, scanned: 0, skipped: 2, generation: 1 });
  vi.mocked(api.appInfo).mockResolvedValue({
    version: '0.1.0',
    sessionsDir: 'C:\\Users\\me\\.copilot\\session-state',
    cacheFile: 'C:\\Users\\me\\AppData\\Local\\Logbook\\index-v4.bin',
  });
  vi.mocked(api.transcript).mockResolvedValue([entry('user', 'why is it flaky?'), entry('assistant', 'A clock race.')]);
});

describe('App', () => {
  it('lists sessions and their repositories', async () => {
    render(<App />);
    const list = await screen.findByRole('listbox', { name: 'Sessions' });
    await within(list).findAllByRole('option');
    expect(within(list).getAllByRole('option')).toHaveLength(2);
    expect(screen.getByRole('button', { name: /^dash/ })).toHaveAttribute('title', 'org/dash');
    expect(screen.getByText('2 sessions indexed')).toBeInTheDocument();
  });

  it('only refetches sessions when a re-scan changed the index', async () => {
    render(<App />);
    await screen.findAllByRole('option');
    expect(api.recentSessions).toHaveBeenCalledTimes(1);
    await act(() => useApp.getState().refresh());
    expect(api.recentSessions).toHaveBeenCalledTimes(1);
    const status = { indexing: false, total: 3, scanned: 1, skipped: 2, generation: 2 };
    vi.mocked(api.indexStatus).mockResolvedValue(status);
    vi.mocked(api.recentSessions).mockResolvedValue([one, two, session({ id: 'three', name: 'New one' })]);
    await act(() => useApp.getState().refresh());
    expect(api.recentSessions).toHaveBeenCalledTimes(2);
    expect(await screen.findByText('New one')).toBeInTheDocument();
  });

  it('shows the transcript of the selected session and resumes it', async () => {
    render(<App />);
    fireEvent.click(await screen.findByText('Fix the flaky test'));
    expect(await screen.findByText('A clock race.')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Resume' }));
    expect(api.resumeSession).toHaveBeenCalledWith('one');
  });

  it('filters the list by repository', async () => {
    render(<App />);
    fireEvent.click(await screen.findByRole('button', { name: /^dash/ }));
    const list = screen.getByRole('listbox', { name: 'Sessions' });
    expect(within(list).getAllByRole('option')).toHaveLength(1);
    expect(within(list).getByText('Dashboard config')).toBeInTheDocument();
  });

  it('searches transcripts as you type and highlights the match', async () => {
    vi.mocked(api.searchSessions).mockResolvedValue([hit(two, 'build the \u0002dashboard\u0003 now')]);
    render(<App />);
    fireEvent.change(await screen.findByRole('searchbox', { name: 'Search sessions' }), { target: { value: 'dash' } });
    expect(api.searchSessions).toHaveBeenCalledWith('dash');
    expect((await screen.findByText('dashboard')).tagName).toBe('MARK');
    expect(within(screen.getByRole('listbox', { name: 'Sessions' })).getAllByRole('option')).toHaveLength(1);
    expect(screen.getByText('1 match for “dash”')).toBeInTheDocument();
  });

  it('only ever shows the results of the latest query', async () => {
    const answers: Record<string, (hits: SearchHit[]) => void> = {};
    vi.mocked(api.searchSessions).mockImplementation((q) => new Promise((resolve) => (answers[q] = resolve)));
    render(<App />);
    const box = await screen.findByRole('searchbox', { name: 'Search sessions' });
    fireEvent.change(box, { target: { value: 'f' } });
    fireEvent.change(box, { target: { value: 'fl' } });
    fireEvent.change(box, { target: { value: 'fla' } });
    // One request in flight; the keystrokes made meanwhile collapse into a single follow-up.
    expect(api.searchSessions).toHaveBeenCalledTimes(1);
    await act(async () => answers['f']!([hit(one, null), hit(two, null)]));
    expect(api.searchSessions).toHaveBeenCalledTimes(2);
    expect(api.searchSessions).toHaveBeenLastCalledWith('fla');
    // The stale answer for "f" never reached the screen.
    expect(screen.queryByText(/for “f”/)).not.toBeInTheDocument();
    await act(async () => answers['fla']!([hit(one, 'a \u0002flaky\u0003 test')]));
    expect(screen.getByText('1 match for “fla”')).toBeInTheDocument();
    expect(within(screen.getByRole('listbox', { name: 'Sessions' })).getAllByRole('option')).toHaveLength(1);
  });

  it('applies the range and repository filters to every search hit, not just the first few hundred', async () => {
    const many = Array.from({ length: 300 }, (_, i) =>
      session({ id: `s${i}`, name: `Session ${i}`, repository: i === 299 ? 'org/rare' : 'org/app' }),
    );
    vi.mocked(api.recentSessions).mockResolvedValue(many);
    vi.mocked(api.searchSessions).mockResolvedValue(many.map((s) => hit(s, null)));
    render(<App />);
    fireEvent.click(await screen.findByRole('button', { name: /^rare/ }));
    fireEvent.change(screen.getByRole('searchbox', { name: 'Search sessions' }), { target: { value: 'session' } });
    expect(await screen.findByText('1 match for “session”, all sessions')).toBeInTheDocument();
    expect(screen.getByText('Session 299')).toBeInTheDocument();
    expect(api.searchSessions).toHaveBeenCalledWith('session');
  });

  it('explains what Logbook is from the ? button or key, and where it reads from', async () => {
    render(<App />);
    await screen.findAllByRole('option');
    fireEvent.click(screen.getByRole('button', { name: 'About Logbook' }));
    const about = await screen.findByRole('dialog', { name: 'About Logbook' });
    expect(await within(about).findByText('Version 0.1.0')).toBeInTheDocument();
    expect(within(about).getByText('C:\\Users\\me\\.copilot\\session-state')).toBeInTheDocument();
    fireEvent.click(within(about).getByRole('button', { name: 'Open the index folder' }));
    expect(api.openFolder).toHaveBeenCalledWith('C:\\Users\\me\\AppData\\Local\\Logbook');

    // While it's open, the app's own shortcuts stay quiet.
    fireEvent.keyDown(window, { key: '2' });
    expect(useApp.getState().range).toBe('all');
    fireEvent.click(within(about).getByRole('button', { name: 'Close' }));
    expect(useApp.getState().about).toBe(false);

    fireEvent.keyDown(window, { key: '?' });
    expect(useApp.getState().about).toBe(true);
  });
});
