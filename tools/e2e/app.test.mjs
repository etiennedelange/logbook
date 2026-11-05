/**
 * End-to-end: the built app (Rust backend, Tauri IPC and the React UI together) against session files on
 * disk, driven over WebView2's DevTools protocol (cdp.mjs). `pnpm ui-check` covers the UI in depth against
 * mock data; this covers the path the mock skips: reading real files, indexing, searching and reading
 * transcripts.
 *
 *   pnpm tauri build --debug --no-bundle   # once, and after a change
 *   pnpm e2e
 *
 * Windows only, and with Logbook itself closed (a second instance hands over to it and exits).
 * LOGBOOK_APP overrides the binary. A failing test leaves a screenshot in tools/e2e/out/.
 */
import assert from 'node:assert/strict';
import { existsSync, mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { after, before, describe, test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { CLAUDE, COPILOT, makeHome } from './fixtures.mjs';
import { launch } from './cdp.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, '..', '..');
const app =
  process.env.LOGBOOK_APP ?? join(root, 'target', 'debug', process.platform === 'win32' ? 'logbook.exe' : 'logbook');
const out = join(here, 'out');

let home;
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

describe('Logbook against session files on disk', { timeout: 120_000, skip: process.platform !== 'win32' && 'Windows only' }, () => {
  before(async () => {
    assert.ok(existsSync(app), `no app at ${app}: run pnpm tauri build --debug --no-bundle first`);
    rmSync(out, { recursive: true, force: true });
    home = makeHome();
    // WebView2's profile goes in the throwaway home too.
    s = await launch(app, { env: { ...home.env, WEBVIEW2_USER_DATA_FOLDER: join(home.root, 'webview2') } });
  });

  after(async () => {
    s?.quit();
    // WebView2's processes hold its profile for a moment after the app is killed. A temp folder left
    // behind isn't a test failure.
    try {
      if (home) rmSync(home.root, { recursive: true, force: true, maxRetries: 20, retryDelay: 250 });
    } catch (e) {
      console.warn(`left ${home.root}: ${e.message}`);
    }
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

  step('Sources lists both agents’ folders, found on disk', async () => {
    const invoke = (cmd, args) => window.__TAURI_INTERNALS__.invoke(cmd, args);
    const info = await s.exec(invoke, 'app_info', {});
    const byAgent = Object.fromEntries(info.sources.map((x) => [x.agent, x]));
    assert.equal(byAgent.copilot?.dir, join(home.env.COPILOT_HOME, 'session-state'));
    assert.equal(byAgent.claude?.dir, join(home.env.CLAUDE_CONFIG_DIR, 'projects'));
    assert.ok(byAgent.copilot.found && byAgent.claude.found, JSON.stringify(info.sources));
    await s.waitFor(
      'the "2 sources" button',
      () => [...document.querySelectorAll('aside footer button')].some((b) => b.innerText.trim() === '2 sources'),
    );
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
    const sessions = await s.exec(invoke, 'recent_sessions', {});
    const byAgent = Object.fromEntries(sessions.map((x) => [x.agent, x]));
    assert.equal(byAgent.copilot?.nativeId, COPILOT.id);
    assert.equal(byAgent.claude?.nativeId, CLAUDE.id);
    const copilot = await s.exec(invoke, 'resume_command', { id: byAgent.copilot.id });
    const claude = await s.exec(invoke, 'resume_command', { id: byAgent.claude.id });
    assert.ok(copilot.includes(`copilot --resume=${COPILOT.id}`), copilot);
    assert.ok(claude.includes(`claude --resume ${CLAUDE.id}`), claude);
  });

  step('the backend reads the files a Claude session changed, and finds it by path and by id', async () => {
    const invoke = (cmd, args) => window.__TAURI_INTERNALS__.invoke(cmd, args);
    const id = `claude:${CLAUDE.id}`;
    const files = await s.exec(invoke, 'get_session_files', { id });
    assert.equal(files.files.length, 1, JSON.stringify(files));
    const path = files.files[0].path;
    assert.ok(/^src[\\/]zebra_rows\.rs$/.test(path), path);
    const edits = await s.exec(invoke, 'get_file_edits', { id, path });
    assert.deepEqual(edits[0].hunks[0].lines, ['-    i % 2 == 0', '+    i == 0']);
    const byPath = await s.exec(invoke, 'search_sessions', { q: 'zebra_rows.rs' });
    assert.deepEqual(byPath.map((h) => [h.id, h.matched]), [[id, 'file']]);
    const byId = await s.exec(invoke, 'search_sessions', { q: `claude --resume ${CLAUDE.id}` });
    assert.deepEqual(byId.map((h) => [h.id, h.matched]), [[id, 'session']]);
  });

  step('a path search opens the session on its Files tab', async (t) => {
    const box = await s.find('input[type="search"]');
    t.after(() => s.escape(box));
    await s.type(box, 'zebra_rows.rs');
    await s.waitFor('the file hit', `() => (${rowTexts})().some((r) => r.includes('Edited'))`);
    assert.ok(await s.exec(`() => (${clickRow})(${JSON.stringify(CLAUDE.prompt)})`), 'Claude row not found');
    await s.waitFor('the Files tab', () => !!document.querySelector('section[data-pane="detail"] section[aria-label="Edited"]'));
    // Back to the transcript, which the next step reads.
    await s.exec(() => [...document.querySelectorAll('nav[aria-label="Session view"] button')].find((b) => b.innerText.trim() === 'Transcript').click());
    await s.escape(box);
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
