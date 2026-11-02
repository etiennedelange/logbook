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
3. Publishing triggers **`site.yml`**, which rebuilds the site and deploys it to Cloudflare Pages. The
   site's download button points at `releases/download/v<version>/Logbook_<version>_x64-setup.exe`, so it
   only works once the release is published.

A tag containing `-` (for example `v0.3.0-rc.1`) is marked as a pre-release.

## The version

`src-tauri/tauri.conf.json` is the source of truth, because the installer is named from it. The version also
lives in `package.json`, `ui/package.json`, `src-tauri/Cargo.toml`, `core/Cargo.toml` and `Cargo.lock`
(two entries). `tools/release/version.mjs` writes all seven; `--check` fails if any disagree.

## One-time setup

| What | Where |
|------|-------|
| Cloudflare Pages project named `logbook` | `npx wrangler pages project create logbook --production-branch=main` |
| `CLOUDFLARE_API_TOKEN` (Pages: Edit) and `CLOUDFLARE_ACCOUNT_ID` | secrets on the `cloudflare-pages` environment |
| `SITE_URL` (for canonical and OG URLs, once the domain is decided) | repository variable |

## Known limits

- **The installer is unsigned.** Windows SmartScreen will warn on first run ("More info", then "Run
  anyway"). Signing needs a code-signing certificate and is deferred.
- **No auto-update.** Users download the next installer themselves. Tauri's updater needs a signing keypair;
  deferred.
- **Windows only.** `bundle.targets` is `nsis`, and Resume launches `wt.exe` or `cmd`.
- **A private repository has private releases.** The download link 404s for anyone not signed in with
  access. Make the repository public before announcing the site.
