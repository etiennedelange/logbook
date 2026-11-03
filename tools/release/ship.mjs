#!/usr/bin/env node
/**
 * One command for a whole release (docs/RELEASING.md, "Cut a release"). Ported from notes.
 *
 *   pnpm release                the next patch after the newest tag (v0.2.0 -> 0.2.1)
 *   pnpm release 0.3.0          a given version; asks once, then does everything below
 *   pnpm release 0.3.0 --yes    without asking
 *   ... --no-follow             stop once the tag is pushed (the Cut release workflow does this)
 *
 * 1. Checks: no uncommitted changes to tracked files, gh signed in, the version is new, and the
 *    CHANGELOG has entries under [Unreleased].
 * 2. Branch release/<version> from origin/main: version.mjs sets all seven version fields, the
 *    [Unreleased] entries get a dated [version] heading; commit "Release <version>", push.
 * 3. PR "Release <version>" (label skip-changelog), squash auto-merge once CI is green.
 * 4. Follows the PR's CI run (`gh run watch`), waits for the merge, tags the merge commit
 *    v<version> and pushes the tag, which starts release.yml.
 * 5. Follows the Release workflow and prints the release page.
 *
 * Safe to run again after a failure or Ctrl+C: steps already done (branch pushed, PR open, PR merged,
 * tag pushed) are picked up where they are. A bare `pnpm release` whose newest tag has no GitHub
 * Release yet follows that tag's Release run instead of starting the next patch.
 * Needs git and gh (`gh auth login`).
 */
import { execFileSync, spawnSync } from 'node:child_process';
import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { createInterface } from 'node:readline/promises';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { CHANGELOG, changelogSection, dateUnreleased, hasNotes } from './changelog.mjs';
import { SEMVER, VERSION_FILES, readVersion, setVersion } from './version.mjs';

export { hasNotes };

const root = join(dirname(fileURLToPath(import.meta.url)), '..', '..');
const REPO = 'etiennedelange/logbook';
// The workflows behind the required checks (ui, rust (ubuntu-latest), rust (windows-latest)); keep in
// step with tools/release/github-settings.sh. Their runs on the PR are what auto-merge waits for.
const CI_WORKFLOWS = ['CI'];
// How often to look at the PR once CI is green.
const POLL_MS = 5_000;
// Auto-merge lands seconds after the checks pass; ten minutes without it means something else is
// blocking the merge (an unresolved review thread, say).
const MERGE_TIMEOUT_MS = 10 * 60_000;

/** -1, 0 or 1. Semver precedence, without build metadata. */
export function compareVersions(a, b) {
  const split = (v) => {
    const [core, pre] = v.split(/-(.*)/s);
    return { core: core.split('.').map(Number), pre: pre ? pre.split('.') : [] };
  };
  const x = split(a);
  const y = split(b);
  for (let i = 0; i < 3; i++) if (x.core[i] !== y.core[i]) return x.core[i] < y.core[i] ? -1 : 1;
  // A pre-release comes before its release.
  if (!x.pre.length || !y.pre.length) return x.pre.length === y.pre.length ? 0 : x.pre.length ? -1 : 1;
  for (let i = 0; i < Math.max(x.pre.length, y.pre.length); i++) {
    const [p, q] = [x.pre[i], y.pre[i]];
    if (p === undefined || q === undefined) return p === undefined ? -1 : 1;
    if (p === q) continue;
    const [pn, qn] = [/^\d+$/.test(p), /^\d+$/.test(q)];
    if (pn && qn) return Number(p) < Number(q) ? -1 : 1;
    if (pn !== qn) return pn ? -1 : 1;
    return p < q ? -1 : 1;
  }
  return 0;
}

/** The newest version among the `v*` tags in `tags`, without the v, or null if there's none. */
export function newestVersion(tags) {
  return (
    tags
      .filter((t) => /^v\d/.test(t))
      .map((t) => t.slice(1))
      .sort(compareVersions)
      .at(-1) ?? null
  );
}

/** Why `version` can't be released after the `v*` tags in `tags`, or null if it can. */
export function versionProblem(version, tags) {
  if (tags.includes(`v${version}`)) return `v${version} is already tagged`;
  const newest = newestVersion(tags);
  if (newest && compareVersions(version, newest) < 0) return `${version} is older than the newest tag, v${newest}`;
  return null;
}

/**
 * The version a bare `pnpm release` makes: the newest `v*` tag with its patch bumped, or the release
 * of a pre-release tag (v0.3.0-beta.1 -> 0.3.0). Null when nothing is tagged yet.
 */
export function nextPatch(tags) {
  const newest = newestVersion(tags);
  if (!newest) return null;
  const [core, pre] = newest.split(/-(.*)/s);
  if (pre !== undefined) return core;
  const [major, minor, patch] = core.split('.').map(Number);
  return `${major}.${minor}.${patch + 1}`;
}

function fail(message) {
  console.error(`\nrelease: ${message}`);
  process.exit(1);
}

const run = (cmd, args, opts = {}) => execFileSync(cmd, args, { cwd: root, encoding: 'utf8', ...opts }).trim();
const git = (...args) => run('git', args);
const gh = (...args) => run('gh', args);
/** Like `run`, but its output goes to the terminal (for commands that print progress). */
const show = (cmd, args) => spawnSync(cmd, args, { cwd: root, stdio: 'inherit' }).status === 0;
const step = (text) => console.log(`\n▸ ${text}`);
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

/** The release PR for `branch`, open or merged, or null. */
function findPr(branch) {
  const prs = JSON.parse(
    gh('pr', 'list', '-R', REPO, '--head', branch, '--state', 'all', '--json', 'number,state,url,headRefOid,mergeCommit'),
  );
  return prs.find((p) => p.state === 'MERGED') ?? prs.find((p) => p.state === 'OPEN') ?? null;
}

/** Whether GitHub has a Release for `tag`; the Release workflow publishes it. */
function hasRelease(tag) {
  try {
    gh('release', 'view', '-R', REPO, tag, '--json', 'tagName');
    return true;
  } catch {
    return false;
  }
}

/** The newest run of each of CI_WORKFLOWS for the PR's head commit, or null if GitHub hasn't started them all. */
async function ciRuns(pr, branch) {
  for (let i = 0; i < 20; i++) {
    const runs = JSON.parse(
      gh(
        'run',
        'list',
        '-R',
        REPO,
        '--branch',
        branch,
        '--commit',
        pr.headRefOid,
        '--event',
        'pull_request',
        '--json',
        'databaseId,workflowName',
      ),
    );
    const found = CI_WORKFLOWS.map((w) => runs.find((r) => r.workflowName === w));
    if (found.every(Boolean)) return found;
    await sleep(3000);
  }
  return null;
}

// Watching the Release run is only for the terminal; a workflow leaves it to its own run.
let follow = true;

async function main(argv) {
  const yes = argv.includes('--yes');
  follow = !argv.includes('--no-follow');
  const arg = argv.find((a) => !a.startsWith('--'));

  step('Checking');
  try {
    gh('auth', 'status');
  } catch {
    fail('gh is not signed in. Run `gh auth login` first.');
  }
  git('fetch', '--quiet', '--tags', 'origin');
  const tags = () => git('tag', '--list', 'v*').split('\n').filter(Boolean);
  if (!arg) {
    // A run stopped after pushing the tag: carry on with that release, not the next patch.
    const newest = newestVersion(tags());
    if (newest && !hasRelease(`v${newest}`)) {
      console.log(
        `v${newest} is tagged but not released yet, so following its Release run.` +
          `\nTo release a different version instead, give it: pnpm release <version>`,
      );
      return followRelease(`v${newest}`);
    }
  }
  const version = arg ? arg.replace(/^v/, '') : nextPatch(tags());
  if (!version) fail("nothing is tagged yet, so there's no patch to bump. Give a version: pnpm release 0.1.0");
  if (!SEMVER.test(version)) fail(`"${arg}" is not a version like 1.2.0 or 1.2.0-beta.1`);
  const tag = `v${version}`;
  const branch = `release/${version}`;
  if (git('ls-remote', '--tags', 'origin', `refs/tags/${tag}`)) {
    console.log(`${tag} is already pushed; nothing to do but follow its Release run.`);
    return followRelease(tag);
  }
  if (git('status', '--porcelain', '--untracked-files=no')) fail('there are uncommitted changes. Commit or stash them first.');
  let pr = findPr(branch);
  if (!pr) {
    const problem = versionProblem(version, tags());
    if (problem) fail(problem);
    if (!hasNotes(git('show', `origin/main:${CHANGELOG}`), version))
      fail(`${CHANGELOG} has no entries under [Unreleased]; the release notes would be empty.`);
  }
  console.log(`Version ${version} (now ${readVersion()} on this checkout); tag ${tag}; branch ${branch}.`);

  if (!yes && !pr) {
    console.log(
      `\nThis will commit "Release ${version}" on ${branch}, push it, open a PR that squash-merges itself when CI is green,` +
        `\nthen tag the merge commit ${tag} and push the tag, which builds and publishes the GitHub Release.`,
    );
    const rl = createInterface({ input: process.stdin, output: process.stdout });
    const answer = await rl.question('Continue? [y/N] ');
    rl.close();
    if (!/^y(es)?$/i.test(answer.trim())) fail('stopped; nothing was changed.');
  }

  if (!pr) {
    step(`Preparing ${branch}`);
    if (git('ls-remote', '--heads', 'origin', branch)) {
      // Pushed by an earlier run that stopped before the PR.
      git('switch', '--quiet', '-C', branch, `origin/${branch}`);
    } else {
      git('switch', '--quiet', '-C', branch, 'origin/main');
      setVersion(version);
      const changelog = join(root, CHANGELOG);
      writeFileSync(changelog, dateUnreleased(readFileSync(changelog, 'utf8'), version));
      git('add', ...VERSION_FILES, CHANGELOG);
      // Signs as configured (commit.gpgsign).
      if (!show('git', ['commit', '--quiet', '-m', `Release ${version}`]))
        fail('git commit failed (see above). Fix it and run this again.');
      git('push', '--quiet', '-u', 'origin', branch);
    }

    step('Opening the PR');
    const notes = changelogSection(readFileSync(join(root, CHANGELOG), 'utf8'), version) ?? '';
    gh(
      'pr',
      'create',
      '-R',
      REPO,
      '--base',
      'main',
      '--head',
      branch,
      '--title',
      `Release ${version}`,
      '--label',
      'skip-changelog',
      '--body',
      `Sets the version to ${version} and dates the CHANGELOG's [Unreleased] entries. After the merge, \`pnpm release\` tags it ${tag}.\n\n${notes}`,
    );
    pr = findPr(branch);
  }
  console.log(pr.url);

  if (pr.state !== 'MERGED') {
    // Squash is the only merge method; the head branch is deleted after the merge.
    try {
      gh('pr', 'merge', '-R', REPO, String(pr.number), '--squash', '--auto');
    } catch {
      fail(
        `couldn't turn on auto-merge for ${pr.url}.\nRun tools/release/github-settings.sh once (it allows auto-merge and sets the required checks), then run this again.`,
      );
    }
    step('Waiting for CI (Ctrl+C is safe; run the same command again to carry on)');
    const runs =
      (await ciRuns(pr, branch)) ?? fail(`no ${CI_WORKFLOWS.join(' and ')} runs for ${pr.url} yet. Check the PR, then run this again.`);
    // One after the other; a run that has already finished returns straight away.
    for (const r of runs) {
      console.log(`\n${r.workflowName}:`);
      if (!show('gh', ['run', 'watch', '-R', REPO, String(r.databaseId), '--interval', '10', '--exit-status']))
        fail(`CI failed (${r.workflowName}): ${pr.url}\nFix it on ${branch}, then run this again.`);
    }

    step('Waiting for the merge');
    const deadline = Date.now() + MERGE_TIMEOUT_MS;
    for (;;) {
      pr = findPr(branch) ?? fail(`the PR for ${branch} was closed without merging.`);
      if (pr.state === 'MERGED') break;
      if (Date.now() > deadline) fail(`CI passed but the PR isn't merged after ten minutes: ${pr.url}`);
      process.stdout.write('.');
      await sleep(POLL_MS);
    }
    console.log(' merged.');
  }

  step(`Tagging ${tag}`);
  const commit = pr.mergeCommit?.oid ?? fail('GitHub reports no merge commit for the PR.');
  git('fetch', '--quiet', 'origin', 'main');
  if (spawnSync('git', ['merge-base', '--is-ancestor', commit, 'origin/main'], { cwd: root }).status !== 0)
    fail(`the merge commit ${commit.slice(0, 7)} is not on main.`);
  console.log(git('log', '-1', '--format=%h %s', commit));
  git('tag', tag, commit);
  git('push', '--quiet', 'origin', tag);
  // Back to main, without the release branch (deleted on GitHub after the merge).
  git('switch', '--quiet', 'main');
  git('merge', '--quiet', '--ff-only', 'origin/main');
  // Only there if this run (or an earlier one on this checkout) made it: a run resumed after the merge,
  // such as a re-run of Cut release on a fresh checkout, never had it.
  if (git('branch', '--list', branch)) git('branch', '-D', branch);

  await followRelease(tag);
}

async function followRelease(tag) {
  if (!follow) {
    console.log(`\n${tag} is pushed. The Release workflow builds and publishes it: https://github.com/${REPO}/actions/workflows/release.yml`);
    return;
  }
  step('Following the Release workflow');
  let id = '';
  for (let i = 0; i < 20 && !id; i++) {
    id = gh('run', 'list', '-R', REPO, '--workflow', 'release.yml', '--branch', tag, '--limit', '1', '--json', 'databaseId', '-q', '.[0].databaseId');
    if (!id) await sleep(3000);
  }
  if (!id) fail(`no Release run for ${tag} yet. See https://github.com/${REPO}/actions/workflows/release.yml`);
  if (!show('gh', ['run', 'watch', '-R', REPO, id, '--interval', '10', '--exit-status']))
    fail(
      `the Release run failed: https://github.com/${REPO}/actions/runs/${id}\nSee docs/RELEASING.md: rerun it, or fix the cause in a new patch version.`,
    );
  console.log(`\nReleased: https://github.com/${REPO}/releases/tag/${tag}`);
}

const isMain =
  import.meta.main ?? (!!process.argv[1] && pathToFileURL(resolve(process.argv[1])).href === import.meta.url);
if (isMain) await main(process.argv.slice(2));
