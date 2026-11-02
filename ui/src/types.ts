/** Mirrors `logbook_core::dto::SessionDto` (camelCase, RFC3339 dates). */
export interface Session {
  id: string;
  name: string | null;
  userNamed: boolean;
  cwd: string | null;
  gitRoot: string | null;
  repository: string | null;
  branch: string | null;
  clientName: string | null;
  createdAt: string | null;
  updatedAt: string | null;
  messageCount: number;
  /** First user message, whitespace-collapsed and cut to 200 characters. */
  firstPrompt: string | null;
  path: string;
}

/**
 * A search hit (`logbook_core::dto::SearchHitDto`): the session's id, joined with the loaded
 * sessions in the UI, and a snippet with matched words wrapped in STX (\u0002) / ETX (\u0003).
 */
export interface SearchHit {
  id: string;
  snippet: string | null;
}

export interface TimelineEntry {
  role: 'user' | 'assistant';
  timestamp: string | null;
  text: string;
}

/** What the About panel shows (`commands::AppInfo`). */
export interface AppInfo {
  version: string;
  /** The Copilot CLI's session folders, which Logbook only reads. */
  sessionsDir: string;
  /** Logbook's own index cache. */
  cacheFile: string;
}

export interface IndexStatus {
  indexing: boolean;
  total: number;
  scanned: number;
  skipped: number;
  /** Changes whenever the index does. */
  generation: number;
}

/** A newer release found on GitHub (`update::UpdateInfo`). */
export interface UpdateInfo {
  version: string;
  /** The release's CHANGELOG section, as Markdown. */
  notes: string | null;
  /** RFC 3339. */
  date: string | null;
}
