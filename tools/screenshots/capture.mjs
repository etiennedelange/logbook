/**
 * Screenshots of the real UI against the mock backend (`pnpm --dir ui dev:mock`'s fake sessions),
 * so no private session data ever appears in them.
 *
 *   pnpm screenshots                       every scene and clip, both themes
 *   pnpm screenshots browse search-clip    only those
 *
 * One-time setup:  pnpm --dir tools/screenshots exec playwright install chromium
 * (that also installs the ffmpeg build Playwright ships, which encodes the clips; an ffmpeg on PATH wins.)
 *
 * Everything renders into tools/screenshots/out/ (gitignored). The PUBLISHED files are also
 * copied to docs/screenshots/, which the README and the marketing site show.
 */
import { spawn } from 'node:child_process';
import { existsSync, readdirSync } from 'node:fs';
import { copyFile, mkdir, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, '..', '..');
const out = join(here, 'out');
const published = join(root, 'docs', 'screenshots');
const PORT = 1431; // not 1420, so this can run next to `pnpm tauri dev`

// The default window (tauri.conf.json: 1180x760). It has no native title bar: the pane headers are it.
const WIDTH = 1180;
const HEIGHT = 760;

// Regions of the window, in CSS pixels. The list pane starts after the 220px sidebar and ends at 603.
const LIST_AND_DETAIL = { x: 220, y: 0, width: 960, height: 520 };
const LIST_ONLY = { x: 220, y: 0, width: 383, height: 240 };

/**
 * Query parameters understood by ui/src/dev/mock.ts. `crop` cuts a region out of the window
 * (no window frame: the site frames those itself); without it the shot is the whole window.
 */
const SCENES = {
  browse: { params: { sel: '0', range: 'all' } },
  search: { params: { q: 'reminder', sel: '1', range: 'all' } },
  about: { params: { sel: '0', range: 'all', about: '1' } },
  // Two agents: Claude Code picked in the sidebar, and a session whose transcript it deleted (seed 8).
  agents: { params: { agent: 'claude', sel: '8', range: 'all' } },
  // An update on offer: the toolbar pill and What's New with the release notes.
  'whats-new': { params: { sel: '0', range: 'all', whatsnew: '1' } },
  // The site's search section: the matching row and the highlighted messages, legible at its size.
  'search-detail': { params: { q: 'reminder', sel: '1', range: 'all' }, crop: LIST_AND_DETAIL },
  // Phone-width stand-ins for the full window, whose text would be ~3px tall there.
  'search-narrow': { params: { q: 'reminder', range: 'all' }, crop: LIST_ONLY },
  'browse-narrow': { params: { range: 'all' }, crop: { ...LIST_ONLY, height: 462 } },
};

/**
 * Short WebM clips, recorded step by step so every frame is a settled render (no timing jitter).
 * Each step changes the UI, then holds that frame for `hold` seconds. Kept under five seconds and
 * played once by the site, so it needs no pause control (WCAG 2.2.2).
 */
const FPS = 25;
const CLIPS = {
  'search-clip': {
    params: { range: 'all' },
    crop: LIST_AND_DETAIL,
    steps: [
      { hold: 0.7 },
      { press: '/', hold: 0.3 },
      ...[...'reminder'].map((ch) => ({ type: ch, hold: 0.1 })),
      { hold: 0.7 },
      { select: 0, hold: 1.6 }, // open the match: the transcript shows the highlighted messages
    ],
  },
};

/** The files the README and the site show; the rest are for looking at after a change. */
const PUBLISHED = [
  'browse-dark.webp',
  'browse-light.webp',
  'agents-dark.webp',
  'agents-light.webp',
  'search-dark.webp',
  'search-light.webp',
  'search-detail-dark.webp',
  'search-detail-light.webp',
  'search-narrow-dark.webp',
  'search-narrow-light.webp',
  'browse-narrow-dark.webp',
  'browse-narrow-light.webp',
  'whats-new-dark.webp',
  'whats-new-light.webp',
  'search-clip-dark.webm',
  'search-clip-light.webm',
];

// The hairline drawn around a whole-window shot: the app's own border colour per theme.
const THEMES = {
  dark: { hairline: 'rgba(255,255,255,.12)' },
  light: { hairline: 'rgba(20,16,40,.14)' },
};

const wanted = process.argv.slice(2);
const pick = (names) => names.filter((s) => wanted.length === 0 || wanted.includes(s));
const scenes = pick(Object.keys(SCENES));
const clips = pick(Object.keys(CLIPS));
if (scenes.length + clips.length === 0) {
  console.error(`No such scene. Scenes: ${[...Object.keys(SCENES), ...Object.keys(CLIPS)].join(', ')}`);
  process.exit(1);
}

const up = (url) => fetch(url).then((r) => r.ok, () => false);

async function startServer() {
  const url = `http://127.0.0.1:${PORT}/`;
  // A server left over from an earlier run would serve stale code; don't silently shoot that.
  if (await up(url)) throw new Error(`Port ${PORT} is already in use; stop whatever is serving ${url} first.`);
  const win = process.platform === 'win32';
  const vp = join(root, 'ui', 'node_modules', '.bin', win ? 'vp.cmd' : 'vp');
  const proc = spawn(vp, ['dev', '--mode', 'mock', '--host', '127.0.0.1', '--port', String(PORT), '--strictPort'], {
    cwd: join(root, 'ui'),
    stdio: ['ignore', 'pipe', 'inherit'],
    shell: win,
    // Its own process group, so stopServer() takes Vite's workers down with it.
    detached: !win,
  });
  for (let i = 0; i < 100; i++) {
    if (await up(url)) return { proc, url };
    await new Promise((r) => setTimeout(r, 300));
  }
  stopServer(proc);
  throw new Error('Dev server did not start');
}

function stopServer(proc) {
  if (process.platform === 'win32') spawn('taskkill', ['/pid', String(proc.pid), '/T', '/F'], { stdio: 'ignore' });
  else {
    try {
      process.kill(-proc.pid);
    } catch {
      /* already gone */
    }
  }
}

/** ffmpeg on PATH, else the build `playwright install` puts next to its browsers. */
function findFfmpeg() {
  const onPath = spawn('ffmpeg', ['-version'], { stdio: 'ignore' });
  const cache =
    process.env.PLAYWRIGHT_BROWSERS_PATH ||
    (process.platform === 'win32'
      ? join(process.env.LOCALAPPDATA ?? '', 'ms-playwright')
      : process.platform === 'darwin'
        ? join(homedir(), 'Library', 'Caches', 'ms-playwright')
        : join(homedir(), '.cache', 'ms-playwright'));
  const bundled = existsSync(cache)
    ? readdirSync(cache)
        .filter((d) => d.startsWith('ffmpeg-'))
        .sort()
        .reverse()
        .flatMap((d) => ['ffmpeg-linux', 'ffmpeg-mac', 'ffmpeg-win64.exe'].map((f) => join(cache, d, f)))
        .find((f) => existsSync(f))
    : undefined;
  return new Promise((resolve) => {
    onPath.on('error', () => resolve(bundled));
    onPath.on('exit', (code) => resolve(code === 0 ? 'ffmpeg' : bundled));
  });
}

async function openScene(browser, url, params, theme) {
  const ctx = await browser.newContext({ viewport: { width: WIDTH, height: HEIGHT }, deviceScaleFactor: 2 });
  const page = await ctx.newPage();
  await page.goto(`${url}?${new URLSearchParams({ ...params, theme })}`);
  await page.waitForSelector('#root *');
  await page.evaluate(() => document.fonts.ready);
  await page.waitForTimeout(600); // list render, search, selection scroll
  await page.mouse.move(WIDTH - 1, HEIGHT - 1);
  return { ctx, page };
}

function frameHtml(theme, dataUrl) {
  return `<!doctype html><meta charset="utf-8"><style>
    html,body{margin:0;background:transparent}
    .win{position:relative;width:${WIDTH}px;height:${HEIGHT}px;border-radius:10px;overflow:hidden}
    .win::after{content:"";position:absolute;inset:0;border-radius:10px;box-shadow:inset 0 0 0 1px ${THEMES[theme].hairline}}
    img{display:block;width:${WIDTH}px;height:${HEIGHT}px}
  </style><div class="win"><img src="${dataUrl}"></div>`;
}

async function toWebp(page, png) {
  const b64 = await page.evaluate(async (src) => {
    const img = new Image();
    img.src = src;
    await img.decode();
    const c = document.createElement('canvas');
    c.width = img.naturalWidth;
    c.height = img.naturalHeight;
    c.getContext('2d').drawImage(img, 0, 0);
    return c.toDataURL('image/webp', 0.95).split(',')[1];
  }, `data:image/png;base64,${png.toString('base64')}`);
  return Buffer.from(b64, 'base64');
}

async function save(name, data) {
  if (data) await writeFile(join(out, name), data);
  const pub = PUBLISHED.includes(name);
  if (pub) await copyFile(join(out, name), join(published, name));
  console.log(`${pub ? '* ' : '  '}${name}`);
}

async function recordClip(browser, url, clip, theme, ffmpeg, file) {
  const { ctx, page } = await openScene(browser, url, clip.params, theme);
  const enc = spawn(
    ffmpeg,
    ['-y', '-loglevel', 'error', '-f', 'image2pipe', '-c:v', 'mjpeg', '-framerate', String(FPS), '-i', 'pipe:0',
      '-c:v', 'libvpx-vp9', '-crf', '30', '-b:v', '0', '-pix_fmt', 'yuv420p', '-an', file],
    { stdio: ['pipe', 'ignore', 'inherit'] },
  );
  const done = new Promise((resolve, reject) => {
    enc.on('error', reject);
    enc.on('exit', (code) => (code === 0 ? resolve() : reject(new Error(`ffmpeg exited with ${code}`))));
  });
  // If ffmpeg dies mid-clip, stop feeding it and report that, rather than an unhandled rejection
  // that would skip the cleanup below and leave the dev server running.
  let failed = false;
  done.catch(() => (failed = true));
  for (const step of clip.steps) {
    if (failed) break;
    if (step.press) await page.keyboard.press(step.press);
    if (step.type) await page.keyboard.type(step.type);
    if (step.select !== undefined) {
      await page.evaluate((i) => document.querySelectorAll('[role="option"]')[i]?.click(), step.select);
    }
    await page.waitForTimeout(120); // let the search answer and the list settle
    const frame = await page.screenshot({ type: 'jpeg', quality: 92, clip: clip.crop });
    for (let i = 0; i < Math.max(1, Math.round(step.hold * FPS)); i++) {
      if (!enc.stdin.write(frame)) await new Promise((r) => enc.stdin.once('drain', r));
    }
  }
  enc.stdin.end();
  await done;
  await ctx.close();
}

await rm(out, { recursive: true, force: true });
await mkdir(out, { recursive: true });
await mkdir(published, { recursive: true });

const ffmpeg = clips.length ? await findFfmpeg() : undefined;
if (clips.length && !ffmpeg) throw new Error('No ffmpeg: run `pnpm --dir tools/screenshots exec playwright install chromium`, or put one on PATH.');

const { proc, url } = await startServer();
const browser = await chromium.launch();
try {
  const framer = await browser.newPage({ deviceScaleFactor: 2, viewport: { width: WIDTH, height: HEIGHT } });
  for (const scene of scenes) {
    const { params, crop } = SCENES[scene];
    for (const theme of Object.keys(THEMES)) {
      const { ctx, page } = await openScene(browser, url, params, theme);
      const raw = await page.screenshot(crop ? { clip: crop } : {});
      await ctx.close();
      let shot = raw;
      if (!crop) {
        await framer.setContent(frameHtml(theme, `data:image/png;base64,${raw.toString('base64')}`));
        await framer.evaluate(() => Promise.all([...document.images].map((i) => i.decode())));
        shot = await framer.screenshot({ omitBackground: true });
      }
      await save(`${scene}-${theme}.webp`, await toWebp(framer, shot));
    }
  }
  for (const name of clips) {
    for (const theme of Object.keys(THEMES)) {
      const file = `${name}-${theme}.webm`;
      await recordClip(browser, url, CLIPS[name], theme, ffmpeg, join(out, file));
      await save(file);
    }
  }
} finally {
  await browser.close();
  stopServer(proc);
}
