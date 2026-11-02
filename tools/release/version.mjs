/**
 * The app's version lives in seven fields across five files. This keeps them equal.
 *
 *   node tools/release/version.mjs 0.2.0         set an exact version
 *   node tools/release/version.mjs patch         or minor, major: bump from the current one
 *   node tools/release/version.mjs --check       fail unless all seven agree
 *   node tools/release/version.mjs --check v0.2.0  ...and agree with this tag
 *
 * src-tauri/tauri.conf.json is the source of truth: the installer is named from it.
 * Dependency-free, so it runs in CI before `pnpm install`.
 */
import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..', '..');
const path = (p) => join(root, p);
const SEMVER = /^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$/;

/** Each target: a file, how to read its version, and how to write one. */
const json = (file) => ({
  file,
  read: (t) => JSON.parse(t).version,
  write: (t, v) => t.replace(/("version"\s*:\s*)"[^"]*"/, `$1"${v}"`),
});
const cargoToml = (file) => ({
  file,
  read: (t) => /^version\s*=\s*"([^"]*)"/m.exec(t)?.[1],
  write: (t, v) => t.replace(/^(version\s*=\s*)"[^"]*"/m, `$1"${v}"`),
});
// Cargo.lock lists the two workspace crates as `name = "x"` then `version = "y"`.
const lockEntry = (name) => ({
  file: 'Cargo.lock',
  label: `Cargo.lock (${name})`,
  read: (t) => new RegExp(`name = "${name}"\\r?\\nversion = "([^"]*)"`).exec(t)?.[1],
  write: (t, v) => t.replace(new RegExp(`(name = "${name}"\\r?\\nversion = )"[^"]*"`), `$1"${v}"`),
});

const targets = [
  json('src-tauri/tauri.conf.json'),
  json('package.json'),
  json('ui/package.json'),
  cargoToml('src-tauri/Cargo.toml'),
  cargoToml('core/Cargo.toml'),
  lockEntry('logbook'),
  lockEntry('logbook-core'),
];

const current = () => targets.map((t) => ({ label: t.label ?? t.file, version: t.read(readFileSync(path(t.file), 'utf8')) }));
const fail = (msg) => {
  console.error(msg);
  process.exit(1);
};

const [arg, tag] = process.argv.slice(2);
if (!arg) fail('Usage: version.mjs <x.y.z|patch|minor|major> | --check [vX.Y.Z]');

if (arg === '--check') {
  const found = current();
  const expected = tag ? tag.replace(/^v/, '') : found[0].version;
  const bad = found.filter((f) => f.version !== expected);
  if (!SEMVER.test(expected)) fail(`"${expected}" is not a semantic version.`);
  if (bad.length) {
    fail(`Expected ${expected}, but:\n${bad.map((b) => `  ${b.label}: ${b.version ?? 'not found'}`).join('\n')}`);
  }
  console.log(`All ${found.length} version fields are ${expected}.`);
} else {
  const [base] = current();
  let next = arg;
  if (['major', 'minor', 'patch'].includes(arg)) {
    const [maj, min, pat] = base.version.split('-')[0].split('.').map(Number);
    next = arg === 'major' ? `${maj + 1}.0.0` : arg === 'minor' ? `${maj}.${min + 1}.0` : `${maj}.${min}.${pat + 1}`;
  }
  if (!SEMVER.test(next)) fail(`"${next}" is not a semantic version.`);
  for (const t of targets) {
    const text = readFileSync(path(t.file), 'utf8');
    const updated = t.write(text, next);
    if (t.read(updated) !== next) fail(`Could not update ${t.label ?? t.file}.`);
    writeFileSync(path(t.file), updated);
  }
  console.log(`${base.version} -> ${next}\nNext: commit, push, then tag v${next} (see docs/RELEASING.md).`);
}
