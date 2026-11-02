# Logbook

Tauri 2 desktop app (Windows) for searching and resuming GitHub Copilot CLI sessions.
`core/` Rust engine, `src-tauri/` shell, `ui/` React, `site/` Astro marketing site. See README.md.
Planned work (more agents, starting with Claude Code) is in `docs/PLAN.md`; update its phase status as work lands.

## Screenshots

A visible UI change isn't finished until the screenshots are regenerated, in the same commit:

```bash
pnpm screenshots            # all scenes; or name one: pnpm screenshots search
```

Published shots go to `docs/screenshots/` (listed in `PUBLISHED` in `tools/screenshots/capture.mjs`); the
README and `site/` show them. A new view or state worth showing gets a new scene in that file, not just a
rerun. Look at the images before committing them. Scenes use `ui/src/dev/mock.ts` data.

## UI check

`pnpm ui-check` (or `pnpm ui-check search agents`) clicks through every control in headless Chromium against
the mock data and checks its state, text, focus and notices; it exits non-zero on any failure. Run it after a
UI change. A new control or state gets a check in `tools/screenshots/check.mjs`; mock data changes may need
its expectations (session names, resume commands) updated.

## Releasing

`docs/RELEASING.md`: `pnpm release [version]`, or Actions → Cut release. Add a `CHANGELOG.md` line under
`[Unreleased]` in the PR for anything a user would notice. Never edit a version by hand: `node tools/release/version.mjs`.

## Site

`site/` follows the personal brand's Instrument register (Nord + Noctis Lux, violet signal), without the lemon
mark and without the Stamp (neobrutalist) style: Etienne doesn't want either on this site. Tokens are generated into
`site/src/styles/brand.css`, so regenerate rather than edit; overrides live in `site/src/styles/site.css`.
OG image and favicons: `pnpm site-assets`. `site/DESIGN.md` records the choices.
