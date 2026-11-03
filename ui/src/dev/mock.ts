/**
 * Browser-only mock of the Tauri backend, for `pnpm dev:mock`. Answers every command with
 * realistic fake sessions, so the UI can be worked on in a plain browser.
 * Loaded only in `mock` mode (see main.tsx), so it never reaches a production build.
 */
import { emit } from '@tauri-apps/api/event';
import { mockIPC } from '@tauri-apps/api/mocks';
import { entry, session } from '../test/fixtures';
import type { SearchHit, Session, TimelineEntry } from '../types';

const H = 60;
const D = 24 * H;

let seed: { s: Session; t: TimelineEntry[] }[] = [
  {
    s: session({
      id: 'copilot:3f1c9a2e-7b44-4d0e-9a51-1c2f0e8b7d10',
      updatedAt: ago(4),
      createdAt: ago(70),
      messageCount: 14,
    }),
    t: [
      entry('user', 'The overdue-loans test fails about one run in five on CI. Can you find out why?', 70),
      entry(
        'assistant',
        'The test seeds loans with `Utc::now()` and then asserts on a due date computed a few milliseconds later, so loans created right at the boundary land on either side of it.\n\nI\u2019ll freeze the clock in the fixture and compute the due date from the same instant.',
        68,
      ),
      entry('user', 'Do it, and add a regression test that pins the boundary.', 40),
      entry(
        'assistant',
        'Done. `due_date_is_inclusive` now covers a loan that falls due exactly now:\n\n```rust\n#[test]\nfn due_date_is_inclusive() {\n    let now = fixed_clock("2026-10-13T12:00:00Z");\n    let loan = loan_at(now - LOAN_PERIOD);\n    assert!(is_overdue(&loan, now));\n}\n```\n\n200 local runs, no failures.',
        4,
      ),
    ],
  },
  {
    s: session({
      id: 'copilot:8d2e4b61-1f0a-4c3b-b2d7-5e9a6c4f3a21',
      name: 'Dashboard env config for staging',
      repository: 'me/reading-list',
      branch: 'main',
      cwd: 'C:\\Repos\\reading-list',
      updatedAt: ago(3 * H),
      createdAt: ago(5 * H),
      messageCount: 22,
    }),
    t: [
      entry('user', 'please build the env config for staging, same shape as prod', 5 * H),
      entry(
        'assistant',
        'Sure, working on it now. I\u2019ll mirror `env/prod.json` into `env/staging.json` and point it at the staging API host.',
        5 * H - 2,
      ),
      entry('user', 'How does the reminder queue fit in?', 4 * H),
      entry(
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
      ),
    ],
  },
  {
    s: session({
      id: 'copilot:b7a0c3e9-22d4-4e18-8f6b-0d4c1a9e5b32',
      name: null,
      userNamed: false,
      repository: null,
      gitRoot: null,
      branch: null,
      cwd: 'C:\\Users\\me\\scratch',
      updatedAt: ago(1 * D + 2 * H),
      createdAt: ago(1 * D + 3 * H),
      messageCount: 4,
    }),
    t: [
      entry('user', 'What does `git rebase --update-refs` do?', D + 3 * H),
      entry(
        'assistant',
        'It moves any branches that point at commits being rebased along with them, so a stack of branches stays stacked.',
        D + 3 * H - 1,
      ),
    ],
  },
  {
    s: session({
      id: 'copilot:c41e7d02-9b3a-4f65-a1c8-7e2d5b0f6c43',
      name: 'Migrate session search to Tauri',
      repository: 'etienne/logbook',
      branch: 'tauri',
      cwd: 'D:\\Repos\\Other\\logbook',
      updatedAt: ago(4 * D),
      createdAt: ago(6 * D),
      messageCount: 57,
    }),
    t: [
      entry('user', 'Move the axum server behind Tauri commands and drop Docker.', 6 * D),
      entry('assistant', 'The core crate stays as is; commands wrap `SessionService`.', 6 * D - 3),
    ],
  },
  {
    s: session({
      id: 'copilot:e5f9a1b3-4c6d-4e8f-9a0b-1c2d3e4f5a54',
      name: 'Jira webhook signature check',
      repository: 'etienne/ado-hooks',
      branch: 'feat/jira',
      cwd: 'D:\\Repos\\Other\\ado-hooks',
      updatedAt: ago(19 * D),
      createdAt: ago(19 * D + 2 * H),
      messageCount: 31,
    }),
    t: [entry('user', 'Verify the webhook HMAC before parsing the body.', 19 * D + 2 * H)],
  },
  // Claude Code sessions, after the Copilot ones so `sel=<index>` in the screenshot scenes keeps its meaning.
  {
    s: claude({
      id: 'claude:9b2f6e1a-3c4d-4a7b-8e5f-1d2c3b4a5e61',
      name: 'Split the indexer into sources',
      repository: 'etienne/logbook',
      branch: 'feat/sources',
      cwd: 'D:\\Repos\\Other\\logbook',
      updatedAt: ago(2 * H),
      createdAt: ago(4 * H),
      messageCount: 38,
    }),
    t: [
      entry('user', 'Put Copilot behind a Source trait so a second agent is one new module.', 4 * H),
      entry(
        'assistant',
        'The indexer and service now only see `Source`: `discover` lists sessions with a fingerprint, `read` parses one, and `resume` returns a program and its arguments, never a shell string.',
        4 * H - 6,
      ),
      entry('user', 'Does the session count still match 0.1.1?', 2 * H + 10),
      entry('assistant', 'Yes: 505 listed on both, and search output is identical.', 2 * H),
    ],
  },
  {
    s: claude({
      id: 'claude:2e8a4c6b-7d1f-4b3a-9c5e-6f0a1b2c3d72',
      name: 'Why is the release job skipping?',
      repository: 'etienne/logbook',
      branch: 'main',
      cwd: 'D:\\Repos\\Other\\logbook',
      updatedAt: ago(D + 5 * H),
      createdAt: ago(D + 6 * H),
      messageCount: 9,
    }),
    t: [
      entry('user', 'The release workflow skips the build job on tags. Why?', D + 6 * H),
      entry(
        'assistant',
        "Its `if:` checks `github.ref == 'refs/heads/main'`, which is never true for a tag push. Use `startsWith(github.ref, 'refs/tags/v')` there.",
        D + 6 * H - 2,
      ),
    ],
  },
  {
    s: claude({
      id: 'claude:7c3d9e0f-1a2b-4c5d-8e6f-0a1b2c3d4e83',
      name: 'Cache headers for /docs',
      repository: 'etienne/site',
      branch: 'main',
      cwd: 'D:\\Repos\\Other\\site',
      updatedAt: ago(9 * D),
      createdAt: ago(9 * D + H),
      messageCount: 17,
    }),
    t: [
      entry('user', 'Docs pages are cached for a day after a deploy. Fix the headers.', 9 * D + H),
      entry('assistant', 'HTML now gets `Cache-Control: no-cache`; hashed assets keep a year and `immutable`.', 9 * D),
    ],
  },
  {
    s: claude({
      id: 'claude:4f5a6b7c-8d9e-4f0a-a1b2-c3d4e5f6a794',
      name: 'Postcard schema bump to v4',
      repository: 'etienne/logbook',
      branch: 'main',
      cwd: 'D:\\Repos\\Other\\logbook',
      updatedAt: ago(40 * D),
      createdAt: ago(40 * D + 2 * H),
      messageCount: 26,
      sourceDeletedAt: ago(9 * D + 3 * H),
    }),
    t: [
      entry('user', 'postcard has no schema, so bump the cache file name when SessionRecord changes', 40 * D + 2 * H),
      entry(
        'assistant',
        'Renamed the cache to `index-v4.bin` and added a test that fails if the record layout changes without a bump.',
        40 * D + 2 * H - 1,
      ),
    ],
  },
];

/** A Claude Code session: like `session`, with Claude Code's path and no client name. */
function claude(over: Partial<Session>): Session {
  const nativeId = over.id!.replace(/^claude:/, '');
  const project = (over.cwd ?? '').replace(/[^A-Za-z0-9]/g, '-');
  return session({
    agent: 'claude',
    clientName: null,
    userNamed: false,
    gitRoot: over.cwd ?? null,
    path: `C:\\Users\\me\\.claude\\projects\\${project}\\${nativeId}.jsonl`,
    ...over,
  });
}

function ago(min: number): string {
  return new Date(Date.now() - min * 60_000).toISOString();
}

/** Word-prefix AND search with STX/ETX-marked snippets, roughly like the Rust index. */
function search(q: string): SearchHit[] {
  const tokens = q
    .toLowerCase()
    .split(/[^\p{L}\p{N}_-]+/u)
    .filter(Boolean);
  if (tokens.length === 0) return [];
  return seed.flatMap(({ s, t }) => {
    const text = t.map((e) => e.text).join('\n');
    const words = text.split(/([^\p{L}\p{N}_-]+)/u);
    const isHit = (w: string) => tokens.some((tok) => w.toLowerCase().startsWith(tok));
    if (!tokens.every((tok) => words.some((w) => w.toLowerCase().startsWith(tok)))) return [];
    const first = words.findIndex(isHit);
    const from = Math.max(0, first - 8);
    const snippet = words
      .slice(from, first + 12)
      .map((w) => (isHit(w) ? `\u0002${w}\u0003` : w))
      .join('');
    return [{ id: s.id, snippet: `${from > 0 ? '... ' : ''}${snippet} ...` }];
  });
}

/**
 * `?many=1500`: that many sessions, copied from the seed across 40 repositories and 90 days,
 * to see how the UI holds up at a realistic size.
 */
function multiply(n: number): void {
  const base = seed;
  seed = Array.from({ length: n }, (_, i) => {
    const { s, t } = base[i % base.length]!;
    const nativeId = `${String(i).padStart(8, '0')}-0000-4000-8000-${s.nativeId.slice(-12)}`;
    const id = `copilot:${nativeId}`;
    const updatedAt = ago(i * ((90 * D) / n));
    return {
      s: {
        ...s,
        id,
        nativeId,
        path: s.path.replace(s.nativeId, nativeId),
        repository: `org/repo-${i % 40}`,
        updatedAt,
        createdAt: updatedAt,
      },
      t,
    };
  });
}

/**
 * States that are hard to reach by hand: `?long` adds a message long enough to start folded,
 * `?firstrun` is the first scan of an empty index, `?loaderror` fails loading the sessions and
 * `?transcripterror` fails reading every transcript.
 */
function longMessage(): string {
  const para =
    'The reminder handler reads the loan from its own store before it does anything, so a renewal that lands between scheduling and firing is seen. ';
  return ['Here is the full trace of the run, step by step.', ...Array.from({ length: 16 }, () => para.repeat(2))].join(
    '\n\n',
  );
}

export function installMock(): void {
  const p = new URLSearchParams(location.search);
  const many = Number(p.get('many'));
  if (many > 0) multiply(many);
  if (p.has('long')) seed[0]!.t.push(entry('assistant', longMessage(), 2));
  const firstRun = p.has('firstrun');
  if (firstRun || p.has('nosources')) seed = [];
  let indexing = firstRun;
  mockIPC(
    (cmd, args) => {
      const a = (args ?? {}) as Record<string, string>;
      const find = (id: string) => seed.find((x) => x.s.id === id);
      switch (cmd) {
        case 'app_info':
          return {
            version: '0.1.0',
            // `?nosources`: neither agent has run on this machine yet.
            sources: [
              {
                agent: 'copilot',
                dir: 'C:\\Users\\me\\.copilot\\session-state',
                short: '~\\.copilot\\session-state',
                found: !p.has('nosources'),
              },
              {
                agent: 'claude',
                dir: 'C:\\Users\\me\\.claude\\projects',
                short: '~\\.claude\\projects',
                found: !p.has('nosources'),
              },
            ],
            cacheFile: 'C:\\Users\\me\\AppData\\Local\\Logbook\\index-v5.bin',
          };
        case 'index_status':
          return { indexing, total: seed.length, scanned: 0, skipped: seed.length, generation: 1 };
        case 'reindex_now':
          indexing = true;
          setTimeout(() => {
            indexing = false;
            void emit('index-updated');
          }, 1200);
          return null;
        case 'recent_sessions':
          if (p.has('loaderror')) throw new Error('The index file is locked by another process.');
          // Newest first, like the real index.
          return seed
            .map((x) => ({ ...x.s, firstPrompt: x.t.find((e) => e.role === 'user')?.text ?? null }))
            .sort((a, b) => (b.updatedAt ?? '').localeCompare(a.updatedAt ?? ''));
        case 'search_sessions':
          return search(a.q ?? '');
        case 'get_transcript':
          if (p.has('transcripterror')) throw new Error('events.jsonl: unexpected end of file at line 212');
          return find(a.id!)?.t ?? [];
        case 'resume_command': {
          const s = find(a.id!)?.s;
          if (!s) return null;
          return s.agent === 'claude' ? `claude --resume ${s.nativeId}` : `copilot --resume=${s.nativeId}`;
        }
        case 'check_update':
          return mockUpdate();
        case 'install_update':
          // The real one never returns on Windows: the installer closes the app.
          return new Promise(() => undefined);
        case 'open_release_page':
        case 'resume_session':
        case 'open_folder':
        case 'open_link':
          return null;
        default:
          return null;
      }
    },
    { shouldMockEvents: true },
  );
  void applyUrlPresets();
}

/**
 * `?update` offers a newer version with realistic notes, and `?whatsnew` also opens its dialog
 * (`?whatsnew=installed` shows it as after an update).
 */
function mockUpdate() {
  const p = new URLSearchParams(location.search);
  if (!p.has('update') && !p.has('whatsnew')) return null;
  return {
    version: '0.2.0',
    date: new Date().toISOString(),
    notes: [
      '### Added',
      '- **Updates.** Logbook checks GitHub for new releases and shows **Update** in the toolbar.',
      '  What’s New shows the release notes, and **Update now** installs it and restarts.',
      '- **Check for updates** next to the version in About.',
      '',
      '### Fixed',
      '- The transcript keeps its scroll position when the index refreshes in the background.',
    ].join('\n'),
  };
}

/** `?theme=light&range=week&agent=claude&q=dash&sel=<id or index>&about&sources` set up a state directly, for screenshots. */
async function applyUrlPresets(): Promise<void> {
  const p = new URLSearchParams(location.search);
  const { useApp, applyTheme } = await import('../state/store');
  const theme = p.get('theme');
  if (theme === 'light' || theme === 'dark') {
    applyTheme(theme);
    useApp.setState({ theme });
  }
  if (p.has('about')) useApp.getState().setAbout(true);
  if (p.has('sources')) useApp.getState().setSourcesOpen(true);
  if (p.has('whatsnew')) {
    const update = await useApp.getState().checkUpdate();
    if (update) {
      const mode = p.get('whatsnew') === 'installed' ? 'installed' : 'update';
      useApp.getState().setWhatsNew({ ...update, mode });
    }
  }
  const range = p.get('range');
  if (range === 'all' || range === 'today' || range === 'week' || range === 'month') useApp.getState().setRange(range);
  const agent = p.get('agent');
  if (agent === 'copilot' || agent === 'claude') useApp.getState().setAgent(agent);
  const q = p.get('q');
  if (q) useApp.getState().setQuery(q);
  const sel = p.get('sel');
  if (sel) useApp.getState().select(seed[Number(sel)]?.s.id ?? sel);
}
