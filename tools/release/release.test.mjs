// pnpm test:tools (node --test)
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { changelogSection, dateUnreleased, hasNotes } from './changelog.mjs';
import { compareVersions, newestVersion, nextPatch, versionProblem } from './ship.mjs';
import { updaterManifest } from './updater.mjs';

test('compareVersions follows semver precedence', () => {
  const ordered = [
    '0.1.0',
    '0.1.1',
    '0.2.0-alpha',
    '0.2.0-alpha.1',
    '0.2.0-alpha.beta',
    '0.2.0-beta.2',
    '0.2.0-beta.11',
    '0.2.0',
    '0.10.0',
    '1.0.0',
  ];
  for (let i = 0; i < ordered.length - 1; i++) {
    assert.equal(compareVersions(ordered[i], ordered[i + 1]), -1, `${ordered[i]} < ${ordered[i + 1]}`);
    assert.equal(compareVersions(ordered[i + 1], ordered[i]), 1, `${ordered[i + 1]} > ${ordered[i]}`);
  }
  assert.equal(compareVersions('1.2.3', '1.2.3'), 0);
});

test('versionProblem refuses a tagged or older version', () => {
  const tags = ['v0.1.0', 'v0.2.0', 'not-a-version'];
  assert.equal(versionProblem('0.2.1', tags), null);
  assert.equal(versionProblem('0.3.0-beta.1', tags), null);
  assert.equal(versionProblem('0.2.0', tags), 'v0.2.0 is already tagged');
  assert.equal(versionProblem('0.1.5', tags), '0.1.5 is older than the newest tag, v0.2.0');
  assert.equal(versionProblem('0.1.0', []), null);
});

test('newestVersion picks the highest v* tag', () => {
  assert.equal(newestVersion(['v0.7.5', 'v0.10.0', 'v0.7.10', 'not-a-version']), '0.10.0');
  assert.equal(newestVersion(['v0.8.0-beta.1', 'v0.7.5']), '0.8.0-beta.1');
  assert.equal(newestVersion([]), null);
});

test("nextPatch bumps the newest tag's patch", () => {
  assert.equal(nextPatch(['v0.6.0', 'v0.10.0', 'v0.7.0']), '0.10.1');
  // A pre-release is followed by its release.
  assert.equal(nextPatch(['v0.7.0', 'v0.8.0-beta.1']), '0.8.0');
  assert.equal(nextPatch([]), null);
});

test('hasNotes needs entries under [Unreleased] or the version', () => {
  const empty = '# Changelog\n\n## [Unreleased]\n\n## [0.1.0] - 2026-10-11\n\n### Added\n- Old\n';
  assert.equal(hasNotes(empty, '0.1.1'), false);
  assert.equal(hasNotes(empty.replace('## [Unreleased]\n', '## [Unreleased]\n\n### Fixed\n- A fix\n'), '0.1.1'), true);
  // An earlier run already dated the section.
  assert.equal(hasNotes('## [Unreleased]\n\n## [0.1.1] - 2026-10-12\n\n- A fix\n', '0.1.1'), true);
  // Headings alone are not notes.
  assert.equal(hasNotes('## [Unreleased]\n\n### Added\n\n## [0.1.0]\n- Old\n', '0.1.1'), false);
});

test('changelogSection stops at the next version', () => {
  const text = '# Changelog\n\n## [Unreleased]\n\n## [0.5.0] - 2026-10-12\n\n### Added\n- Search\n\n## [0.4.0] - 2026-09-26\n- Old';
  assert.equal(changelogSection(text, '0.5.0'), '### Added\n- Search');
  assert.equal(changelogSection(text, 'Unreleased'), '');
  assert.equal(changelogSection(text, '9.9.9'), null);
});

test('dateUnreleased moves the entries under a dated heading', () => {
  const text = '# Changelog\n\n## [Unreleased]\n\n### Added\n- Search\n\n## [0.1.0] - 2026-10-01\n- Old\n';
  const dated = dateUnreleased(text, '0.2.0', new Date('2026-11-02T12:00:00Z'));
  assert.equal(changelogSection(dated, '0.2.0'), '### Added\n- Search');
  assert.equal(changelogSection(dated, 'Unreleased'), '');
  assert.match(dated, /^## \[0\.2\.0\] - 2026-11-02$/m);
  // Running it again leaves it alone.
  assert.equal(dateUnreleased(dated, '0.2.0'), dated);
  assert.throws(() => dateUnreleased('# Changelog\n', '0.2.0'), /no "## \[Unreleased\]" heading/);
});

test('updaterManifest points the updater at the release download link', () => {
  const date = new Date('2026-11-03T10:00:00Z');
  const manifest = updaterManifest({
    version: '0.5.0',
    notes: '### Fixed\n- x',
    exe: 'target/release/bundle/nsis/Logbook_0.5.0_x64-setup.exe',
    signature: 'c2ln\n',
    date,
  });
  assert.deepEqual(manifest, {
    version: '0.5.0',
    notes: '### Fixed\n- x',
    pub_date: '2026-11-03T10:00:00.000Z',
    platforms: {
      'windows-x86_64': {
        url: 'https://github.com/etiennedelange/logbook/releases/download/v0.5.0/Logbook_0.5.0_x64-setup.exe',
        signature: 'c2ln',
      },
    },
  });
});
