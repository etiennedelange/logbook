/** `logbook_core::models::Agent`. */
export type Agent = 'copilot' | 'claude';

/** Mirrors `logbook_core::dto::SessionDto` (camelCase, RFC3339 dates). */
export interface Session {
  /** `<agent>:<native id>`: what every command takes. */
  id: string;
  /** Which coding agent wrote the session (`logbook_core::models::Agent`). */
  agent: Agent;
  /** The agent's own id for the session, as shown to the user. */
  nativeId: string;
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
  /**
   * RFC 3339. When Logbook found the session's file deleted by its agent. The session is then
   * read from Logbook's kept copy and can't be resumed.
   */
  sourceDeletedAt: string | null;
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
  /** Each agent's session folder, which Logbook reads and never writes to. */
  sources: { agent: Agent; dir: string }[];
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
