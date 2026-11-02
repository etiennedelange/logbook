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
    s: session({ id: '3f1c9a2e-7b44-4d0e-9a51-1c2f0e8b7d10', updatedAt: ago(4), createdAt: ago(70), messageCount: 14 }),
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
      id: '8d2e4b61-1f0a-4c3b-b2d7-5e9a6c4f3a21',
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
      id: 'b7a0c3e9-22d4-4e18-8f6b-0d4c1a9e5b32',
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
      id: 'c41e7d02-9b3a-4f65-a1c8-7e2d5b0f6c43',
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
      id: 'e5f9a1b3-4c6d-4e8f-9a0b-1c2d3e4f5a54',
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
];

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
    const id = `${String(i).padStart(8, '0')}-0000-4000-8000-${s.id.slice(-12)}`;
    const updatedAt = ago(i * ((90 * D) / n));
    return { s: { ...s, id, repository: `org/repo-${i % 40}`, updatedAt, createdAt: updatedAt }, t };
  });
}

export function installMock(): void {
  const many = Number(new URLSearchParams(location.search).get('many'));
  if (many > 0) multiply(many);
  let indexing = false;
  mockIPC(
    (cmd, args) => {
      const a = (args ?? {}) as Record<string, string>;
      const find = (id: string) => seed.find((x) => x.s.id === id);
      switch (cmd) {
        case 'app_info':
          return {
            version: '0.1.0',
            sessionsDir: 'C:\\Users\\me\\.copilot\\session-state',
            cacheFile: 'C:\\Users\\me\\AppData\\Local\\Logbook\\index-v3.bin',
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
          return seed.map((x) => ({ ...x.s, firstPrompt: x.t.find((e) => e.role === 'user')?.text ?? null }));
        case 'search_sessions':
          return search(a.q ?? '');
        case 'get_transcript':
          return find(a.id!)?.t ?? [];
        case 'resume_command':
          return `copilot --resume=${a.id}`;
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

/** `?theme=light&range=week&q=dash&sel=<id or index>&about` set up a state directly, for screenshots. */
async function applyUrlPresets(): Promise<void> {
  const p = new URLSearchParams(location.search);
  const { useApp, applyTheme } = await import('../state/store');
  const theme = p.get('theme');
  if (theme === 'light' || theme === 'dark') {
    applyTheme(theme);
    useApp.setState({ theme });
  }
  if (p.has('about')) useApp.getState().setAbout(true);
  const range = p.get('range');
  if (range === 'all' || range === 'today' || range === 'week' || range === 'month') useApp.getState().setRange(range);
  const q = p.get('q');
  if (q) useApp.getState().setQuery(q);
  const sel = p.get('sel');
  if (sel) useApp.getState().select(seed[Number(sel)]?.s.id ?? sel);
}
