/**
 * The marketing site's raster assets: og.png (1200x630) and the favicon PNGs.
 *
 *   pnpm site-assets
 *
 * Needs docs/screenshots/browse-dark.webp (run `pnpm screenshots` first) and the site's installed
 * font packages. Renders with the real Plex Sans and JetBrains Mono files, never the machine's fonts.
 */
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { dirname, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { chromium } from 'playwright';

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, '..', '..');
const site = join(root, 'site');
const require = createRequire(join(site, 'package.json'));
const font = (pkg, file) => pathToFileURL(join(dirname(require.resolve(`${pkg}/package.json`)), 'files', file)).href;
const sans = font('@fontsource-variable/ibm-plex-sans', 'ibm-plex-sans-latin-wght-normal.woff2');
const mono = font('@fontsource-variable/jetbrains-mono', 'jetbrains-mono-latin-wght-normal.woff2');

// Nord ground and the violet signal, from the brand tokens in site/src/styles/brand.css.
const c = { bg: '#2e3440', elevated: '#3b4252', border: '#4c566a', fg: '#eceff4', muted: '#d8dee9', signal: '#ceb0f2' };

const shot = `data:image/webp;base64,${(await readFile(join(root, 'docs', 'screenshots', 'browse-dark.webp'))).toString('base64')}`;
const icon = `data:image/svg+xml;base64,${(await readFile(join(site, 'public', 'favicon.svg'))).toString('base64')}`;

const og = `<!doctype html><meta charset="utf-8"><style>
  @font-face{font-family:Plex;src:url(${sans});font-weight:100 700}
  @font-face{font-family:Mono;src:url(${mono});font-weight:100 800}
  *{box-sizing:border-box}
  body{margin:0;width:1200px;height:630px;background:${c.bg};color:${c.fg};font-family:Plex;position:relative;overflow:hidden}
  .copy{position:absolute;left:72px;top:72px;width:520px}
  h1{font-weight:600;font-size:60px;line-height:1.08;letter-spacing:-.02em;margin:0;text-wrap:balance}
  .sub{font-size:24px;line-height:1.4;color:${c.muted};margin:24px 0 0}
  .brand{position:absolute;left:72px;bottom:64px;display:flex;align-items:center;gap:14px;font:700 26px Mono}
  .brand img{width:44px;height:44px}
  .shot{position:absolute;left:640px;top:96px;width:900px;border:1px solid ${c.border};border-radius:12px;
    box-shadow:0 24px 60px rgba(0,0,0,.4)}
</style><div class="copy"><h1>Find the agent session you half remember</h1>
<p class="sub">A Windows app for the GitHub Copilot CLI and Claude Code</p></div>
<div class="brand"><img src="${icon}">logbook</div><img class="shot" src="${shot}">`;

const browser = await chromium.launch();
try {
  const p = await browser.newPage({ viewport: { width: 1200, height: 630 } });
  // A file:// page, so it may load the font files; about:blank from setContent may not.
  const page = join(here, 'out', 'og.html');
  await mkdir(dirname(page), { recursive: true });
  await writeFile(page, og);
  await p.goto(pathToFileURL(page).href);
  await p.evaluate(() => document.fonts.ready);
  const faces = await p.evaluate(() => [...document.fonts].filter((f) => f.status === 'loaded').map((f) => f.family));
  if (!faces.includes('Plex') || !faces.includes('Mono')) throw new Error(`Fonts did not load: ${faces}`);
  await p.evaluate(() => Promise.all([...document.images].map((i) => i.decode())));
  await p.screenshot({ path: join(site, 'public', 'og.png') });

  for (const [name, size, bg] of [['favicon-32', 32, null], ['apple-touch-icon', 180, '#ffffff']]) {
    const q = await browser.newPage({ viewport: { width: size, height: size } });
    await q.setContent(`<body style="margin:0;background:${bg ?? 'transparent'}"><img src="${icon}" width="${size}" height="${size}"></body>`);
    await q.evaluate(() => Promise.all([...document.images].map((i) => i.decode())));
    await writeFile(join(site, 'public', `${name}.png`), await q.screenshot({ omitBackground: !bg }));
  }
  console.log('wrote site/public/og.png, favicon-32.png, apple-touch-icon.png');
} finally {
  await browser.close();
}
