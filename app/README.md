# gpui-proto

Logbook rebuilt in GPUI ([gpui-kit](https://crates.io/crates/gpui-kit)) over `logbook-core`: no webview, no IPC. An
experiment next to the Tauri app, whose React UI stays the shipping one. See "Experiment: a GPUI front end" in
`docs/PLAN.md` for the question it answers and what has been measured.

```bash
cd gpui-proto   # its own workspace and lockfile, separate from the app's
cargo run --release
cargo test
```

Linux needs `libxkbcommon-dev libxkbcommon-x11-dev` to build, and a Vulkan driver (`mesa-vulkan-drivers`) to open a window.
The `gpui-proto` workflow builds the Windows exe on every push that touches this folder or `core/`.

## What it has

The same screens as the web UI, looking the same (the same fonts, the same OKLCH colour tokens, both themes):

- Sidebar: time ranges, Stats, agents (`Alt+1`…), repositories with a filter, Sources popover, re-index, theme.
  Below 960px it folds to an icon rail.
- Session list: day groups with a pinned heading, search (every keystroke, results with highlighted snippets), empty states.
- Detail: facts, tabs, transcript (selectable markdown, search marks, folding of long messages, your last message
  pinned above the fold), Files with per-file diffs, the resume dock (Resume / copy command / open folder).
- Stats: the fact strip, totals, 26-week activity map, hours, agents, repositories, highlights.
- About, What's New, the update pill, notices, `Ctrl`+wheel zoom (`Ctrl+0` resets), custom title bar and caption buttons.
- Keys: `/` search, `↑ ↓ Home End`, `Enter` resume, `1`–`4` ranges, `5` Stats, `r` re-index, `Esc`, `?`, `Alt+1…9`.

## Where it differs from the web UI

- **Updates**: it checks `latest.json` and offers the release page, but doesn't install. Installing a signed installer stays with
  the Tauri updater.
- **Motion**: the Stats view's entrance animations (rise, pop, grow, count-up), the streak flame and confetti, the "+1"
  arrival chip, and the wordmark's blinking cursor are left out. Everything else that moves (the re-index spinner) does.
- **Focus and accessibility**: no roving focus rings on rows, and no ARIA roles beyond what GPUI provides.
- **Diffs**: a line longer than the card is clipped, not scrolled sideways.
- No single-instance guard, no window icon, and the window's size and place aren't remembered.

## Layout

- `src/lib.rs`: the plain-data layer, tested on any platform with no window system: `model/` (a port of `ui/src/model/*.ts`:
  formatting, search-word matching, filters, agents, stats), `transcript.rs` (messages, folding, search marks), `diff.rs`, `update.rs`.
- `src/main.rs`: the window, root view and keyboard. `app.rs` is the state and everything that changes it (the store);
  `view_*.rs` draw it; `theme.rs`, `ui.rs`, `assets.rs`, `platform.rs` are the support.
- The detail pane is one virtual list (facts, tabs and content scroll together, only what's on screen is laid out); so is the
  session list. Messages render lazily and cache their markdown view.

## Trying it without a history of your own

```bash
tools/make-fixtures.py /tmp/lb small          # ~80 varied sessions; `big 10000` for timing
COPILOT_HOME=/tmp/lb/copilot CLAUDE_CONFIG_DIR=/tmp/lb/claude LOGBOOK_CACHE_DIR=/tmp/lb/cache cargo run --release
```

`LOGBOOK_TRACE=1` prints how long loading, filtering, searching and reading a transcript take. `LOGBOOK_DEV_UPDATE=1` shows the
update pill and What's New with made-up notes. `tools/drive.sh` runs the app in a virtual display, clicks and types, and takes
screenshots (see its header), which is how most of this was checked without a Windows machine.
