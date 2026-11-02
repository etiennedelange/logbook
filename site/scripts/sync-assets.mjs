// The screenshots live in docs/screenshots (one source for the README and this site).
// Copy them into public/ before dev and build; public/screenshots is gitignored.
import { cp, mkdir, readFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..', '..');
const dest = join(root, 'site', 'public', 'screenshots');
await mkdir(dest, { recursive: true });
await cp(join(root, 'docs', 'screenshots'), dest, { recursive: true });

// The app version, for the download link: src-tauri/tauri.conf.json is the one the installer is named from.
const { version } = JSON.parse(await readFile(join(root, 'src-tauri', 'tauri.conf.json'), 'utf8'));
await mkdir(join(root, 'site', 'src', 'generated'), { recursive: true });
await import('node:fs/promises').then((fs) =>
  fs.writeFile(join(root, 'site', 'src', 'generated', 'version.json'), JSON.stringify({ version }) + '\n'),
);
