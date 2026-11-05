/**
 * A throwaway home for the app under test: one Copilot CLI session and one Claude Code session, in the
 * folders COPILOT_HOME and CLAUDE_CONFIG_DIR point at, plus an empty cache (LOGBOOK_CACHE_DIR). Written
 * fresh each run, with timestamps relative to now, so the sessions are always in "Today".
 */
import { mkdirSync, mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

export const COPILOT = {
  id: '5b0e7c2a-41d7-4c58-9a1e-2f3d4c5b6a71',
  name: 'Fix the flaky overdue-loans test',
  branch: 'fix/overdue-flake',
  prompt: 'The overdue-loans test fails about one run in five. Should we quarantine it?',
  reply: 'No need to quarantine it: the fixture seeds loans with the wall clock. I will freeze it.',
};

export const CLAUDE = {
  id: 'c3a1f9d2-8e4b-4f6a-b7c5-1d2e3f4a5b62',
  branch: 'feat/e2e',
  prompt: 'Why does the tokenizer drop zebra-striped rows?',
  reply: 'The row filter treats alternate rows as headers. Fixed in `split_rows`.',
  /** The one file the session edited, under its folder. */
  file: 'zebra_rows.rs',
};

const iso = (msAgo) => new Date(Date.now() - msAgo).toISOString();
const HOUR = 3600_000;

export function makeHome() {
  const root = mkdtempSync(join(tmpdir(), 'logbook-e2e-'));
  const work = (name) => {
    const dir = join(root, 'work', name);
    mkdirSync(dir, { recursive: true });
    return dir;
  };

  const copilotHome = join(root, 'copilot');
  const session = join(copilotHome, 'session-state', COPILOT.id);
  mkdirSync(session, { recursive: true });
  writeFileSync(
    join(session, 'workspace.yaml'),
    [
      `id: ${COPILOT.id}`,
      `name: ${COPILOT.name}`,
      `cwd: ${work('bookshelf')}`,
      `branch: ${COPILOT.branch}`,
      `created_at: ${iso(2 * HOUR)}`,
      `updated_at: ${iso(HOUR)}`,
      '',
    ].join('\n'),
  );
  writeFileSync(
    join(session, 'events.jsonl'),
    [
      { type: 'session.start', timestamp: iso(2 * HOUR) },
      { type: 'user.message', timestamp: iso(2 * HOUR - 60_000), data: { content: COPILOT.prompt } },
      { type: 'assistant.message', timestamp: iso(HOUR), data: { content: COPILOT.reply } },
    ]
      .map((e) => JSON.stringify(e))
      .join('\n') + '\n',
  );

  const claudeHome = join(root, 'claude');
  const cwd = work('logbook');
  const project = join(claudeHome, 'projects', cwd.replace(/[^A-Za-z0-9]/g, '-'));
  mkdirSync(project, { recursive: true });
  const claudeFile = join(project, `${CLAUDE.id}.jsonl`);
  const line = (type, msAgo, message) =>
    JSON.stringify({ type, isSidechain: false, message, timestamp: iso(msAgo), cwd, gitBranch: CLAUDE.branch, sessionId: CLAUDE.id });
  writeFileSync(
    claudeFile,
    [
      line('user', 3 * HOUR, { role: 'user', content: CLAUDE.prompt }),
      line('assistant', 3 * HOUR - 30_000, { id: 'msg_1', role: 'assistant', content: [{ type: 'text', text: CLAUDE.reply }] }),
      // An Edit and its result, as Claude Code records a successful one.
      line('assistant', 3 * HOUR - 20_000, {
        id: 'msg_2',
        role: 'assistant',
        content: [{ type: 'tool_use', id: 'e1', name: 'Edit', input: { file_path: join(cwd, 'src', CLAUDE.file) } }],
      }),
      JSON.stringify({
        type: 'user',
        isSidechain: false,
        message: { role: 'user', content: [{ type: 'tool_result', tool_use_id: 'e1', content: 'ok' }] },
        toolUseResult: {
          filePath: join(cwd, 'src', CLAUDE.file),
          structuredPatch: [{ oldStart: 7, oldLines: 1, newStart: 7, newLines: 1, lines: ['-    i % 2 == 0', '+    i == 0'] }],
        },
        timestamp: iso(3 * HOUR - 19_000),
        cwd,
        gitBranch: CLAUDE.branch,
        sessionId: CLAUDE.id,
      }),
    ].join('\n') + '\n',
  );

  const cache = join(root, 'cache');
  mkdirSync(cache, { recursive: true });

  return {
    root,
    claudeFile,
    env: { COPILOT_HOME: copilotHome, CLAUDE_CONFIG_DIR: claudeHome, LOGBOOK_CACHE_DIR: cache },
  };
}
