/**
 * End-to-end: the built app (Rust backend, Tauri IPC and the React UI together) against session files on
 * disk, driven through tauri-driver. `pnpm ui-check` covers the UI in depth against mock data; this
 * covers the path the mock skips: reading real files, indexing, searching and reading transcripts.
 *
 *   pnpm tauri build --debug --no-bundle   # once, and after a change
 *   pnpm e2e
 *
 * Needs tauri-driver (`cargo install tauri-driver --locked`) and the platform's driver: WebKitWebDriver
 * (Debian and Ubuntu: webkitgtk-webdriver, formerly webkit2gtk-driver) under a display (xvfb-run on a headless box), or msedgedriver on Windows.
 * LOGBOOK_APP overrides the binary. A failing test leaves a screenshot in tools/e2e/out/.
 */
import assert from 'node:assert/strict';
import { existsSync, mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { after, before, describe, test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { CLAUDE, COPILOT, makeHome } from './fixtures.mjs';
import { Session, startDriver } from './webdriver.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, '..', '..');
const app =
  process.env.LOGBOOK_APP ?? join(root, 'target', 'debug', process.platform === 'win32' ? 'logbook.exe' : 'logbook');
const out = join(here, 'out');

let home;
let driver;
let s;

/** Runs a test step; on failure saves a screenshot named after it before failing. */
const step = (name, fn) =>
  test(name, async (t) => {
    try {
      await fn(t);
    } catch (e) {
      try {
        mkdirSync(out, { recursive: true });
        writeFileSync(join(out, `${name.replace(/[^\w-]+/g, '-')}.png`), await s.screenshot());
      } catch {}
      throw e;
    }
  });

// ---- In-page helpers (each runs inside the webview; keep them self-contained) ----

const rowTexts = () => [...document.querySelectorAll('[role="option"]')].map((r) => r.innerText.replace(/\s+/g, ' '));

const clickRow = (text) => {
  const r = [...document.querySelectorAll('[role="option"]')].find((e) => e.innerText.includes(text));
  if (!r) return false;
  r.click();
  return true;
};

const detailText = () => document.querySelector('section[data-pane="detail"]')?.innerText ?? '';

describe('Logbook against session files on disk', { timeout: 120_000 }, () => {
  before(async () => {
    assert.ok(existsSync(app), `no app at ${app}: run pnpm tauri build --debug --no-bundle first`);
    rmSync(out, { recursive: true, force: true });
    home = makeHome();
    driver = await startDriver({ env: home.env });
    s = await Session.start(driver.base, app);
  });

  after(async () => {
    await s?.quit().catch(() => {});
    driver?.proc.kill();
    if (home) rmSync(home.root, { recursive: true, force: true });
  });

  step('indexes the Copilot and Claude Code sessions it finds', async () => {
    const rows = await s.waitFor('two session rows', `() => { const r = (${rowTexts})(); return r.length === 2 && r; }`);
    assert.ok(rows.some((r) => r.includes(COPILOT.name)), `no Copilot row in ${JSON.stringify(rows)}`);
    // An unnamed Claude session is titled by its first prompt.
    assert.ok(rows.some((r) => r.includes(CLAUDE.prompt)), `no Claude row in ${JSON.stringify(rows)}`);
    const agents = await s.exec(() => document.querySelector('nav[aria-labelledby="agents-heading"]')?.innerText ?? '');
    assert.match(agents, /Copilot CLI/);
    assert.match(agents, /Claude Code/);
  });

  step('search finds a word that is only in a transcript, and marks it', async (t) => {
    const box = await s.find('input[type="search"]');
    t.after(() => s.escape(box)); // so a failure here doesn't leave the list filtered for the next steps
    // In the Claude reply only: not in either title.
    await s.type(box, 'alternate');
    const rows = await s.waitFor(
      'one search hit',
      `() => { const r = (${rowTexts})(); return r.length === 1 && document.querySelector('[role="option"] mark') && r; }`,
    );
    assert.ok(rows[0].includes(CLAUDE.prompt), rows[0]);
    const mark = await s.exec(() => document.querySelector('[role="option"] mark').textContent);
    assert.equal(mark.toLowerCase(), 'alternate');
    await s.escape(box);
    await s.waitFor('both rows back', `() => (${rowTexts})().length === 2`);
  });

  step('opening a session reads its transcript from disk', async () => {
    assert.ok(await s.exec(`() => (${clickRow})(${JSON.stringify(COPILOT.name)})`), 'Copilot row not found');
    const text = await s.waitFor('the transcript', `() => { const t = (${detailText})(); return t.includes('freeze it') && t; }`);
    assert.ok(text.includes(COPILOT.name), 'title missing');
    assert.ok(text.includes(COPILOT.prompt), 'prompt missing');
    assert.ok(text.includes(COPILOT.branch), 'branch missing');
  });

  step('the backend builds each agent’s resume command', async () => {
    const invoke = (cmd, args) => window.__TAURI_INTERNALS__.invoke(cmd, args);
    const sessions = await s.execAsync(invoke, 'recent_sessions', {});
    const byAgent = Object.fromEntries(sessions.map((x) => [x.agent, x]));
    assert.equal(byAgent.copilot?.nativeId, COPILOT.id);
    assert.equal(byAgent.claude?.nativeId, CLAUDE.id);
    const copilot = await s.execAsync(invoke, 'resume_command', { id: byAgent.copilot.id });
    const claude = await s.execAsync(invoke, 'resume_command', { id: byAgent.claude.id });
    assert.ok(copilot.includes(`copilot --resume=${COPILOT.id}`), copilot);
    assert.ok(claude.includes(`claude --resume ${CLAUDE.id}`), claude);
  });

  step('a Claude session whose file is deleted stays readable, marked and not resumable', async () => {
    rmSync(home.claudeFile);
    await s.click(await s.find('button[aria-label^="Re-index sessions"]'));
    await s.waitFor('the "source deleted" mark', `() => (${rowTexts})().some((r) => r.includes('source deleted'))`);
    assert.ok(await s.exec(`() => (${clickRow})(${JSON.stringify(CLAUDE.prompt)})`), 'Claude row not found');
    await s.waitFor('the kept transcript', `() => (${detailText})().includes('split_rows')`);
    const disabled = await s.exec(() => {
      const b = [...document.querySelectorAll('section[data-pane="detail"] button')].find((e) => e.innerText.trim() === 'Resume' || e.getAttribute('aria-label') === 'Resume');
      return b ? b.disabled : 'no Resume button';
    });
    assert.equal(disabled, true);
  });
});
