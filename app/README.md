# app

Logbook itself: GPUI ([gpui-kit](https://crates.io/crates/gpui-kit)) over `logbook-core`, with no webview and no IPC.
See "GPUI is the app" in `docs/PLAN.md` for how it came to be and what was measured; the repository README covers building and
running it.

## What it has

- Sidebar: time ranges, Stats, agents (`Alt+1`…), repositories with a filter, Sources popover, re-index, theme. Below 960px
  it folds to an icon rail.
- Session list: day groups with a pinned heading, search (every keystroke, results with highlighted snippets), empty states.
- Detail: facts, tabs, transcript (selectable markdown, search marks, folding of long messages, your last message pinned above
  the fold), Files with per-file diffs, the resume dock (Resume / copy command / open folder).
- Stats: the fact strip, totals, 26-week activity map, hours, agents, repositories, highlights.
- About, What's New, the update pill and in-app updates, notices, `Ctrl`+wheel zoom (`Ctrl+0` resets), a custom title bar and
  caption buttons, a single-instance guard, the window's size and place remembered.
- Keys: `/` search, `↑ ↓ Home End`, `Enter` resume, `1`–`4` ranges, `5` Stats, `r` re-index, `Esc`, `?`, `Alt+1…9`.

## Left out, compared with the old web UI

- **Motion**: the Stats view's entrance animations (rise, pop, grow, count-up), the streak flame and confetti, the "+1" arrival chip,
  and the wordmark's blinking cursor. The re-index spinner stays.
- **Focus and accessibility**: no roving focus rings on rows, and no ARIA roles beyond what GPUI provides.
- **Diffs**: a line longer than the card is clipped, not scrolled sideways.

## Layout

- `src/lib.rs` (the `logbook_app` library): the plain-data layer, tested on any platform with no window system: `model/` (filters,
  formatting, search-word matching, agents, stats), `transcript.rs` (messages, folding, search marks), `diff.rs`, `update.rs`
  (reading `latest.json`, checking the installer's signature).
- `src/main.rs`: the window, the root view and the keyboard. `app.rs` is the state and everything that changes it (the store);
  `view_*.rs` draw it; `theme.rs`, `ui.rs`, `assets.rs`, `platform.rs` are the support; `tests.rs` the headless UI tests.
- The detail pane is one virtual list (facts, tabs and content scroll together, and only what is on screen is laid out); so is the
  session list. Messages render lazily and cache their markdown view.
- `build.rs` embeds the exe's icon and version info. `tauri.conf.json` configures the installer maker only (the Tauri CLI's bundler,
  run from `app/`); `icons/` and `assets/` hold the icons and the fonts (IBM Plex Sans and JetBrains Mono, both SIL OFL; licences
  are in `assets/fonts`).

## Trying it without a history of your own

```bash
tools/make-fixtures.py /tmp/lb small          # ~80 varied sessions; `big 10000` for timing
COPILOT_HOME=/tmp/lb/copilot CLAUDE_CONFIG_DIR=/tmp/lb/claude LOGBOOK_CACHE_DIR=/tmp/lb/cache cargo run --release -p logbook
```

`LOGBOOK_TRACE=1` prints how long loading, filtering, searching and reading a transcript take. `LOGBOOK_DEV_UPDATE=1` shows the
update pill and What's New with made-up notes. `LOGBOOK_NO_UPDATE_CHECK=1` never looks for updates. `tools/drive.sh` runs the
app in a virtual display, clicks and types, and takes screenshots (see its header).
