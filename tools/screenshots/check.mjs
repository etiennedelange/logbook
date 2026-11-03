/**
 * Clicks through the real UI in headless Chromium against the mock backend (ui/src/dev/mock.ts) and
 * checks what every control does: state (aria-*, disabled), text, notices, focus and the clipboard.
 *
 *   pnpm ui-check                  every section
 *   pnpm ui-check search agents    only those
 *
 * Starts its own mock dev server. Every check reports PASS or FAIL and the run carries on; it exits
 * non-zero if any check failed or the page logged an error. Rows are found by session name and counts
 * compared with the sidebar's badges, so the checks hold whatever the time of day (the mock's
 * "Today" and "Last 7 days" depend on it).
 */
import { chromium } from 'playwright';
import { startServer, stopServer } from './server.mjs';

const PORT = 1432; // not 1420 (`pnpm tauri dev`) or 1431 (screenshots)
const WIDTH = 1180; // the default window (tauri.conf.json)
const HEIGHT = 760;

const results = [];
const errors = [];
let section = '';

async function check(name, fn) {
  try {
    const note = await fn();
    results.push({ section, name, ok: true, note: typeof note === 'string' ? note : '' });
  } catch (e) {
    results.push({ section, name, ok: false, note: String(e?.message ?? e).split('\n')[0] });
  }
}

function eq(got, want, what) {
  if (JSON.stringify(got) !== JSON.stringify(want)) {
    throw new Error(`${what}: got ${JSON.stringify(got)}, want ${JSON.stringify(want)}`);
  }
}

function ok(cond, what) {
  if (!cond) throw new Error(what);
}

// ---- Page helpers ----

let browser;
let base;

async function open(params = {}, viewport = { width: WIDTH, height: HEIGHT }) {
  const ctx = await browser.newContext({ viewport, permissions: ['clipboard-read', 'clipboard-write'] });
  const page = await ctx.newPage();
  const qs = new URLSearchParams({ range: 'all', ...params }).toString().replace(/=(?=&|$)/g, '');
  page.on('console', (m) => m.type() === 'error' && errors.push(`[${section} ?${qs}] ${m.text()}`));
  page.on('pageerror', (e) => errors.push(`[${section} ?${qs}] ${e.message}`));
  await page.goto(`${base}?${qs}`);
  await page.waitForSelector('#root *');
  await page.waitForTimeout(500); // sessions, search and selection settle
  return page;
}

const rows = (p) => p.getByRole('option');
const row = (p, name) => p.getByRole('option').filter({ hasText: name });
const listPane = (p) => p.locator('section[aria-label="Sessions"]');
const detail = (p) => p.locator('section[data-pane="detail"]');
const transcript = (p) => detail(p).locator('ol[aria-label^="Transcript"]');
const listTitle = async (p) => (await listPane(p).locator('h1').textContent()).trim();
const searchBox = (p) => p.getByRole('searchbox', { name: 'Search sessions' });
const rangeNav = (p) => p.getByRole('navigation', { name: 'Time range' });
const agentNav = (p) => p.locator('aside nav').filter({ hasText: 'Agents' });
const currentRange = async (p) => (await rangeNav(p).locator('[aria-current="page"]').innerText()).split('\n')[0];
const status = async (p) => (await p.locator('aside [role="status"]').textContent()).trim();
const focused = (p) =>
  p.evaluate(() => {
    const e = document.activeElement;
    return e?.getAttribute('aria-label') ?? e?.getAttribute('role') ?? e?.tagName;
  });
/** Clicks somewhere inert so the app-wide hotkeys see the next key. */
const blur = (p) => p.evaluate(() => document.activeElement instanceof HTMLElement && document.activeElement.blur());
/** The badge on a sidebar button (range, agent or repository). */
const badge = async (btn) => Number((await btn.locator('span').last().textContent()).trim());

async function notice(p) {
  const n = p.locator('[aria-live="polite"] p');
  await n.waitFor({ timeout: 2000 });
  return (await n.textContent()).trim();
}

/**
 * Waits for a notice with this text (an earlier one may still be showing), and checks it's on top: an
 * open modal dialog sits in the top layer, above any z-index, and once hid About's "Check for updates"
 * result.
 */
async function noticeIs(p, text) {
  const n = p.locator('[aria-live="polite"] p').filter({ hasText: text });
  await n.waitFor({ timeout: 2000 });
  const onTop = await n.evaluate((el) => {
    // Hit-testing skips elements that ignore the pointer, as the notice does; count it in for this test.
    el.style.pointerEvents = 'auto';
    const r = el.getBoundingClientRect();
    const hit = document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2);
    el.style.pointerEvents = '';
    return el.contains(hit) || `covered by <${hit?.tagName.toLowerCase()}${hit?.id ? `#${hit.id}` : ''}>`;
  });
  ok(onTop === true, `notice "${text}" is ${onTop}`);
  return text;
}

// ---- Sections ----

const SECTIONS = {
  async start() {
    const p = await open();
    await check('loads every mock session, newest first', async () => {
      const all = await badge(rangeNav(p).getByRole('button', { name: /All sessions/ }));
      eq(await rows(p).count(), all, 'rows');
      eq(await status(p), `${all} sessions indexed`, 'status');
      const first = await rows(p).first().innerText();
      ok(first.includes('Fix the flaky overdue-loans test'), `first row: ${first.split('\n')[0]}`);
      return `${all} sessions`;
    });
    await check('nothing selected: empty detail with the shortcuts', async () => {
      await detail(p).getByText('No session selected').waitFor();
      ok((await detail(p).locator('kbd').count()) >= 7, 'shortcut keys missing');
    });
  },

  async ranges() {
    const p = await open();
    for (const [key, label] of [
      ['2', 'Today'],
      ['3', 'Last 7 days'],
      ['4', 'Last 30 days'],
      ['1', 'All sessions'],
    ]) {
      await check(`hotkey ${key} picks ${label}: current, title, rows match its badge`, async () => {
        await blur(p);
        await p.keyboard.press(key);
        const btn = rangeNav(p).getByRole('button', { name: new RegExp(label) });
        eq(await btn.getAttribute('aria-current'), 'page', 'aria-current');
        eq(await rangeNav(p).locator('[aria-current="page"]').count(), 1, 'one current range');
        eq(await listTitle(p), label, 'list title');
        eq(await rows(p).count(), await badge(btn), 'rows vs badge');
      });
    }
    await check('clicking a range picks it', async () => {
      await rangeNav(p).getByRole('button', { name: /Last 30 days/ }).click();
      eq(await currentRange(p), 'Last 30 days', 'range');
      await rangeNav(p).getByRole('button', { name: /All sessions/ }).click();
    });
    await check('an empty range offers "Show all sessions", which resets it', async () => {
      // ado-hooks has one session, 19 days old: nothing in it today.
      await p.locator('aside').getByRole('button', { name: /ado-hooks/ }).click();
      await blur(p);
      await p.keyboard.press('2');
      await listPane(p).getByText('Nothing in this range').waitFor({ timeout: 1500 });
      await p.getByRole('button', { name: 'Show all sessions' }).click();
      eq(await currentRange(p), 'All sessions', 'range');
      eq(await rows(p).count(), 1, 'rows (still filtered to ado-hooks)');
    });
  },

  async agents() {
    const p = await open();
    const copilot = agentNav(p).getByRole('button', { name: /Copilot CLI/ });
    const claude = agentNav(p).getByRole('button', { name: /Claude Code/ });
    await check('sidebar lists both agents; their badges add up to the total', async () => {
      const total = await rows(p).count();
      const [a, b] = [await badge(copilot), await badge(claude)];
      eq(a + b, total, 'copilot + claude');
      return `Copilot ${a}, Claude ${b}`;
    });
    await check('rows say which agent they are from', async () => {
      ok((await row(p, 'Split the indexer').innerText()).includes('Claude Code'), 'claude row');
      ok((await row(p, 'Dashboard env config').innerText()).includes('Copilot CLI'), 'copilot row');
    });
    await check('clicking an agent filters to it: pressed, title, rows, no agent label on rows', async () => {
      await claude.click();
      eq(await claude.getAttribute('aria-pressed'), 'true', 'pressed');
      eq(await listTitle(p), 'Claude Code', 'title');
      eq(await rows(p).count(), await badge(claude), 'rows vs badge');
      ok(!(await rows(p).first().innerText()).includes('Claude Code'), 'row still names its agent');
      await claude.click();
      eq(await claude.getAttribute('aria-pressed'), 'false', 'unpressed after a second click');
    });
    await check('Alt+1 / Alt+2 pick an agent, and again clears it', async () => {
      await blur(p);
      await p.keyboard.press('Alt+1');
      eq(await copilot.getAttribute('aria-pressed'), 'true', 'Alt+1');
      await p.keyboard.press('Alt+2');
      eq(await claude.getAttribute('aria-pressed'), 'true', 'Alt+2');
      eq(await copilot.getAttribute('aria-pressed'), 'false', 'Alt+2 replaces Alt+1');
      await p.keyboard.press('Alt+2');
      eq(await claude.getAttribute('aria-pressed'), 'false', 'Alt+2 again clears');
    });
    await check('Alt+1 works while typing in the search box', async () => {
      await searchBox(p).focus();
      await p.keyboard.press('Alt+1');
      eq(await copilot.getAttribute('aria-pressed'), 'true', 'pressed');
      eq(await searchBox(p).inputValue(), '', 'nothing typed');
      await p.keyboard.press('Alt+1');
    });
    await check('a repository with sessions from both agents splits by agent', async () => {
      const logbook = p.locator('aside').getByRole('button', { name: /^logbook/ });
      await logbook.click();
      const all = await rows(p).count();
      await claude.click();
      const onlyClaude = await rows(p).count();
      const sub = (await listPane(p).locator('header span').first().textContent()).trim();
      await claude.click();
      await logbook.click();
      ok(onlyClaude < all && onlyClaude > 0, `logbook ${all}, of which Claude ${onlyClaude}`);
      return `logbook: ${all}, Claude only: ${onlyClaude} ("${sub}")`;
    });
  },

  async repos() {
    const p = await open();
    const reading = p.locator('aside').getByRole('button', { name: /reading-list/ });
    await check('a repository toggles: pressed, title, rows, "All repositories" and "Clear"', async () => {
      await reading.click();
      eq(await reading.getAttribute('aria-pressed'), 'true', 'pressed');
      eq(await listTitle(p), 'reading-list', 'title');
      eq(await rows(p).count(), await badge(reading), 'rows vs badge');
      await p.getByRole('button', { name: 'All repositories' }).waitFor();
      await p.getByRole('button', { name: /Clear repository filter/ }).waitFor();
      await reading.click();
      eq(await reading.getAttribute('aria-pressed'), 'false', 'unpressed');
      eq(await p.getByRole('button', { name: 'All repositories' }).count(), 0, '"All repositories" gone');
    });
    await check('rows drop the repository name while filtered to one', async () => {
      await reading.click();
      const text = await rows(p).first().innerText();
      await p.getByRole('button', { name: 'All repositories' }).click();
      ok(!text.includes('reading-list'), `row: ${text.replace(/\s+/g, ' ')}`);
    });
    await check('"Clear" in the sidebar clears the filter', async () => {
      const total = await rows(p).count();
      await p.locator('aside').getByRole('button', { name: /^logbook/ }).click();
      await p.getByRole('button', { name: /Clear repository filter/ }).click();
      eq(await rows(p).count(), total, 'rows');
    });
  },

  async search() {
    const p = await open();
    const total = await rows(p).count();
    await check('/ focuses the search box', async () => {
      await blur(p);
      await p.keyboard.press('/');
      eq(await focused(p), 'Search sessions', 'focus');
    });
    await check('"reminder" finds one session, with the word marked in its snippet', async () => {
      await p.keyboard.type('reminder');
      await p.waitForTimeout(300);
      eq(await rows(p).count(), 1, 'rows');
      ok((await rows(p).first().locator('mark').count()) > 0, 'no mark in the snippet');
      return (await listPane(p).locator('header span').first().textContent()).trim();
    });
    await check('× clears the search', async () => {
      await p.getByRole('button', { name: 'Clear filter' }).click();
      eq(await searchBox(p).inputValue(), '', 'value');
      eq(await rows(p).count(), total, 'rows');
    });
    await check('Esc clears the box; a second Esc leaves it', async () => {
      await searchBox(p).fill('dash');
      await searchBox(p).press('Escape');
      eq(await searchBox(p).inputValue(), '', 'cleared');
      await searchBox(p).press('Escape');
      eq(await focused(p), 'BODY', 'blurred');
    });
    await check('no matches in a narrowed range offers "Search all sessions"', async () => {
      await p.keyboard.press('2');
      await searchBox(p).fill('webhook');
      await listPane(p).getByText('No matches').waitFor();
      await p.getByRole('button', { name: 'Search all sessions' }).click();
      await p.waitForTimeout(200);
      eq(await currentRange(p), 'All sessions', 'range');
      eq(await rows(p).count(), 1, 'rows');
      await searchBox(p).fill('');
    });
    await check('no matches across everything offers nothing more', async () => {
      await searchBox(p).fill('zzzqqq');
      await listPane(p).getByText('No matches').waitFor();
      eq(await p.getByRole('button', { name: 'Search all sessions' }).count(), 0, 'action shown');
      await searchBox(p).fill('');
    });
    await check('range hotkeys type normally in the search box', async () => {
      await searchBox(p).focus();
      await p.keyboard.type('2');
      eq(await searchBox(p).inputValue(), '2', 'value');
      eq(await currentRange(p), 'All sessions', 'range');
      await searchBox(p).fill('');
    });
    await check('a selected match: marks in the transcript and a "messages match" line', async () => {
      await searchBox(p).fill('reminder');
      await p.waitForTimeout(300);
      await rows(p).first().click();
      await transcript(p).locator('mark').first().waitFor();
      const marks = await transcript(p).locator('mark').count();
      const line = await detail(p).getByText(/match(es)? your search/).textContent();
      await searchBox(p).fill('');
      return `${marks} marks; "${line}"`;
    });
  },

  async selection() {
    const p = await open();
    await check('clicking a row selects it and shows its transcript', async () => {
      await row(p, 'Dashboard env config').click();
      eq(await row(p, 'Dashboard env config').getAttribute('aria-selected'), 'true', 'selected');
      await detail(p).locator('h1', { hasText: 'Dashboard env config for staging' }).waitFor();
      return await transcript(p).getAttribute('aria-label');
    });
    await check('Markdown renders: heading, lists, table, code, quote, link', async () => {
      const d = detail(p);
      const found = {};
      for (const s of ['h3', 'ol li', 'ul li', 'table', 'code', 'blockquote', 'a[href]']) found[s] = await d.locator(s).count();
      const missing = Object.entries(found).filter(([, n]) => n === 0);
      ok(missing.length === 0, `missing: ${missing.map(([s]) => s).join(', ')}`);
    });
    await check('arrows, Home and End move the selection and focus with it, stopping at the ends', async () => {
      const n = await rows(p).count();
      const sel = () => rows(p).evaluateAll((els) => els.findIndex((e) => e.getAttribute('aria-selected') === 'true'));
      await row(p, 'Dashboard env config').focus();
      const start = await sel();
      await p.keyboard.press('ArrowDown');
      eq(await sel(), start + 1, 'down');
      await p.keyboard.press('End');
      eq(await sel(), n - 1, 'end');
      await p.keyboard.press('ArrowDown');
      eq(await sel(), n - 1, 'stays at the end');
      await p.keyboard.press('Home');
      eq(await sel(), 0, 'home');
      await p.keyboard.press('ArrowUp');
      eq(await sel(), 0, 'stays at the top');
      eq(await p.evaluate(() => document.activeElement?.getAttribute('aria-selected')), 'true', 'focus follows');
    });
    await check('only one row is a Tab stop', async () => {
      eq(await rows(p).evaluateAll((els) => els.filter((e) => e.tabIndex === 0).length), 1, 'tabbable rows');
    });
    await check('Space selects the focused row; Enter on a selected row resumes it', async () => {
      const r = row(p, 'Migrate session search');
      await r.focus();
      await p.keyboard.press(' ');
      eq(await r.getAttribute('aria-selected'), 'true', 'selected');
      await p.keyboard.press('Enter');
      return await noticeIs(p, 'Opened a terminal to resume this session');
    });
    await check('Resume button resumes', async () => {
      await row(p, 'Dashboard env config').click();
      await p.getByRole('button', { name: 'Resume', exact: true }).click();
      return await noticeIs(p, 'Opened a terminal to resume this session');
    });
    await check('Copy puts the Copilot resume command on the clipboard', async () => {
      await p.getByRole('button', { name: 'Copy resume command' }).click();
      await noticeIs(p, 'Resume command copied');
      const clip = await p.evaluate(() => navigator.clipboard.readText());
      eq(clip, 'copilot --resume=8d2e4b61-1f0a-4c3b-b2d7-5e9a6c4f3a21', 'clipboard');
      return clip;
    });
    await check('Copy on a Claude Code session copies its own command; its replies say "Claude"', async () => {
      await row(p, 'Split the indexer').click();
      await transcript(p).waitFor();
      await p.getByRole('button', { name: 'Copy resume command' }).click();
      await noticeIs(p, 'Resume command copied');
      const clip = await p.evaluate(() => navigator.clipboard.readText());
      eq(clip, 'claude --resume 9b2f6e1a-3c4d-4a7b-8e5f-1d2c3b4a5e61', 'clipboard');
      ok((await transcript(p).innerText()).includes('Claude'), 'speaker');
      return clip;
    });
    await check('the resume dock shows the folder and command, and its folder button opens the folder', async () => {
      await row(p, 'Dashboard env config').click();
      const dock = detail(p).locator('code[title*="--resume"]');
      const text = (await dock.innerText()).replace(/\s+/g, ' ');
      ok(text.startsWith('C:\\Repos\\reading-list') && text.includes('--resume'), text);
      await detail(p).getByRole('button', { name: 'Open C:\\Repos\\reading-list' }).click();
      eq(await detail(p).locator('header').getByRole('button', { name: /^Open / }).count(), 0, 'in the header');
      return text;
    });
    await check('an untitled session is titled by its first prompt', async () => {
      await row(p, 'git rebase --update-refs').click();
      const h = (await detail(p).locator('header').innerText()).replace(/\s+/g, ' ');
      ok(h.includes('What does git rebase --update-refs do?'), h);
      return h.slice(0, 80);
    });
    await check('Esc in the detail returns to the row; Esc again closes the detail', async () => {
      await p.getByRole('button', { name: 'Resume', exact: true }).focus();
      await p.keyboard.press('Escape');
      eq(await focused(p), 'option', 'focus on the row');
      await p.keyboard.press('Escape');
      await detail(p).getByText('No session selected').waitFor({ timeout: 1500 });
      eq(await rows(p).locator('[aria-selected="true"]').count(), 0, 'selection');
    });
  },

  async deleted() {
    const p = await open({ sel: 'claude:4f5a6b7c-8d9e-4f0a-a1b2-c3d4e5f6a794' });
    await check('a session whose file is gone: readable, marked, and Resume is off', async () => {
      await detail(p).locator('h1', { hasText: 'Postcard schema bump to v4' }).waitFor();
      ok(await p.getByRole('button', { name: 'Resume', exact: true }).isDisabled(), 'Resume enabled');
      await detail(p).getByText(/deleted this session’s file/).waitFor();
      ok((await transcript(p).locator(':scope > li').count()) > 0, 'no transcript');
    });
  },

  async theme() {
    const p = await open({ theme: 'light' });
    await check('the theme button flips the theme, its label, and remembers it', async () => {
      const btn = p.locator('aside footer').getByRole('button', { name: /Switch to/ });
      const label = await btn.getAttribute('aria-label');
      const before = await p.evaluate(() => document.documentElement.dataset.theme);
      await btn.click();
      const after = await p.evaluate(() => document.documentElement.dataset.theme);
      ok(before !== after, `theme stayed ${before}`);
      ok((await btn.getAttribute('aria-label')) !== label, 'label unchanged');
      eq(await p.evaluate(() => localStorage.getItem('logbook-theme')), after, 'saved');
      return `${before} → ${after}`;
    });
  },

  async reindex() {
    const p = await open();
    const btn = p.getByRole('button', { name: 'Re-index sessions (r)' });
    await check('Re-index: off and spinning while it runs, "Indexing sessions…", then back', async () => {
      const done = await status(p);
      await btn.click();
      await p.waitForTimeout(100);
      ok(await btn.isDisabled(), 'button enabled while indexing');
      ok((await btn.locator('svg').getAttribute('class'))?.includes('animate-spin'), 'not spinning');
      eq(await status(p), 'Indexing sessions…', 'status');
      await p.waitForTimeout(1600);
      ok(!(await btn.isDisabled()), 'still disabled');
      eq(await status(p), done, 'status after');
    });
    await check('r re-indexes', async () => {
      await blur(p);
      await p.keyboard.press('r');
      await p.waitForTimeout(100);
      eq(await status(p), 'Indexing sessions…', 'status');
      await p.waitForTimeout(1500);
    });
  },

  async about() {
    const p = await open();
    const dialog = p.getByRole('dialog', { name: 'About Logbook' });
    await check('? opens About with focus inside; Esc closes it, and ? opens it again', async () => {
      await blur(p);
      await p.keyboard.press('?');
      await dialog.waitFor();
      ok(await p.evaluate(() => !!document.activeElement?.closest('dialog')), 'focus outside the dialog');
      await p.keyboard.press('Escape');
      await dialog.waitFor({ state: 'hidden' });
      // The dialog hides at once; the app hears of it from the close event a moment later.
      await p.waitForTimeout(100);
      await p.keyboard.press('?');
      await dialog.waitFor();
      await p.keyboard.press('Escape');
      await dialog.waitFor({ state: 'hidden' });
    });
    await check('the ? button opens it; Close and a click on the backdrop close it', async () => {
      await p.getByRole('button', { name: 'About Logbook' }).click();
      await dialog.waitFor();
      await dialog.getByRole('button', { name: 'Close' }).click();
      await dialog.waitFor({ state: 'hidden' });
      await p.getByRole('button', { name: 'About Logbook' }).click();
      await dialog.waitFor();
      await p.mouse.click(8, 8);
      await dialog.waitFor({ state: 'hidden' });
    });
    await check('hotkeys do nothing behind it', async () => {
      await p.getByRole('button', { name: 'About Logbook' }).click();
      await dialog.waitFor();
      await p.keyboard.press('2');
      await p.keyboard.press('r');
      await p.keyboard.press('Escape');
      eq(await currentRange(p), 'All sessions', 'range');
      ok(!(await status(p)).startsWith('Indexing'), 'r re-indexed');
    });
    await check('Tab stays inside it', async () => {
      await p.getByRole('button', { name: 'About Logbook' }).click();
      await dialog.waitFor();
      for (let i = 0; i < 10; i++) {
        await p.keyboard.press('Tab');
        const inside = await p.evaluate(
          () => !!document.activeElement?.closest('dialog') || document.activeElement === document.body,
        );
        ok(inside, `Tab ${i + 1} left the dialog`);
      }
      await p.keyboard.press('Escape');
    });
    await check('version, an open button per agent folder and the index, "Check for updates"', async () => {
      await p.getByRole('button', { name: 'About Logbook' }).click();
      await dialog.getByText('Version 0.1.0').waitFor();
      const folders = await dialog.getByRole('button', { name: /^Open the .* folder$/ }).count();
      eq(folders, 3, 'folder buttons (Copilot, Claude Code, index)');
      await dialog.getByRole('button', { name: 'Check for updates' }).click();
      await noticeIs(p, 'Logbook is up to date');
      await p.keyboard.press('Escape');
    });
  },

  async updates() {
    let p = await open({ update: '', sel: '0' });
    const wn = p.getByRole('dialog', { name: 'What’s New' });
    await check('an update shows the toolbar pill (after the 5 s startup check)', async () => {
      const pill = p.getByRole('button', { name: 'Update to 0.2.0' });
      await pill.waitFor({ timeout: 8000 });
      return `"${(await pill.innerText()).trim()}" at ${WIDTH}px`;
    });
    await check('the pill opens What’s New with "Update now" focused', async () => {
      await p.getByRole('button', { name: 'Update to 0.2.0' }).click();
      await wn.waitFor();
      await p.waitForTimeout(100);
      eq(await p.evaluate(() => document.activeElement?.textContent?.trim()), 'Update now', 'focus');
    });
    await check('Later closes it', async () => {
      await wn.getByRole('button', { name: 'Later' }).click();
      await wn.waitFor({ state: 'hidden' });
    });
    await check('About → Check for updates opens What’s New in About’s place', async () => {
      await p.getByRole('button', { name: 'About Logbook' }).click();
      await p.getByRole('dialog', { name: 'About Logbook' }).getByRole('button', { name: 'Check for updates' }).click();
      await wn.waitFor();
      ok(!(await p.getByRole('dialog', { name: 'About Logbook' }).isVisible()), 'About still open');
    });
    await check('"Release page" works', async () => {
      await wn.getByRole('button', { name: 'Release page' }).click();
    });
    await check('Update now: everything off, and the dialog stays until Logbook restarts', async () => {
      await wn.getByRole('button', { name: 'Update now' }).click();
      ok(await wn.getByRole('button', { name: 'Updating…' }).isDisabled(), '"Updating…" enabled');
      ok(await wn.getByRole('button', { name: 'Later' }).isDisabled(), 'Later enabled');
      ok(await wn.getByRole('button', { name: 'Close' }).isDisabled(), 'Close enabled');
      for (const how of ['Escape', 'Escape', 'backdrop']) {
        if (how === 'backdrop') await p.mouse.click(8, 8);
        else await p.keyboard.press(how);
        await p.waitForTimeout(150);
        ok(await wn.isVisible(), `closed by ${how}`);
      }
      ok(await p.getByRole('button', { name: 'Updating…' }).first().isDisabled(), 'pill enabled');
      return (await wn.locator('#whats-new-sub').textContent()).trim();
    });

    p = await open({ whatsnew: '' });
    await check('without an install running, Esc closes What’s New', async () => {
      const d = p.getByRole('dialog', { name: 'What’s New' });
      await d.waitFor();
      await p.keyboard.press('Escape');
      await d.waitFor({ state: 'hidden' });
    });
    p = await open({ whatsnew: 'installed' });
    await check('after an update it opens once, with "Got it"', async () => {
      const d = p.getByRole('dialog', { name: 'What’s New' });
      await d.waitFor();
      await d.getByRole('button', { name: 'Got it' }).click();
      await d.waitFor({ state: 'hidden' });
    });
  },

  async layout() {
    let p = await open({ update: '', sel: '1' }, { width: 1440, height: 800 });
    await check('from 1400px the pill names the version', async () => {
      const pill = p.getByRole('button', { name: 'Update to 0.2.0' });
      await pill.waitFor({ timeout: 8000 });
      eq((await pill.innerText()).trim(), 'Update to 0.2.0', 'pill');
    });
    for (const width of [1180, 900, 760]) {
      p = await open({ sel: '1' }, { width, height: 700 });
      await check(`${width}px: no sideways scroll, dates whole, placeholder fits`, async () => {
        eq(await p.evaluate(() => document.documentElement.scrollWidth - innerWidth), 0, 'page overflow');
        const cut = await detail(p)
          .locator('dl')
          .first()
          .locator('dd')
          .evaluateAll((els) => els.filter((e) => e.scrollWidth > e.clientWidth).map((e) => e.textContent));
        eq(cut, [], 'truncated facts');
        const fits = await searchBox(p).evaluate((e) => {
          const s = getComputedStyle(e);
          const ctx = document.createElement('canvas').getContext('2d');
          ctx.font = `${s.fontSize} ${s.fontFamily}`;
          return ctx.measureText(e.placeholder).width <= e.clientWidth - parseFloat(s.paddingLeft) - parseFloat(s.paddingRight);
        });
        ok(fits, 'placeholder cut off');
        const h1 = detail(p).locator('h1');
        const [sw, cw] = await h1.evaluate((e) => [e.scrollWidth, e.clientWidth]);
        return `title ${sw > cw ? `cut (${cw}/${sw}px)` : 'whole'}`;
      });
      // Issue #23: screen-reader-only labels (position: absolute) deep in a pane made the page taller
      // than the window, so scrolling a row or match into view slid the whole app up.
      await check(`${width}px: the page never scrolls; only the panes do`, async () => {
        const page = () =>
          p.evaluate(() => [document.documentElement.scrollHeight - innerHeight, document.scrollingElement.scrollTop]);
        eq(await page(), [0, 0], 'page overflow, scrollTop on load');
        await searchBox(p).fill('the');
        await p.waitForTimeout(400);
        const n = Math.min(await rows(p).count(), 8);
        ok(n > 1, 'too few rows to scroll');
        for (let i = 0; i < n; i++) {
          await rows(p).nth(i).click();
          await p.waitForTimeout(150);
          eq(await page(), [0, 0], `page overflow, scrollTop after row ${i}`);
        }
        await searchBox(p).fill('');
      });
    }
    await check('below 960px the sidebar is an icon rail', async () => {
      const w = await p.locator('aside').evaluate((e) => e.getBoundingClientRect().width);
      eq(w, 56, 'sidebar width');
    });
    await check('at 760px the resume dock fits: Resume, copy and open folder all in view', async () => {
      const pane = await detail(p).evaluate((e) => e.getBoundingClientRect().right);
      for (const name of ['Resume', 'Copy resume command', /^Open /]) {
        const b = detail(p).getByRole('button', { name, exact: typeof name === 'string' });
        const r = await b.evaluate((e) => e.getBoundingClientRect().right);
        ok(r <= pane, `${name} overflows the pane (${r} > ${pane})`);
      }
    });
  },

  async states() {
    let p = await open({ long: '', sel: '0' });
    await check('?long: a long message starts folded; "Show full message" and "Show less" toggle it', async () => {
      const more = p.getByRole('button', { name: /Show full message/ });
      const label = await more.textContent();
      await more.click();
      await p.getByRole('button', { name: 'Show less' }).click();
      await more.waitFor();
      return label;
    });
    p = await open({ long: '', q: 'renewal', sel: '0' });
    await check('?long with a matching search: the message starts unfolded', async () => {
      await p.getByRole('button', { name: 'Show less' }).waitFor();
    });
    p = await open({ transcripterror: '', sel: '1' });
    await check('?transcripterror: the detail says why it couldn’t read the transcript', async () => {
      return (await detail(p).getByRole('alert').textContent()).trim();
    });
    p = await open({ loaderror: '' });
    await check('?loaderror: the list says why it couldn’t load', async () => {
      await listPane(p).getByText('Couldn’t load sessions').waitFor();
    });
    p = await open({ firstrun: '' });
    await check('?firstrun: an empty index being built explains itself', async () => {
      await listPane(p).getByText('Indexing your sessions…').waitFor();
      eq(await status(p), 'Indexing sessions…', 'status');
    });
  },

  async scale() {
    const p = await open({ many: '1500' });
    await check('1500 sessions: the list renders a page and grows as it scrolls', async () => {
      const before = await rows(p).count();
      await p.getByRole('listbox', { name: 'Sessions' }).evaluate((e) => (e.scrollTop = e.scrollHeight));
      await p.waitForTimeout(400);
      const after = await rows(p).count();
      ok(after > before, `${before} → ${after}`);
      return `${before} → ${after} rows`;
    });
    await check('End from the first row goes further down the list', async () => {
      const before = await rows(p).count();
      await rows(p).first().focus();
      await p.keyboard.press('End');
      await p.waitForTimeout(300);
      return `${before} → ${await rows(p).count()} rows`;
    });
    await check('the repository filter box narrows the list; Esc clears it', async () => {
      const f = p.getByRole('searchbox', { name: 'Filter repositories' });
      await f.fill('repo-1');
      const n = await p.locator('aside').getByRole('button', { name: /repo-1/ }).count();
      await f.fill('nope');
      await p.getByText('No repositories match.').waitFor();
      await f.press('Escape');
      eq(await f.inputValue(), '', 'cleared');
      return `${n} repositories match "repo-1"`;
    });
    await check('/ goes to the session search, not the repository filter', async () => {
      await blur(p);
      await p.keyboard.press('/');
      eq(await focused(p), 'Search sessions', 'focus');
    });
  },

  async look() {
    for (const theme of ['light', 'dark']) {
      const p = await open({ sel: '1', theme });
      await check(`${theme}: the window's close button turns red on hover`, async () => {
        const close = p.getByRole('group', { name: 'Window' }).getByRole('button', { name: 'Close' });
        await close.hover();
        const bg = await close.evaluate((e) => getComputedStyle(e).backgroundColor);
        ok(bg !== 'rgba(0, 0, 0, 0)', 'transparent: the white × vanishes');
      });
    }
    const p = await open({ sel: '1' });
    await check('every Tab stop shows a focus ring', async () => {
      const missing = [];
      for (let i = 0; i < 25; i++) {
        await p.keyboard.press('Tab');
        const r = await p.evaluate(() => {
          const e = document.activeElement;
          if (!e || e === document.body) return null;
          const s = getComputedStyle(e);
          const ring = (s.outlineStyle !== 'none' && parseFloat(s.outlineWidth) > 0) || s.boxShadow !== 'none';
          return { ring, name: (e.getAttribute('aria-label') ?? e.textContent ?? e.tagName).trim().slice(0, 30) };
        });
        if (r && !r.ring) missing.push(r.name);
      }
      eq(missing, [], 'no ring on');
    });
  },
};

// ---- Run ----

const wanted = process.argv.slice(2);
const names = wanted.length ? wanted : Object.keys(SECTIONS);
const unknown = names.filter((n) => !(n in SECTIONS));
if (unknown.length) {
  console.error(`No such section: ${unknown.join(', ')}. Sections: ${Object.keys(SECTIONS).join(', ')}`);
  process.exit(1);
}

const server = await startServer(PORT);
base = server.url;
browser = await chromium.launch();
try {
  for (const name of names) {
    section = name;
    await SECTIONS[name]();
  }
} finally {
  await browser.close();
  stopServer(server.proc);
}

let last = '';
for (const r of results) {
  if (r.section !== last) console.log(`\n${(last = r.section)}`);
  console.log(`  ${r.ok ? 'PASS' : 'FAIL'}  ${r.name}${r.note ? `  — ${r.note}` : ''}`);
}
const failed = results.filter((r) => !r.ok).length;
console.log(`\n${results.length - failed}/${results.length} passed`);
if (errors.length) console.log(`\nPage errors:\n  ${errors.join('\n  ')}`);
process.exit(failed || errors.length ? 1 : 0);
