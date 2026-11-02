import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { AppInfo, IndexStatus, SearchHit, Session, TimelineEntry, UpdateInfo } from './types';

export const api = {
  appInfo: () => invoke<AppInfo>('app_info'),
  indexStatus: () => invoke<IndexStatus>('index_status'),
  /** Starts an incremental re-scan in the background; `onIndexUpdated` fires when it ends. */
  reindex: () => invoke<void>('reindex_now'),
  recentSessions: (limit?: number) => invoke<Session[]>('recent_sessions', { limit }),
  /** Word-prefix AND search over names and transcripts, newest first. */
  searchSessions: (q: string, limit?: number) => invoke<SearchHit[]>('search_sessions', { q, limit }),
  transcript: (id: string) => invoke<TimelineEntry[]>('get_transcript', { id }),
  resumeCommand: (id: string) => invoke<string>('resume_command', { id }),
  /** Opens a terminal in the session's folder running `copilot --resume=<id>`. */
  resumeSession: (id: string) => invoke<void>('resume_session', { id }),
  openFolder: (path: string) => invoke<void>('open_folder', { path }),
  /** Opens an http(s) link from a transcript in the default browser. */
  openLink: (url: string) => invoke<void>('open_link', { url }),
  /** Asks GitHub Releases for a newer version; null when Logbook is up to date. */
  checkUpdate: () => invoke<UpdateInfo | null>('check_update'),
  /** Downloads and installs the update. On Windows the installer closes Logbook and starts the new version. */
  installUpdate: () => invoke<void>('install_update'),
  openReleasePage: (version: string) => invoke<void>('open_release_page', { version }),
};

/** Fires after every indexer run (at startup and after `reindex`). */
export const onIndexUpdated = (cb: () => void): Promise<UnlistenFn> => listen('index-updated', () => cb());
