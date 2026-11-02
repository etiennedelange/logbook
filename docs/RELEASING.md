# Releasing

A release is a version bump, a tag and one click. Everything between is automated.

## Cut a release

```bash
node tools/release/version.mjs minor      # or patch, major, or an exact 0.3.0
pnpm screenshots                          # only if the UI changed since the last release
git commit -am "Release v0.2.0"
git push                                  # CI must pass on main first
git tag v0.2.0 && git push origin v0.2.0
```

1. **`release.yml`** checks that the tag and all seven version fields agree, builds the NSIS installer on
   Windows, and attaches `Logbook_<version>_x64-setup.exe` to a **draft** release with generated notes.
2. Open the draft on GitHub, read the notes, then **Publish release**.
3. Publishing triggers **`site-rebuild.yml`**, which calls the Workers Builds deploy hook to rebuild the
   site. The site's download button points at
   `releases/download/v<version>/Logbook_<version>_x64-setup.exe`, so it only works once the release is
   published. (Workers Builds also deploys the site on pushes to `main` that touch it, through its GitHub
   link.)

A tag containing `-` (for example `v0.3.0-rc.1`) is marked as a pre-release.

## The version

`src-tauri/tauri.conf.json` is the source of truth, because the installer is named from it. The version also
lives in `package.json`, `ui/package.json`, `src-tauri/Cargo.toml`, `core/Cargo.toml` and `Cargo.lock`
(two entries). `tools/release/version.mjs` writes all seven; `--check` fails if any disagree.

## One-time setup

| What | Where |
|------|-------|
| Worker `logbook-site` (static assets, `site/wrangler.jsonc`), linked to this repo | first `wrangler deploy` from `site/` creates it; then the Worker's Settings > Build > Connect |
| Build settings: root directory `site`, build command `pnpm build`, deploy command `npx wrangler deploy`; watch paths `site/*` and `docs/screenshots/*`, so a version bump doesn't publish a download link before its release | the Worker's Settings > Build |
| Build variable `SITE_URL` (canonical and OG URLs), now `https://logbook-site.etiennedelange.workers.dev` | the Worker's Settings > Build > Variables and secrets |
| Deploy hook for `main`, saved as the `CLOUDFLARE_DEPLOY_HOOK` secret | the Worker's Settings > Build > Deploy Hooks, then `gh secret set CLOUDFLARE_DEPLOY_HOOK` |

## Known limits

- **The installer is unsigned.** Windows SmartScreen will warn on first run ("More info", then "Run
  anyway"). Signing needs a code-signing certificate and is deferred.
- **No auto-update.** Users download the next installer themselves. Tauri's updater needs a signing keypair;
  deferred.
- **Windows only.** `bundle.targets` is `nsis`, and Resume launches `wt.exe` or `cmd`.
- **A private repository has private releases.** The download link 404s for anyone not signed in with
  access. Make the repository public before announcing the site.
