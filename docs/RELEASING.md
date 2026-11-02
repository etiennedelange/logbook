# Releasing

A release is one command, or one button on GitHub. Everything after it is automated. Ported from the notes
repo's release flow.

## Day to day

Add a line under `[Unreleased]` in `CHANGELOG.md`, in the same PR, for anything a user would notice. Those
lines become the top of the release notes. Label the PR `feature` or `bug` to sort it in GitHub's list of
merged PRs below them (`.github/release.yml`), or `skip-changelog` to leave it out. Squash and merge, so
the PR title becomes the commit subject on `main` and a line in that list.

## Cut a release

```bash
pnpm screenshots      # only if the UI changed since the last release; commit them first
pnpm release          # the next patch: v0.2.0 -> 0.2.1
pnpm release 0.3.0    # any other version (the first release needs one: pnpm release 0.1.0)
```

`tools/release/ship.mjs` checks first: no uncommitted changes to tracked files, `gh` signed in, the version
not tagged yet and not older than the newest tag, and entries under the CHANGELOG's `[Unreleased]`. The
question before anything happens shows which version it picked. After you confirm, it:

1. Branches `release/<version>` from `origin/main`, sets all seven version fields (`version.mjs`), dates
   the `[Unreleased]` entries as `## [<version>] - <date>`, commits "Release <version>" (signed, if your
   git signs commits) and pushes.
2. Opens the PR "Release <version>" with the `skip-changelog` label and turns on squash auto-merge.
3. Follows the PR's CI run live with `gh run watch`, waits for the auto-merge, tags the merge commit
   `v<version>`, pushes the tag, and switches you back to an up-to-date `main`.
4. Follows the Release workflow and prints the release page.

Ctrl+C is safe at any point. Run the same command again and it carries on from where it stopped,
including after the tag is pushed: if the newest tag has no GitHub Release yet, a bare `pnpm release`
follows that tag's Release run instead of starting the next patch. `--yes` skips the question. If CI fails,
fix it on the release branch and run it again.

### Cut a release from GitHub

**Actions → Cut release → Run workflow** (`cut-release.yml`) does the same without your machine, so it
also works from a phone or a Claude session. Leave the version empty for the next patch. It runs
`ship.mjs --yes --no-follow` on `main`, so the checks, the resuming and the PR are the same; it stops once
the tag is pushed, and the Release run takes over. The release commit is authored by whoever started the
run (unsigned, but it's squashed into `main` anyway).

It pushes and opens the PR with the `RELEASE_TOKEN` secret, a fine-grained personal access token limited
to this repo, with **Contents**, **Pull requests** and **Issues** set to read and write (Issues for the
`skip-changelog` label). The job's own `GITHUB_TOKEN` can't do this: a PR or tag it creates starts no
workflows, so CI would never report on the PR and `release.yml` would never run. The token expires after
a year at most; renew it under **Settings → Developer settings → Fine-grained tokens** and run
`gh secret set RELEASE_TOKEN` again. Until then the run fails at its first step.

### What the tag starts

1. **`release.yml`** checks that the tag is on `main` and agrees with all seven version fields, and that the
   updater key is set. It builds the NSIS installer on Windows and publishes the GitHub Release with
   `Logbook_<version>_x64-setup.exe`, its updater signature (`.sig`), `SHA256SUMS.txt` and `latest.json` for the in-app
   updater (below). The notes are the CHANGELOG's section for the version, followed by GitHub's list of
   merged PRs.
2. It then starts **`site-rebuild.yml`**, which calls the Workers Builds deploy hook to rebuild the site.
   The site's download button points at `releases/download/v<version>/Logbook_<version>_x64-setup.exe`,
   so it only works once the release exists. (A release made with the job's own token doesn't fire the
   `release: published` event that `site-rebuild.yml` also listens for. Workers Builds also deploys the
   site on pushes to `main` that touch it, through its GitHub link.)

A version with a pre-release part (`0.3.0-rc.1`) is published as a pre-release and doesn't rebuild the
site. If the Release run fails, fix the cause and rerun it from the Actions tab; if the fix needs code,
release a new patch version instead.

## In-app updater

`tauri-plugin-updater` reads
`https://github.com/etiennedelange/logbook/releases/latest/download/latest.json` (`plugins.updater` in
`src-tauri/tauri.conf.json`). The Release run writes that file and uploads it with each release. Its `notes`
are the CHANGELOG section alone, which is what Logbook shows as **What's New**.

The installer's `url` in it is the release's download link
(`github.com/.../releases/download/v<version>/Logbook_<version>_x64-setup.exe`), written by
`tools/release/updater.mjs` as notes and gitr do. Releases up to 0.4.0 were published by `tauri-action`,
whose `latest.json` points at the REST API (`api.github.com/.../assets/<id>`) instead. Installed copies
download without a token, and GitHub allows 60 unauthenticated API requests an hour per IP address and
then answers 403, so **Update now** could fail behind a shared address.

- Logbook checks 5 seconds after it starts and then every hour until it finds an update. It's often left
  open for days, and a single check would never see a later release. A failed check is only logged.
  **About → Check for updates** checks right away and says what happened.
- An update shows as **Update** in the toolbar (**Update to x.y.z** when the window is wide). That opens **What's New** with the release notes.
  **Update now** downloads the installer and checks its signature against `plugins.updater.pubkey`
  (`src-tauri/src/update.rs`). It then runs the installer in passive mode (a progress bar, no
  questions). The installer closes Logbook and starts the new version.
- Before installing, the UI keeps the notes in `localStorage`. On its first start the new version shows
  them once as "You're now on Logbook x.y.z" (`ui/src/update.ts`).
- `latest.json` holds only the newest release, so an update that skips versions shows the newest notes
  alone. **Release page** in the dialog links to the rest.
- A pre-release never reaches installed copies, because GitHub's *latest* skips pre-releases. To try the
  updater end to end, release two normal patch versions.
- 0.1.1 and earlier have no updater. They need the next installer downloaded once by hand, and from then
  on they update themselves.
- `pnpm mock:web` with `?update` in the URL offers a fake 0.2.0. `?whatsnew` opens its dialog, and
  `?whatsnew=installed` shows the dialog as it looks after an update.

### The updater signing key

`bundle.createUpdaterArtifacts` makes `tauri build` sign the installer with a minisign key, and every
installed copy checks updates against the public half in `tauri.conf.json`. This is not code signing, so
SmartScreen still warns on first install. It does stop anyone who could swap a release asset from
pushing their own installer to every copy.

Once, on your own machine:

```sh
pnpm tauri signer generate -w ~/.tauri/logbook-updater.key   # asks for a password
gh secret set TAURI_SIGNING_PRIVATE_KEY < ~/.tauri/logbook-updater.key
gh secret set TAURI_SIGNING_PRIVATE_KEY_PASSWORD               # paste the password
```

Put the public key (`~/.tauri/logbook-updater.key.pub`, one line) into `plugins.updater.pubkey` in
`src-tauri/tauri.conf.json`, replacing `REPLACE_WITH_THE_PUBLIC_KEY`, and merge that before the next release.
The release job fails early while either is missing.

**Back up the private key and its password** outside this machine, for example in a password manager.
Without them no installed copy can be updated again, and every user would have to download a new installer
by hand. Changing the key works the same way, because copies that trust the old key reject updates signed
with the new one.

A local `pnpm tauri build` needs the key in `TAURI_SIGNING_PRIVATE_KEY`, or
`--config src-tauri/tauri.no-updater.conf.json` to build without the updater signature.

## The version

`src-tauri/tauri.conf.json` is the source of truth, because the installer is named from it. The version also
lives in `package.json`, `ui/package.json`, `src-tauri/Cargo.toml`, `core/Cargo.toml` and `Cargo.lock`
(two entries). `tools/release/version.mjs` writes all seven; `--check` fails if any disagree. `pnpm release`
sets them for you; `version.mjs` on its own is for checking, or for a version change outside a release.

## One-time setup

| What | Where |
|------|-------|
| Repository settings: squash merge only, auto-merge, head branches deleted after merging, rulesets for `main` (PRs only, CI must pass) and `v*` tags (can't be moved or deleted), and the release-notes labels | `tools/release/github-settings.sh`, once after `gh auth login`, and again after changing it. Its required checks are CI's job names, so rename them together |
| `RELEASE_TOKEN` for the Cut release workflow | fine-grained token, see above; `gh secret set RELEASE_TOKEN` |
| Worker `logbook-site` (static assets, `site/wrangler.jsonc`), linked to this repo | first `wrangler deploy` from `site/` creates it; then the Worker's Settings > Build > Connect |
| Build settings: root directory `site`, build command `pnpm build`, deploy command `npx wrangler deploy`; watch paths `site/*` and `docs/screenshots/*`, so a version bump doesn't publish a download link before its release | the Worker's Settings > Build |
| Build variable `SITE_URL` (canonical and OG URLs), now `https://logbook-site.etiennedelange.workers.dev` | the Worker's Settings > Build > Variables and secrets |
| Deploy hook for `main`, saved as the `CLOUDFLARE_DEPLOY_HOOK` secret | the Worker's Settings > Build > Deploy Hooks, then `gh secret set CLOUDFLARE_DEPLOY_HOOK` |

## Known limits

- **The installer is unsigned.** Windows SmartScreen will warn on first run ("More info", then "Run
  anyway"). Signing needs a code-signing certificate and is deferred.
- **Windows only.** `bundle.targets` is `nsis`, and Resume launches `wt.exe` or `cmd`.
