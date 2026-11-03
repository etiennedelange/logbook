import { create } from 'zustand';
import { api } from '../api';
import { sessionsById, type Range } from '../model/select';
import type { Agent, IndexStatus, SearchHit, Session, SourceInfo, TimelineEntry, UpdateInfo } from '../types';
import { saveWhatsNew } from '../update';

export type Theme = 'dark' | 'light';
const THEME_KEY = 'logbook-theme';

function savedTheme(): Theme | null {
  try {
    const v = localStorage.getItem(THEME_KEY);
    return v === 'dark' || v === 'light' ? v : null;
  } catch {
    return null;
  }
}

function systemTheme(): Theme {
  try {
    return window.matchMedia('(prefers-color-scheme: light)').matches ? 'light' : 'dark';
  } catch {
    return 'dark';
  }
}

export function applyTheme(t: Theme): void {
  document.documentElement.dataset.theme = t;
}

/** Call once at startup; returns the theme to seed the store with. */
export function initTheme(): Theme {
  const t = savedTheme() ?? systemTheme();
  applyTheme(t);
  return t;
}

/** What the What's New dialog shows: an update on offer, or the version just installed. */
export interface WhatsNew {
  version: string;
  notes: string | null;
  mode: 'update' | 'installed';
}

const msg = (e: unknown): string => (typeof e === 'string' ? e : e instanceof Error ? e.message : String(e));

export interface Detail {
  id: string;
  transcript: TimelineEntry[] | null;
  error: string | null;
}

interface AppState {
  sessions: Session[];
  status: IndexStatus | null;
  loaded: boolean;
  error: string | null;
  /** Search box text. */
  query: string;
  /** Every match for `hitsQuery`, best first, or null while the box is empty. */
  hits: SearchHit[] | null;
  /** The query `hits` answers; it trails `query` while a search is in flight. */
  hitsQuery: string;
  searching: boolean;
  range: Range;
  repo: string | null;
  /** Only this agent's sessions, or every agent's. */
  agent: Agent | null;
  selId: string | null;
  detail: Detail | null;
  theme: Theme;
  /** A short-lived confirmation or error shown at the bottom of the window. */
  notice: { text: string; tone: 'ok' | 'error' } | null;
  /** The About panel is open. */
  about: boolean;
  /** Each agent's session folder, from the backend; null until it answers. */
  sources: SourceInfo[] | null;
  /** When the last index run finished (ms since the epoch), as far as this window saw. */
  readAt: number | null;
  /** The sidebar's Sources popover is open. */
  sourcesOpen: boolean;
  /** A newer release found on GitHub. */
  update: UpdateInfo | null;
  /** The update is downloading and installing; Logbook restarts when it's done. */
  updating: boolean;
  /** The What's New dialog is open with this. */
  whatsNew: WhatsNew | null;
  refresh: () => Promise<void>;
  reindex: () => Promise<void>;
  setQuery: (q: string) => void;
  setRange: (r: Range) => void;
  setRepo: (r: string | null) => void;
  setAgent: (a: Agent | null) => void;
  select: (id: string) => void;
  clearSelection: () => void;
  toggleTheme: () => void;
  setAbout: (open: boolean) => void;
  setSourcesOpen: (open: boolean) => void;
  /** Asks GitHub for a newer version. `manual` (from About) reports the outcome; otherwise failures stay quiet. */
  checkUpdate: (manual?: boolean) => Promise<UpdateInfo | null>;
  installUpdate: () => Promise<void>;
  setWhatsNew: (whatsNew: WhatsNew | null) => void;
  openReleasePage: (version: string) => Promise<void>;
  resume: (id: string) => Promise<void>;
  copyResume: (id: string) => Promise<void>;
  openFolder: (path: string) => Promise<void>;
  notify: (text: string, tone?: 'ok' | 'error') => void;
}

// Searching takes a few milliseconds, so it runs on every keystroke with no debounce. Only
// one request is in flight at a time: keystrokes made meanwhile collapse into `pendingQuery`,
// and a response is dropped unless it answers the latest query (`searchSeq`).
let searchSeq = 0;
let searchInFlight = false;
let pendingQuery: string | null = null;
let noticeTimer: ReturnType<typeof setTimeout> | undefined;
// The index generation `sessions` was fetched at. Not `status.generation`: other calls (reindex)
// store a newer status before the list is refetched, and comparing with that would skip the refetch.
let listedGeneration: number | null = null;

export const useApp = create<AppState>()((set, get) => ({
  sessions: [],
  status: null,
  loaded: false,
  error: null,
  query: '',
  hits: null,
  hitsQuery: '',
  searching: false,
  range: 'all',
  repo: null,
  agent: null,
  selId: null,
  detail: null,
  theme: 'dark',
  notice: null,
  about: false,
  sources: null,
  readAt: null,
  sourcesOpen: false,
  update: null,
  updating: false,
  whatsNew: null,

  refresh: async () => {
    // The folders can appear (an agent installed) or move between runs, so ask again each time.
    void Promise.resolve()
      .then(() => api.appInfo())
      .then(
        (info) => set({ sources: info.sources }),
        () => undefined,
      );
    try {
      const status = await api.indexStatus();
      if (!status.indexing) set({ readAt: Date.now() });
      // A re-scan that found nothing new leaves the index as it was: keep the list (and
      // everything worked out from it) rather than fetching and re-filtering it all.
      if (get().loaded && !get().error && status.generation === listedGeneration) {
        set({ status });
        return;
      }
      // Every session is loaded up front; the range and repo filters run in the UI.
      const sessions = await api.recentSessions();
      listedGeneration = status.generation;
      set({ sessions, status, loaded: true, error: null });
      // Re-run an active search so its hits reflect the new index.
      const { query } = get();
      if (query.trim()) get().setQuery(query);
    } catch (e) {
      set({ loaded: true, error: msg(e) });
    }
  },

  reindex: async () => {
    try {
      await api.reindex();
      set({ status: await api.indexStatus() });
    } catch (e) {
      get().notify(msg(e), 'error');
    }
  },

  setQuery: (q) => {
    searchSeq++;
    if (!q.trim()) {
      pendingQuery = null;
      set({ query: q, hits: null, hitsQuery: '', searching: false });
      return;
    }
    set({ query: q, searching: true });
    if (searchInFlight) {
      pendingQuery = q;
      return;
    }
    const run = async (q: string, seq: number) => {
      searchInFlight = true;
      try {
        const hits = await api.searchSessions(q);
        if (seq === searchSeq) set({ hits, hitsQuery: q, searching: false });
      } catch (e) {
        if (seq === searchSeq) set({ hits: [], hitsQuery: q, searching: false, error: msg(e) });
      } finally {
        searchInFlight = false;
        const next = pendingQuery;
        pendingQuery = null;
        if (next !== null) void run(next, searchSeq);
      }
    };
    void run(q, searchSeq);
  },

  setRange: (range) => set({ range }),
  setRepo: (repo) => set({ repo }),
  setAgent: (agent) => set({ agent }),

  select: (id) => {
    if (get().detail?.id === id) {
      set({ selId: id });
      return;
    }
    set({ selId: id, detail: { id, transcript: null, error: null } });
    api.transcript(id).then(
      (transcript) => {
        if (get().detail?.id === id) set({ detail: { id, transcript, error: null } });
      },
      (e) => {
        if (get().detail?.id === id) set({ detail: { id, transcript: null, error: msg(e) } });
      },
    );
  },

  clearSelection: () => set({ selId: null, detail: null }),

  toggleTheme: () => {
    const theme: Theme = get().theme === 'dark' ? 'light' : 'dark';
    applyTheme(theme);
    try {
      localStorage.setItem(THEME_KEY, theme);
    } catch {
      /* storage unavailable: the theme still applies for this session */
    }
    set({ theme });
  },

  setAbout: (about) => set({ about }),
  setSourcesOpen: (sourcesOpen) => set({ sourcesOpen }),

  checkUpdate: async (manual = false) => {
    try {
      const update = await api.checkUpdate();
      if (update) set({ update });
      if (manual) {
        if (update) set({ about: false, whatsNew: { ...update, mode: 'update' } });
        else get().notify('Logbook is up to date');
      }
      return update;
    } catch (e) {
      if (manual) get().notify(msg(e), 'error');
      else console.warn(e);
      return null;
    }
  },

  installUpdate: async () => {
    const { update, updating } = get();
    if (!update || updating) return;
    set({ updating: true });
    saveWhatsNew(update);
    try {
      // On Windows this doesn't return: the installer closes Logbook and starts the new version.
      await api.installUpdate();
    } catch (e) {
      set({ updating: false });
      get().notify(msg(e), 'error');
    }
  },

  setWhatsNew: (whatsNew) => set({ whatsNew }),

  openReleasePage: async (version) => {
    try {
      await api.openReleasePage(version);
    } catch (e) {
      get().notify(msg(e), 'error');
    }
  },

  resume: async (id) => {
    if (sessionsById(get().sessions).get(id)?.sourceDeletedAt) {
      get().notify('This session’s file was deleted, so it can’t be resumed. Copy still copies the command.', 'error');
      return;
    }
    try {
      await api.resumeSession(id);
      get().notify('Opened a terminal to resume this session');
    } catch (e) {
      get().notify(msg(e), 'error');
    }
  },

  copyResume: async (id) => {
    try {
      await navigator.clipboard.writeText(await api.resumeCommand(id));
      get().notify('Resume command copied');
    } catch (e) {
      get().notify(msg(e), 'error');
    }
  },

  openFolder: async (path) => {
    try {
      await api.openFolder(path);
    } catch (e) {
      get().notify(msg(e), 'error');
    }
  },

  notify: (text, tone = 'ok') => {
    clearTimeout(noticeTimer);
    set({ notice: { text, tone } });
    noticeTimer = setTimeout(() => set({ notice: null }), 3200);
  },
}));
