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
  /** How many files the session created, and how many it edited (see `SessionFiles`). */
  filesCreated: number;
  filesEdited: number;
}

/** A file a session created or edited (`dto::FileChangeDto`), summed over its edits. */
export interface FileChange {
  /** Relative to the session's folder when inside it. */
  path: string;
  created: boolean;
  edits: number;
  added: number;
  removed: number;
}

/** The detail pane's Files tab (`dto::SessionFilesDto`). */
export interface SessionFiles {
  files: FileChange[];
  /** Shell commands that look like they change files; Logbook can't tell which. */
  commands: string[];
}

/** A run of changed lines, each starting with ` `, `+` or `-`. */
export interface Hunk {
  oldStart: number;
  newStart: number;
  lines: string[];
}

/** One edit to a file (`dto::FileEditDto`), read from the agent's own file. */
export interface FileEdit {
  timestamp: string | null;
  created: boolean;
  hunks: Hunk[];
  /** Lines were left out of a very large edit. */
  truncated: boolean;
}

/**
 * A search hit (`logbook_core::dto::SearchHitDto`): the session's id, joined with the loaded
 * sessions in the UI, and a snippet with matched words wrapped in STX (\u0002) / ETX (\u0003).
 */
export interface SearchHit {
  id: string;
  snippet: string | null;
  /** What the snippet shows: the transcript, the path of `file`, or the session's id. */
  matched: 'text' | 'file' | 'session';
  file: FileChange | null;
}

export interface TimelineEntry {
  role: 'user' | 'assistant';
  timestamp: string | null;
  text: string;
}

/** An agent's session folder (`commands::SourceInfo`), which Logbook reads and never writes to. */
export interface SourceInfo {
  agent: Agent;
  dir: string;
  /** `dir` with the home folder written as `~`, for showing. */
  short: string;
  /** The folder exists: the agent is installed and has run. */
  found: boolean;
}

/** What About and the sidebar's Sources show (`commands::AppInfo`). */
export interface AppInfo {
  version: string;
  sources: SourceInfo[];
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
