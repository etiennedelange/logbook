/**
 * Screenshots and short clips of the real app, against made-up sessions (demo.mjs), so no private session data
 * ever appears in them. It starts the app in a virtual display (Xvfb), drives it with the keyboard and mouse
 * (xdotool) the way a person would, and photographs the window (ImageMagick, ffmpeg for clips).
 *
 *   pnpm screenshots                       every scene and clip, both themes
 *   pnpm screenshots browse search-clip    only those
 *
 * Linux only; the dev container has what it needs (Xvfb, xdotool, ImageMagick with WebP, ffmpeg, a software
 * Vulkan driver), and the release build of the app must exist: `cargo build --release -p logbook`.
 *
 * Everything renders into tools/screenshots/out/ (gitignored). The PUBLISHED files are also copied to
 * docs/screenshots/, which the README and the marketing site show.
 *
 * The window is photographed at twice the size it has on screen (zoom 2 in a 2360x1520 window), so the shots
 * are sharp and match what the site expects: the layout is the 1180x760 one, drawn at 2x.
 */
import { spawn, spawnSync } from 'node:child_process';
import { copyFileSync, existsSync, mkdirSync, rmSync, unlinkSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { setTimeout as sleep } from 'node:timers/promises';
import { fileURLToPath } from 'node:url';
import { DELETED_FILE_ID, DELETED_SESSION, EDITED_FILE, writeDemoHome } from './demo.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, '..', '..');
const out = join(here, 'out');
const published = join(root, 'docs', 'screenshots');

// The default window: 1180x760 CSS pixels, drawn at 2x.
const WIDTH = 1180;
const HEIGHT = 760;
const SCALE = 2;
const SCREEN = '2400x1600x24';
const ORIGIN = { x: 20, y: 20 };

// Regions of the window, in CSS pixels. The list pane starts after the 220px sidebar and ends at 603.
const LIST_AND_DETAIL = { x: 220, y: 0, width: 960, height: 520 };
const LIST_ONLY = { x: 220, y: 0, width: 383, height: 240 };

/**
 * Steps to reach a scene, after the app has loaded (the first Down selects the first row):
 *   key: an xdotool key name · type: text · click: a point in CSS pixels · wait: seconds
 * `env` is extra environment for the app (see app.rs: LOGBOOK_DEV_*). `crop` cuts a region out of the window (no
 * window frame: the site frames those itself); without it the shot is the whole window. `history` adds 26 weeks.
 */
const SEARCH = [{ key: 'slash' }, { type: 'reminder' }, { wait: 0.6 }];
const SCENES = {
  browse: { steps: [{ key: 'Down' }] },
  search: { steps: [...SEARCH, { key: 'Down' }] },
  about: { steps: [{ key: 'Down' }, { type: '?' }] },
  // Two agents: Claude Code picked in the sidebar (Alt+2), and the session whose file it deleted (the 4th Claude one).
  agents: { steps: [{ key: 'alt+2' }, ...Array(4).fill({ key: 'Down' })] },
  // The Files tab of a Claude Code session: what it created and edited, and one file's edits.
  files: { steps: [{ key: 'alt+2' }, { key: 'Down' }], env: { LOGBOOK_DEV_TAB: 'files' } },
  'file-edits': { steps: [{ key: 'alt+2' }, { key: 'Down' }], env: { LOGBOOK_DEV_TAB: 'files', LOGBOOK_DEV_FILE: EDITED_FILE } },
  // The sidebar's Sources popover: each agent's folder, its sessions, and the kept Claude session.
  sources: { steps: [{ key: 'Down' }, { click: [53, 736] }] },
  // An update on offer: the toolbar pill and What's New with the release notes.
  'whats-new': { steps: [], env: { LOGBOOK_DEV_UPDATE: '1' } },
  // The site's search section: the matching row and the highlighted messages, legible at its size.
  'search-detail': { steps: [...SEARCH, { key: 'Down' }], crop: LIST_AND_DETAIL },
  // Phone-width stand-ins for the full window, whose text would be ~3px tall there.
  'search-narrow': { steps: SEARCH, crop: LIST_ONLY },
  'browse-narrow': { steps: [], crop: { ...LIST_ONLY, height: 462 } },
  // The Stats view over 26 weeks of generated history.
  stats: { steps: [{ type: '5' }, { wait: 1 }], history: true },
};

/**
 * Short WebM clips, recorded step by step so every frame is a settled render (no timing jitter). Each step changes
 * the UI, then holds that frame for `hold` seconds. Kept under five seconds and played once by the site, so it
 * needs no pause control (WCAG 2.2.2).
 */
const FPS = 25;
const CLIPS = {
  'search-clip': {
    crop: LIST_AND_DETAIL,
    steps: [
      { hold: 0.7 },
      { key: 'slash', hold: 0.3 },
      ...[...'reminder'].map((ch) => ({ type: ch, hold: 0.1 })),
      { hold: 0.7 },
      { key: 'Down', hold: 1.6 }, // open the match: the transcript shows the highlighted messages
    ],
  },
};

/** The files the README and the site show; the rest are for looking at after a change. */
const PUBLISHED = [
  'browse', 'agents', 'files', 'file-edits', 'search', 'search-detail', 'search-narrow', 'browse-narrow', 'whats-new', 'stats',
].flatMap((n) => [`${n}-dark.webp`, `${n}-light.webp`]).concat(['search-clip-dark.webm', 'search-clip-light.webm']);

// The hairline drawn around a whole-window shot: the app's own border colour per theme.
const THEMES = { dark: { hairline: 'rgba(255,255,255,.12)' }, light: { hairline: 'rgba(20,16,40,.14)' } };

const wanted = process.argv.slice(2);
const pick = (names) => names.filter((s) => wanted.length === 0 || wanted.includes(s));
const scenes = pick(Object.keys(SCENES));
const clips = pick(Object.keys(CLIPS));
if (scenes.length + clips.length === 0) {
  console.error(`No such scene. Scenes: ${[...Object.keys(SCENES), ...Object.keys(CLIPS)].join(', ')}`);
  process.exit(1);
}

const app = [join(root, 'target', 'release', 'logbook'), join(root, 'target', 'debug', 'logbook')].find((p) => existsSync(p));
if (!app) {
  console.error('No build of the app. Run: cargo build --release -p logbook');
  process.exit(1);
}
for (const tool of ['Xvfb', 'xdotool', 'import', 'convert']) {
  if (spawnSync('which', [tool]).status !== 0) {
    console.error(`${tool} is not installed (see .devcontainer/post-create.sh).`);
    process.exit(1);
  }
}
if (clips.length && spawnSync('which', ['ffmpeg']).status !== 0) {
  console.error('ffmpeg is not installed, which the clips need (see .devcontainer/post-create.sh).');
  process.exit(1);
}

const DISPLAY = `:${90 + (process.pid % 9)}`;
const css = (n) => n * SCALE;

function run(cmd, args, opts = {}) {
  const r = spawnSync(cmd, args, { env: { ...process.env, DISPLAY }, maxBuffer: 1 << 28, ...opts });
  if (r.status !== 0) throw new Error(`${cmd} ${args.join(' ')} failed: ${r.stderr?.toString() ?? ''}`);
  return r.stdout;
}

/** The app, started inside the demo home (its folders are relative paths there), until `stop()`. */
async function startApp(home, theme, env = {}) {
  writeFileSync(
    join(home, 'cache', 'gpui-settings.json'),
    JSON.stringify({ dark: theme === 'dark', zoom: SCALE, window: { x: ORIGIN.x, y: ORIGIN.y, w: WIDTH * SCALE, h: HEIGHT * SCALE, maximized: false } }),
  );
  const child = spawn(app, [], {
    cwd: home,
    stdio: 'ignore',
    env: {
      ...process.env,
      DISPLAY,
      COPILOT_HOME: join(home, 'copilot'),
      CLAUDE_CONFIG_DIR: join(home, 'claude'),
      LOGBOOK_CACHE_DIR: join(home, 'cache'),
      LOGBOOK_NO_UPDATE_CHECK: '1',
      ...env,
    },
  });
  run('xdotool', ['search', '--sync', '--name', '^Logbook$']);
  await sleep(4500); // the list loads, the first scan runs
  return () => child.kill();
}

const xdo = (...args) => run('xdotool', args);

async function perform(step) {
  if (step.key) xdo('key', '--clearmodifiers', step.key);
  if (step.type) xdo('type', '--delay', '60', step.type);
  if (step.click) {
    xdo('mousemove', String(ORIGIN.x + css(step.click[0])), String(ORIGIN.y + css(step.click[1])));
    await sleep(150);
    xdo('click', '1');
  }
  await sleep((step.wait ?? 0.7) * 1000);
}

/** The window (or a crop of it) as a PNG. */
function shot(crop) {
  const c = crop ?? { x: 0, y: 0, width: WIDTH, height: HEIGHT };
  return run('import', ['-window', 'root', '-crop', `${css(c.width)}x${css(c.height)}+${ORIGIN.x + css(c.x)}+${ORIGIN.y + css(c.y)}`, '+repage', 'png:-']);
}

/** PNG to WebP; a whole-window shot also gets rounded corners and a hairline, like the window has on screen. */
function toWebp(png, theme, whole) {
  const w = css(WIDTH);
  const h = css(HEIGHT);
  const r = 10 * SCALE;
  const args = ['png:-'];
  if (whole) {
    args.push(
      '(', '+clone', '-alpha', 'transparent', '-fill', 'white', '-draw', `roundrectangle 0,0,${w - 1},${h - 1},${r},${r}`, ')',
      '-alpha', 'set', '-compose', 'DstIn', '-composite',
      '(', '-size', `${w}x${h}`, 'xc:none', '-fill', 'none', '-stroke', THEMES[theme].hairline, '-strokewidth', '2', '-draw', `roundrectangle 1,1,${w - 2},${h - 2},${r},${r}`, ')',
      '-compose', 'Over', '-composite',
    );
  }
  args.push('-quality', '95', 'webp:-');
  return run('convert', args, { input: png });
}

function save(name, data) {
  if (data) writeFileSync(join(out, name), data);
  const pub = PUBLISHED.includes(name);
  if (pub) copyFileSync(join(out, name), join(published, name));
  console.log(`${pub ? '* ' : '  '}${name}`);
}

/** Records a clip: each step's settled frame, held for its time, encoded as VP9. */
async function recordClip(home, theme, clip, file) {
  const frames = join(out, `frames-${theme}`);
  rmSync(frames, { recursive: true, force: true });
  mkdirSync(frames, { recursive: true });
  const stop = await startApp(home, theme);
  const list = [];
  try {
    for (const [i, step] of clip.steps.entries()) {
      await perform({ ...step, wait: 0.15 });
      const name = `f${String(i).padStart(3, '0')}.png`;
      writeFileSync(join(frames, name), shot(clip.crop));
      list.push(`file '${name}'`, `duration ${step.hold}`);
    }
  } finally {
    stop();
  }
  // The concat demuxer drops the last duration unless its file is listed again.
  list.push(list[list.length - 2]);
  writeFileSync(join(frames, 'frames.txt'), `${list.join('\n')}\n`);
  run('ffmpeg', ['-y', '-loglevel', 'error', '-f', 'concat', '-i', 'frames.txt', '-vf', `fps=${FPS},scale=trunc(iw/2)*2:trunc(ih/2)*2,format=yuv420p`, '-c:v', 'libvpx-vp9', '-crf', '30', '-b:v', '0', '-an', file], { cwd: frames });
  rmSync(frames, { recursive: true, force: true });
}

rmSync(out, { recursive: true, force: true });
mkdirSync(out, { recursive: true });
mkdirSync(published, { recursive: true });

const xvfb = spawn('Xvfb', [DISPLAY, '-screen', '0', SCREEN], { stdio: 'ignore' });
await sleep(1000);
const homes = { plain: join(out, 'home'), history: join(out, 'home-history') };
try {
  const needed = [];
  if (scenes.some((s) => !SCENES[s].history) || clips.length) needed.push(['plain', false]);
  if (scenes.some((s) => SCENES[s].history)) needed.push(['history', true]);
  for (const [key, history] of needed) {
    writeDemoHome(homes[key], { history });
    // Index once, then delete one Claude session's file: the next start keeps its own copy, marked "source deleted".
    const stop = await startApp(homes[key], 'dark');
    stop();
    await sleep(500);
    const dirs = join(homes[key], 'claude', 'projects');
    const file = spawnSync('find', [dirs, '-name', `${DELETED_FILE_ID}.jsonl`]).stdout.toString().trim();
    if (file) unlinkSync(file);
  }

  for (const scene of scenes) {
    const { steps, crop, env, history } = SCENES[scene];
    for (const theme of Object.keys(THEMES)) {
      const stop = await startApp(history ? homes.history : homes.plain, theme, env);
      try {
        for (const step of steps) await perform(step);
        xdo('mousemove', String(ORIGIN.x + css(WIDTH) - 1), String(ORIGIN.y + css(HEIGHT) - 1));
        await sleep(400);
        save(`${scene}-${theme}.webp`, toWebp(shot(crop), theme, !crop));
      } finally {
        stop();
      }
    }
  }
  for (const name of clips) {
    for (const theme of Object.keys(THEMES)) {
      const file = `${name}-${theme}.webm`;
      await recordClip(homes.plain, theme, CLIPS[name], join(out, file));
      save(file);
    }
  }
} finally {
  xvfb.kill();
}
console.log(`The kept Claude session is "${DELETED_SESSION}".`);
