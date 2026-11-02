import type { UpdateInfo } from './types';

/** The startup check waits this long, so it never competes with loading the sessions. */
export const UPDATE_CHECK_DELAY_MS = 5000;

/**
 * Logbook is often left open for days, so it checks again on this interval until it finds an update.
 * A single startup check would never see a release published after it.
 */
export const UPDATE_RECHECK_MS = 60 * 60 * 1000;

/**
 * Checks after a short delay, then every hour, until `check` reports an update. Returns a stop function.
 * `check` handles its own errors.
 */
export function scheduleUpdateChecks(check: () => Promise<UpdateInfo | null>): () => void {
  let stopped = false;
  let timer: ReturnType<typeof setTimeout>;
  const run = async () => {
    const found = await check();
    if (!found && !stopped) timer = setTimeout(() => void run(), UPDATE_RECHECK_MS);
  };
  timer = setTimeout(() => void run(), UPDATE_CHECK_DELAY_MS);
  return () => {
    stopped = true;
    clearTimeout(timer);
  };
}

/**
 * The notes of the version being installed, kept across the update so the new version can show them
 * once on its first start. WebView2's storage survives the installer: the app identifier is unchanged.
 */
const WHATS_NEW_KEY = 'logbook-whats-new';

export interface SavedNotes {
  version: string;
  notes: string | null;
}

export function saveWhatsNew(update: UpdateInfo): void {
  try {
    localStorage.setItem(WHATS_NEW_KEY, JSON.stringify({ version: update.version, notes: update.notes }));
  } catch {
    /* storage unavailable: the update still installs, it just won't say what's new afterwards */
  }
}

/**
 * The saved notes if they're for `running` (the update was installed), and forgets them either way, so
 * they show once. Notes for another version mean the update didn't go through; they're dropped too.
 */
export function takeWhatsNew(running: string): SavedNotes | null {
  try {
    const raw = localStorage.getItem(WHATS_NEW_KEY);
    if (raw === null) return null;
    localStorage.removeItem(WHATS_NEW_KEY);
    const saved = JSON.parse(raw) as Partial<SavedNotes>;
    if (saved.version !== running) return null;
    return { version: running, notes: typeof saved.notes === 'string' ? saved.notes : null };
  } catch {
    return null;
  }
}
