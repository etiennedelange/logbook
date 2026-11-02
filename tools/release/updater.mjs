/**
 * latest.json for the in-app updater, as notes and gitr make it. Dependency-free.
 *
 *   node tools/release/updater.mjs manifest 0.5.0 <setup.exe> <setup.exe.sig>   print latest.json
 *
 * tauri-plugin-updater reads it from releases/latest/download/latest.json (plugins.updater in
 * src-tauri/tauri.conf.json). The installer's `url` is the release's plain download link. tauri-action
 * wrote an api.github.com asset URL there instead, which installed copies fetch without a token, so
 * past GitHub's 60 unauthenticated API requests an hour per IP address "Update now" failed with 403.
 */
import { readFileSync } from 'node:fs';
import { basename, dirname, join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { CHANGELOG, changelogSection } from './changelog.mjs';

const root = join(dirname(fileURLToPath(import.meta.url)), '..', '..');

/** Where the Release workflow uploads the installer; the updater downloads it from there. */
export const RELEASES = 'https://github.com/etiennedelange/logbook/releases';

/**
 * latest.json. `signature` is the .sig file `tauri build` writes next to the installer; `notes` is the
 * CHANGELOG section, which Logbook shows as What's New.
 */
export function updaterManifest({ version, notes, exe, signature, date = new Date() }) {
  return {
    version,
    notes: notes ?? '',
    pub_date: date.toISOString(),
    platforms: {
      'windows-x86_64': {
        url: `${RELEASES}/download/v${version}/${encodeURIComponent(basename(exe))}`,
        signature: signature.trim(),
      },
    },
  };
}

function main([command, version, exe, sig]) {
  if (command !== 'manifest' || !version || !exe || !sig) {
    console.error('usage: node tools/release/updater.mjs manifest <x.y.z> <setup.exe> <setup.exe.sig>');
    process.exit(1);
  }
  version = version.replace(/^v/, '');
  const notes = changelogSection(readFileSync(join(root, CHANGELOG), 'utf8'), version);
  if (!notes) {
    console.error(`${CHANGELOG} has no entries under "## [${version}]"`);
    process.exit(1);
  }
  const manifest = updaterManifest({ version, notes, exe, signature: readFileSync(sig, 'utf8') });
  console.log(JSON.stringify(manifest, null, 2));
}

const isMain =
  import.meta.main ?? (!!process.argv[1] && pathToFileURL(resolve(process.argv[1])).href === import.meta.url);
if (isMain) main(process.argv.slice(2));
