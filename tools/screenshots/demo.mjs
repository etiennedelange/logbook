/**
 * A made-up history of Copilot CLI and Claude Code sessions, written as the agents' own files, so the real app
 * can be photographed without any private session data (tools/screenshots/capture.mjs).
 *
 * The folders are named like the Windows ones the screenshots show (`D:\Repos\Other\logbook`), as plain
 * directories under the demo home: the app is started inside it, so those relative paths resolve, and the
 * fake `.git/config` in each gives its sessions the repository the sidebar groups by.
 *
 * Every time is relative to when this runs, as the old mock's were, so a shot always says "4m" and "Today".
 */
import { mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const MIN = 1;
const H = 60 * MIN;
const D = 24 * H;

export const FOLDERS = {
  logbook: ['etienne/logbook', 'D:\\Repos\\Other\\logbook'],
  reading: ['me/reading-list', 'C:\\Repos\\reading-list'],
  site: ['etienne/site', 'D:\\Repos\\Other\\site'],
  hooks: ['etienne/ado-hooks', 'D:\\Repos\\Other\\ado-hooks'],
  scratch: [null, 'C:\\Users\\me\\scratch'],
};

/** The Claude session the Files scenes open, and the one whose file the capture tool deletes (a kept copy). */
export const FILES_SESSION = 'Split the indexer into sources';
export const DELETED_SESSION = 'Postcard schema bump to v4';
export const DELETED_FILE_ID = '4f5a6b7c-8d9e-4f0a-a1b2-c3d4e5f6a794';
export const EDITED_FILE = 'core\\src\\indexer.rs';

const iso = (minsAgo) => new Date(Date.now() - minsAgo * 60_000).toISOString();

function write(path, text) {
  mkdirSync(join(path, '..'), { recursive: true });
  writeFileSync(path, text);
}

/** A directory named with backslashes (a Linux file name), holding a git config with this origin. */
function folder(root, [repository, cwd]) {
  if (repository) {
    write(join(root, cwd, '.git', 'config'), `[remote "origin"]\n\turl = https://github.com/${repository}.git\n`);
  } else {
    mkdirSync(join(root, cwd), { recursive: true });
  }
  return cwd;
}

const FILLER = ['Thanks, that works.', 'Anything else worth checking?', 'Looks good. Run the tests once more.', 'All green.'];

/** `messages` are the ones the screenshots show; the rest pad the count to `total` and come after them. */
function pad(messages, total, updatedAgo) {
  const out = [...messages];
  for (let i = out.length; i < total; i++) {
    out.push([i % 2 === 0 ? 'user' : 'assistant', FILLER[i % FILLER.length], Math.max(0, updatedAgo + (total - i) * 0.001)]);
  }
  return out;
}

function copilot(root, { id, name, folderKey, branch, updated, created, count, messages }) {
  const [repository, cwd] = FOLDERS[folderKey];
  folder(root, FOLDERS[folderKey]);
  const dir = join(root, 'copilot', 'session-state', id);
  const yaml = [`id: ${id}`, name ? `name: ${name}` : null, repository ? `repository: ${repository}` : null, `cwd: ${cwd}`, branch ? `branch: ${branch}` : null, `created_at: ${iso(created)}`, `updated_at: ${iso(updated)}`]
    .filter(Boolean)
    .join('\n');
  write(join(dir, 'workspace.yaml'), `${yaml}\n`);
  const events = [{ type: 'session.start', timestamp: iso(created) }];
  for (const [role, text, ago] of pad(messages, count, updated)) {
    events.push({ type: role === 'user' ? 'user.message' : 'assistant.message', timestamp: iso(ago), data: { content: text } });
  }
  write(join(dir, 'events.jsonl'), `${events.map((e) => JSON.stringify(e)).join('\n')}\n`);
}

function claude(root, { id, name, folderKey, branch, updated, created, count, messages, edits = [] }) {
  const [, cwd] = FOLDERS[folderKey];
  folder(root, FOLDERS[folderKey]);
  const project = join(root, 'claude', 'projects', cwd.replace(/[^A-Za-z0-9]/g, '-'));
  const base = { isSidechain: false, cwd, gitBranch: branch, sessionId: id };
  const lines = [{ type: 'custom-title', customTitle: name }];
  for (const [role, text, ago] of pad(messages, count, updated)) {
    lines.push({
      ...base,
      type: role,
      timestamp: iso(ago),
      message: role === 'user' ? { role, content: text } : { id: `m${lines.length}`, role, content: [{ type: 'text', text }] },
    });
  }
  edits.forEach((edit, i) => {
    const toolId = `e${i}`;
    lines.push({ ...base, type: 'assistant', timestamp: iso(edit.ago), message: { id: `t${i}`, role: 'assistant', content: [{ type: 'tool_use', id: toolId, name: edit.created ? 'Write' : 'Edit', input: { file_path: edit.path } }] } });
    lines.push({
      ...base,
      type: 'user',
      timestamp: iso(edit.ago - 0.01),
      message: { role: 'user', content: [{ type: 'tool_result', tool_use_id: toolId, content: 'ok' }] },
      toolUseResult: edit.created
        ? { type: 'create', filePath: edit.path, content: edit.content, structuredPatch: [] }
        : { type: 'update', filePath: edit.path, structuredPatch: edit.hunks },
    });
  });
  // The compact form: the reader skips lines that don't contain these exact patterns.
  write(join(project, `${id}.jsonl`), `${lines.map((l) => JSON.stringify(l)).join('\n')}\n`);
}

/** A hunk of `n` added lines of Rust-ish code. */
const rust = (n, what) => Array.from({ length: n }, (_, i) => (i === 0 ? `//! ${what}` : i % 6 === 0 ? '' : `pub fn step_${i}() {}`)).join('\n');
const win = (p) => `D:\\Repos\\Other\\logbook\\${p}`;

export function writeDemoHome(root, { history = false } = {}) {
  rmSync(root, { recursive: true, force: true });
  mkdirSync(join(root, 'cache'), { recursive: true });

  copilot(root, {
    id: '3f1c9a2e-7b44-4d0e-9a51-1c2f0e8b7d10',
    folderKey: 'reading',
    branch: 'fix/overdue-flake',
    updated: 4,
    created: 70,
    count: 14,
    messages: [
      ['user', 'The overdue-loans test fails about one run in five on CI. Can you find out why?', 70],
      ['assistant', 'The test seeds loans with `Utc::now()` and then asserts on a due date computed a few milliseconds later, so loans created right at the boundary land on either side of it.\n\nI\u2019ll freeze the clock in the fixture and compute the due date from the same instant.', 68],
      ['user', 'Do it, and add a regression test that pins the boundary.', 40],
      ['assistant', 'Done. `due_date_is_inclusive` now covers a loan that falls due exactly now:\n\n```rust\n#[test]\nfn due_date_is_inclusive() {\n    let now = fixed_clock("2026-10-13T12:00:00Z");\n    let loan = loan_at(now - LOAN_PERIOD);\n    assert!(is_overdue(&loan, now));\n}\n```\n\n200 local runs, no failures.', 4],
    ],
  });
  copilot(root, {
    id: '8d2e4b61-1f0a-4c3b-b2d7-5e9a6c4f3a21',
    name: 'Dashboard env config for staging',
    folderKey: 'reading',
    branch: 'main',
    updated: 3 * H,
    created: 5 * H,
    count: 22,
    messages: [
      ['user', 'please build the env config for staging, same shape as prod', 5 * H],
      ['assistant', 'Sure, working on it now. I\u2019ll mirror `env/prod.json` into `env/staging.json` and point it at the staging API host.', 5 * H - 2],
      ['user', 'How does the reminder queue fit in?', 4 * H],
      [
        'assistant',
        [
          'Your picture is mostly right. One correction: the reminder queue doesn\u2019t poll every five minutes.',
          '',
          '**What gets scheduled:** only a key, such as a loan ID, and a due time.',
          '',
          '**Normal flow**',
          '1. A renewal calls `ScheduleAsync("loan-123", due + 1 day)`.',
          '2. Another renewal replaces the due time rather than adding a second entry.',
          '',
          '### When a reminder falls due',
          '- read the loan\u2019s due date from its own store;',
          '- if it\u2019s really overdue, send the email;',
          '  - and schedule the next reminder;',
          '- if the book came back in between, do nothing.',
          '',
          '| Reminder | After |',
          '|----------|-------|',
          '| first    | 1 day |',
          '| second   | 7 days |',
          '',
          '> The handler must be safe to run twice for the same key.',
          '',
          'See the [queue docs](https://example.com/queue) for retries.',
        ].join('\n'),
        4 * H - 1,
      ],
    ],
  });
  copilot(root, {
    id: 'b7a0c3e9-22d4-4e18-8f6b-0d4c1a9e5b32',
    folderKey: 'scratch',
    updated: 1 * D + 2 * H,
    created: 1 * D + 3 * H,
    count: 4,
    messages: [
      ['user', 'What does `git rebase --update-refs` do?', D + 3 * H],
      ['assistant', 'It moves any branches that point at commits being rebased along with them, so a stack of branches stays stacked.', D + 3 * H - 1],
    ],
  });
  copilot(root, {
    id: 'c41e7d02-9b3a-4f65-a1c8-7e2d5b0f6c43',
    name: 'Migrate session search to Tauri',
    folderKey: 'logbook',
    branch: 'tauri',
    updated: 4 * D,
    created: 6 * D,
    count: 57,
    messages: [
      ['user', 'Move the axum server behind Tauri commands and drop Docker.', 6 * D],
      ['assistant', 'The core crate stays as is; commands wrap `SessionService`.', 6 * D - 3],
    ],
  });
  copilot(root, {
    id: 'e5f9a1b3-4c6d-4e8f-9a0b-1c2d3e4f5a54',
    name: 'Jira webhook signature check',
    folderKey: 'hooks',
    branch: 'feat/jira',
    updated: 19 * D,
    created: 19 * D + 2 * H,
    count: 31,
    messages: [['user', 'Verify the webhook HMAC before parsing the body.', 19 * D + 2 * H]],
  });

  claude(root, {
    id: '9b2f6e1a-3c4d-4a7b-8e5f-1d2c3b4a5e61',
    name: FILES_SESSION,
    folderKey: 'logbook',
    branch: 'feat/sources',
    updated: 2 * H,
    created: 4 * H,
    count: 38,
    messages: [
      ['user', 'Put Copilot behind a Source trait so a second agent is one new module.', 4 * H],
      ['assistant', 'The indexer and service now only see `Source`: `discover` lists sessions with a fingerprint, `read` parses one, and `resume` returns a program, its arguments and a folder.', 4 * H - 6],
      ['user', 'Does the session count still match 0.1.1?', 2 * H + 10],
      ['assistant', 'Yes: 505 listed on both, and search output is identical.', 2 * H],
    ],
    edits: [
      { path: win('core\\src\\source\\mod.rs'), created: true, ago: 4 * H - 3, content: rust(124, 'Where sessions come from: one `Source` per coding agent.') },
      { path: win('core\\src\\source\\copilot.rs'), created: true, ago: 4 * H - 4, content: rust(64, 'Copilot CLI sessions.') },
      {
        path: win('core\\src\\indexer.rs'),
        ago: 4 * H - 4,
        hunks: [
          {
            oldStart: 41,
            oldLines: 3,
            newStart: 41,
            newLines: 6,
            lines: [' pub fn run(index: &mut SessionIndex) -> Result<IndexStats> {', '-    let root = paths::session_state_root();', '-    let found = discover_sessions(&root)?;', '+    let mut found = Vec::new();', '+    for source in source::all() {', '+        found.extend(source.discover()?);', '+    }', '     let mut stats = IndexStats::default();'],
          },
        ],
      },
      {
        path: win('core\\src\\indexer.rs'),
        ago: 4 * H - 5,
        hunks: [
          { oldStart: 88, oldLines: 4, newStart: 90, newLines: 1, lines: ['-fn discover_sessions(root: &Path) -> Result<Vec<SessionDir>> {', '-    // moved to source/copilot.rs', '-}', '+// Discovery lives in each source now (see source/mod.rs).'] },
        ],
      },
      { path: win('core\\src\\indexer.rs'), ago: 2 * H + 8, hunks: [{ oldStart: 3, oldLines: 1, newStart: 3, newLines: 2, lines: [' use crate::index::SessionIndex;', '+use crate::source;'] }] },
      { path: win('core\\src\\models.rs'), ago: 3 * H, hunks: [{ oldStart: 10, oldLines: 1, newStart: 10, newLines: 3, lines: ['-pub struct SessionRecord {', '+pub struct SessionRecord {', '+    pub agent: Agent,', '+    pub native_id: String,'] }] },
      { path: win('core\\src\\paths.rs'), ago: 3 * H - 1, hunks: [{ oldStart: 5, oldLines: 1, newStart: 5, newLines: 1, lines: ['-pub fn root() -> PathBuf {', '+pub fn copilot_root() -> PathBuf {'] }] },
    ],
  });
  claude(root, {
    id: '2e8a4c6b-7d1f-4b3a-9c5e-6f0a1b2c3d72',
    name: 'Why is the release job skipping?',
    folderKey: 'logbook',
    branch: 'main',
    updated: D + 5 * H,
    created: D + 6 * H,
    count: 9,
    messages: [
      ['user', 'The release workflow skips the build job on tags. Why?', D + 6 * H],
      ['assistant', "Its `if:` checks `github.ref == 'refs/heads/main'`, which is never true for a tag push. Use `startsWith(github.ref, 'refs/tags/v')` there.", D + 6 * H - 2],
    ],
  });
  claude(root, {
    id: '7c3d9e0f-1a2b-4c5d-8e6f-0a1b2c3d4e83',
    name: 'Cache headers for /docs',
    folderKey: 'site',
    branch: 'main',
    updated: 9 * D,
    created: 9 * D + H,
    count: 17,
    messages: [
      ['user', 'Docs pages are cached for a day after a deploy. Fix the headers.', 9 * D + H],
      ['assistant', 'HTML now gets `Cache-Control: no-cache`; hashed assets keep a year and `immutable`.', 9 * D],
    ],
  });
  claude(root, {
    id: DELETED_FILE_ID,
    name: DELETED_SESSION,
    folderKey: 'logbook',
    branch: 'main',
    updated: 40 * D,
    created: 40 * D + 2 * H,
    count: 26,
    messages: [
      ['user', 'postcard has no schema, so bump the cache file name when SessionRecord changes', 40 * D + 2 * H],
      ['assistant', 'Renamed the cache to `index-v4.bin` and added a test that fails if the record layout changes without a bump.', 40 * D + 2 * H - 1],
    ],
  });

  if (history) addHistory(root);
}

/**
 * About 26 weeks of older sessions, so Stats has shape to show: busier on weekdays and afternoons, with Claude Code
 * taking over in the last months. Seeded, so every run gets the same sessions relative to now.
 */
function addHistory(root) {
  let x = 7;
  const rnd = () => {
    x = (x * 1103515245 + 12345) % 2147483648;
    return x / 2147483648;
  };
  const names = ['Tidy the release notes', 'Speed up the first index', 'Why does cargo rebuild everything?', 'Fix flaky upload retry', 'Bump plugins', 'Dark mode for the settings page', 'Retry the webhook on 502', 'Explain this borrow checker error', 'Add a test for the empty index', 'Reading list import from CSV'];
  const keys = ['logbook', 'logbook', 'reading', 'site', 'hooks'];
  const hourWeights = [1, 0, 0, 0, 0, 0, 0, 1, 3, 6, 7, 6, 3, 5, 7, 8, 6, 4, 2, 2, 3, 3, 2, 1];
  const hourPick = () => {
    let r = rnd() * hourWeights.reduce((a, b) => a + b, 0);
    for (let h = 0; h < 24; h++) if ((r -= hourWeights[h]) < 0) return h;
    return 15;
  };
  const now = new Date();
  let n = 0;
  for (let ago = 3; ago < 26 * 7; ago++) {
    const day = new Date(now.getFullYear(), now.getMonth(), now.getDate() - ago);
    const weekend = day.getDay() === 0 || day.getDay() === 6;
    if (rnd() < (weekend ? 0.72 : 0.12)) continue;
    const sessions = 1 + Math.floor(rnd() * (weekend ? 2 : 4));
    for (let i = 0; i < sessions; i++) {
      const at = new Date(day.getFullYear(), day.getMonth(), day.getDate(), hourPick(), Math.floor(rnd() * 60));
      const minsAgo = Math.max(10, (now.getTime() - at.getTime()) / 60_000);
      const count = 2 + Math.floor(rnd() * 30);
      const key = keys[Math.floor(rnd() * keys.length)];
      const name = names[Math.floor(rnd() * names.length)];
      const id = `${String(100000 + n).slice(1)}0000-0000-4000-8000-${String(n).padStart(12, '0')}`;
      n++;
      const messages = [['user', name, minsAgo + 5], ['assistant', 'Done.', minsAgo]];
      // Claude Code takes over in the last ~4 months.
      if (ago < 120 && rnd() < 0.55) claude(root, { id, name, folderKey: key, branch: 'main', updated: minsAgo, created: minsAgo + 5, count, messages });
      else copilot(root, { id, name, folderKey: key, branch: 'main', updated: minsAgo, created: minsAgo + 5, count, messages });
    }
  }
}
