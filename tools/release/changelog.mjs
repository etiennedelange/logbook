/**
 * CHANGELOG.md helpers for releases. Dependency-free, so it runs in CI before `pnpm install`.
 *
 *   node tools/release/changelog.mjs notes 0.2.0   print that version's section (the release notes)
 *
 * ship.mjs (`pnpm release`) imports the rest: it dates the [Unreleased] entries as the new version.
 */
import { readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

export const CHANGELOG = 'CHANGELOG.md';
const root = join(dirname(fileURLToPath(import.meta.url)), '..', '..');

const escape = (s) => s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');

/** The lines under "## [version]" up to the next "## " heading, trimmed; null if there's no such heading. */
export function changelogSection(text, version) {
  const lines = text.split(/\r?\n/);
  const heading = new RegExp(`^## \\[${escape(version)}\\]`);
  const start = lines.findIndex((l) => heading.test(l));
  if (start < 0) return null;
  const end = lines.findIndex((l, i) => i > start && l.startsWith('## '));
  return lines
    .slice(start + 1, end < 0 ? undefined : end)
    .join('\n')
    .trim();
}

/** Whether there's anything under [Unreleased], or already a [version] section with entries. */
export function hasNotes(text, version) {
  const notes = (s) => !!s && /^\s*[-*] /m.test(s);
  return notes(changelogSection(text, version)) || notes(changelogSection(text, 'Unreleased'));
}

/**
 * The CHANGELOG with the [Unreleased] entries moved under "## [version] - date", and a fresh, empty
 * [Unreleased] above them. Unchanged if that version already has a section (an earlier run made it).
 */
export function dateUnreleased(text, version, date = new Date()) {
  if (changelogSection(text, version) !== null) return text;
  if (!/^## \[Unreleased\]/m.test(text)) throw new Error(`${CHANGELOG} has no "## [Unreleased]" heading`);
  const day = date.toISOString().slice(0, 10);
  return text.replace(/^## \[Unreleased\]/m, `## [Unreleased]\n\n## [${version}] - ${day}`);
}

function main([command, version]) {
  if (command !== 'notes' || !version) {
    console.error('Usage: changelog.mjs notes <x.y.z>');
    process.exit(1);
  }
  const section = changelogSection(readFileSync(join(root, CHANGELOG), 'utf8'), version.replace(/^v/, ''));
  if (!section) {
    console.error(`${CHANGELOG} has no entries under "## [${version}]"`);
    process.exit(1);
  }
  console.log(section);
}

const isMain =
  import.meta.main ?? (!!process.argv[1] && pathToFileURL(resolve(process.argv[1])).href === import.meta.url);
if (isMain) main(process.argv.slice(2));
