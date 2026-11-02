import { act, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vite-plus/test';

vi.mock('./api', () => ({
  api: {
    appInfo: vi.fn(),
    indexStatus: vi.fn(),
    reindex: vi.fn().mockResolvedValue(undefined),
    recentSessions: vi.fn(),
    searchSessions: vi.fn(),
    transcript: vi.fn(),
    checkUpdate: vi.fn(),
    installUpdate: vi.fn(),
    openReleasePage: vi.fn().mockResolvedValue(undefined),
    openLink: vi.fn().mockResolvedValue(undefined),
  },
  onIndexUpdated: vi.fn().mockResolvedValue(() => undefined),
}));

import { api } from './api';
import App from './App';
import { useApp } from './state/store';
import type { UpdateInfo } from './types';
import { saveWhatsNew, scheduleUpdateChecks, takeWhatsNew, UPDATE_CHECK_DELAY_MS, UPDATE_RECHECK_MS } from './update';

const next: UpdateInfo = {
  version: '0.2.0',
  date: '2026-11-02T09:00:00.000Z',
  notes: '### Added\n- **Updates** from GitHub.\n\n<img src=x onerror=alert(1)>',
};

beforeEach(() => {
  vi.clearAllMocks();
  localStorage.clear();
  useApp.setState(useApp.getInitialState(), true);
  vi.mocked(api.recentSessions).mockResolvedValue([]);
  vi.mocked(api.indexStatus).mockResolvedValue({ indexing: false, total: 0, scanned: 0, skipped: 0, generation: 1 });
  vi.mocked(api.appInfo).mockResolvedValue({
    version: '0.1.1',
    sources: [{ agent: 'copilot', dir: 'C:\\s' }],
    cacheFile: 'C:\\c\\index.bin',
  });
  vi.mocked(api.checkUpdate).mockResolvedValue(null);
});

afterEach(() => {
  vi.useRealTimers();
});

describe('scheduleUpdateChecks', () => {
  it('checks after a delay, then hourly, and stops once it finds one', async () => {
    vi.useFakeTimers();
    const check = vi.fn().mockResolvedValueOnce(null).mockResolvedValueOnce(next);
    scheduleUpdateChecks(check);
    await vi.advanceTimersByTimeAsync(UPDATE_CHECK_DELAY_MS - 1);
    expect(check).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(1);
    expect(check).toHaveBeenCalledTimes(1);
    await vi.advanceTimersByTimeAsync(UPDATE_RECHECK_MS);
    expect(check).toHaveBeenCalledTimes(2);
    await vi.advanceTimersByTimeAsync(UPDATE_RECHECK_MS * 3);
    expect(check).toHaveBeenCalledTimes(2);
  });

  it('stops when told to', async () => {
    vi.useFakeTimers();
    const check = vi.fn().mockResolvedValue(null);
    const stop = scheduleUpdateChecks(check);
    stop();
    await vi.advanceTimersByTimeAsync(UPDATE_RECHECK_MS * 2);
    expect(check).not.toHaveBeenCalled();
  });
});

describe('saved notes', () => {
  it('come back once, for the version they were saved for', () => {
    saveWhatsNew(next);
    expect(takeWhatsNew('0.2.0')).toEqual({ version: '0.2.0', notes: next.notes });
    expect(takeWhatsNew('0.2.0')).toBeNull();
  });

  it('are dropped when the update did not go through', () => {
    saveWhatsNew(next);
    expect(takeWhatsNew('0.1.1')).toBeNull();
    expect(takeWhatsNew('0.2.0')).toBeNull();
  });
});

describe('update UI', () => {
  it('shows nothing while Logbook is up to date', async () => {
    render(<App />);
    await act(() => useApp.getState().checkUpdate());
    expect(screen.queryByRole('button', { name: /update to/i })).not.toBeInTheDocument();
  });

  it('offers an update in the toolbar and shows its notes in What’s New', async () => {
    vi.mocked(api.checkUpdate).mockResolvedValue(next);
    vi.mocked(api.installUpdate).mockReturnValue(new Promise(() => undefined));
    render(<App />);
    await act(() => useApp.getState().checkUpdate());

    fireEvent.click(screen.getByRole('button', { name: 'Update to 0.2.0' }));
    const dialog = screen.getByRole('dialog', { name: 'What’s New', hidden: true });
    expect(within(dialog).getByText('Logbook 0.2.0 is available')).toBeInTheDocument();
    expect(within(dialog).getByRole('heading', { name: 'Added', hidden: true })).toBeInTheDocument();
    expect(within(dialog).getByText('Updates')).toBeInTheDocument();
    // Raw HTML in the notes stays text.
    expect(dialog.querySelector('img')).toBeNull();

    fireEvent.click(within(dialog).getByRole('button', { name: /update now/i, hidden: true }));
    expect(api.installUpdate).toHaveBeenCalledTimes(1);
    expect(takeWhatsNew('0.2.0')).toEqual({ version: '0.2.0', notes: next.notes });
    expect(await within(dialog).findByRole('button', { name: /updating/i, hidden: true })).toBeDisabled();

    // It stays open until Logbook restarts: Close is off, and Esc or a backdrop click does nothing.
    expect(within(dialog).getByRole('button', { name: 'Close', hidden: true })).toBeDisabled();
    fireEvent.click(dialog);
    fireEvent(dialog, new Event('close'));
    expect(useApp.getState().whatsNew).not.toBeNull();
  });

  it('reports a failed install and lets you try again', async () => {
    vi.mocked(api.checkUpdate).mockResolvedValue(next);
    vi.mocked(api.installUpdate).mockRejectedValue('Couldn’t update Logbook: offline');
    render(<App />);
    await act(() => useApp.getState().checkUpdate());
    await act(() => useApp.getState().installUpdate());
    expect(useApp.getState().updating).toBe(false);
    expect(useApp.getState().notice).toEqual({ text: 'Couldn’t update Logbook: offline', tone: 'error' });
  });

  it('shows the installed version’s notes once after an update', async () => {
    vi.mocked(api.appInfo).mockResolvedValue({
      version: '0.2.0',
      sources: [{ agent: 'copilot', dir: 'C:\\s' }],
      cacheFile: 'C:\\c\\i.bin',
    });
    saveWhatsNew(next);
    render(<App />);
    const dialog = await screen.findByRole('dialog', { name: 'What’s New', hidden: true });
    expect(await within(dialog).findByText('You’re now on Logbook 0.2.0')).toBeInTheDocument();
    expect(within(dialog).queryByRole('button', { name: /update now/i, hidden: true })).not.toBeInTheDocument();
    expect(localStorage.getItem('logbook-whats-new')).toBeNull();
  });

  it('says so when a manual check finds nothing, and shows a failure', async () => {
    await act(() => useApp.getState().checkUpdate(true));
    expect(useApp.getState().notice?.text).toBe('Logbook is up to date');
    vi.mocked(api.checkUpdate).mockRejectedValue('Couldn’t check for updates: offline');
    await act(() => useApp.getState().checkUpdate(true));
    expect(useApp.getState().notice).toEqual({ text: 'Couldn’t check for updates: offline', tone: 'error' });
  });

  it('keeps a failed background check quiet', async () => {
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => undefined);
    vi.mocked(api.checkUpdate).mockRejectedValue('offline');
    await act(() => useApp.getState().checkUpdate());
    expect(useApp.getState().notice).toBeNull();
    warn.mockRestore();
  });
});
