# Logbook

Native Windows desktop app (GPUI, Rust: no webview) for searching and resuming GitHub Copilot CLI and Claude Code sessions.
`core/` Rust engine, `app/` the GPUI app, `site/` Astro marketing site. See README.md.
Planned work (more agents, starting with Claude Code) is in `docs/PLAN.md`; update its phase status as work lands.

## The app

`app/src/lib.rs` is the plain-data layer (`model/`: filters, formatting, search-word matching, agents, stats; `transcript.rs`,
`diff.rs`, `update.rs`): no GPUI, unit-tested on any platform. Put logic there when it can live there. `app/src/app.rs` is the
state and everything that changes it; `view_*.rs` only draw it. Sizes are `u(px)` (the web UI's CSS pixels as rems, so
`Ctrl`+wheel zoom scales everything); colours come from `theme.rs` (the OKLCH tokens, both themes).

## Tests

```bash
cargo test --workspace      # core, the plain-data library, and the headless UI tests
```

`app/src/tests.rs` drives the real views in a headless window with real clicks and keys against a temporary history of
Copilot CLI and Claude Code sessions, and checks what the user would see (rows, selection, transcript, filters, dialogs,
files and diffs, a session the agent deleted). A new control or state gets a test there: name its element with
`.test_id("...")` (a no-op in the app) so the test can click it, and check the state it changes. Mock-free: a change to what
indexing, search or reading files does gets a test against the fixture history, not a stub.

## Screenshots

A visible UI change isn't finished until the screenshots are regenerated, in the same commit:

```bash
cargo build --release -p logbook
pnpm screenshots            # all scenes and clips; or name some: pnpm screenshots search stats
```

It starts the app in a virtual display against made-up sessions (`tools/screenshots/demo.mjs`), drives it like a person, and
photographs it. Published shots go to `docs/screenshots/` (listed in `PUBLISHED` in `tools/screenshots/capture.mjs`); the README
and `site/` show them. A new view or state worth showing gets a new scene in that file, not just a rerun. Look at the images
before committing them. Needs Xvfb, xdotool, ImageMagick and ffmpeg (the dev container has them).

To see a change working without the full set: `app/tools/drive.sh <fixtures> <out> step...` runs the app, clicks, types and takes
screenshots; `LOGBOOK_TRACE=1` prints timings; `LOGBOOK_DEV_UPDATE=1` shows the update pill and What's New.

## Releasing

`docs/RELEASING.md`: `pnpm release [version]`, or Actions → Cut release. Add a `CHANGELOG.md` line under
`[Unreleased]` in the PR for anything a user would notice. Never edit a version by hand: `node tools/release/version.mjs`.
CI builds an unsigned installer on every run (Actions → CI → Artifacts), so the release path is exercised before a release.

## Site

`site/` follows the personal brand's Instrument register (Nord + Noctis Lux, violet signal), without the lemon
mark and without the Stamp (neobrutalist) style: Etienne doesn't want either on this site. Tokens are generated into
`site/src/styles/brand.css`, so regenerate rather than edit; overrides live in `site/src/styles/site.css`.
OG image and favicons: `pnpm site-assets`. `site/DESIGN.md` records the choices.
