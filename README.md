## Logbook

A logbook for your coding-agent sessions from the GitHub Copilot CLI and Claude Code: search,
browse and resume them from a desktop app.
Built with Tauri 2 (Rust) and React, laid out like
[DevFlow](../ado-hooks): sidebar, list, detail.

![Logbook showing a session transcript](docs/screenshots/browse-dark.webp)

- **Keyword search** over session names and full transcripts, with highlighted snippets.
- **Time ranges**: all sessions, today, last 7 days, last 30 days (`1`–`4`).
- **Agents**: with sessions from both agents, filter to one from the sidebar (`Alt+1`, `Alt+2`).
- **Repositories**: filter to one repository from the sidebar.
- **Transcript view** with session metadata.
- **Stats** (`5`): sessions, messages and streaks for the last 7 days, 30 days or 26 weeks; a
  day-by-day activity map; when you start sessions; agents and top repositories; and a few
  highlights, like your chattiest session. New sessions appear live while it's open.
- **Resume**: opens a terminal in the session's folder running `copilot --resume=<id>` or
  `claude --resume <id>` (or **Copy** the command).
- **Incremental indexing**: a small `postcard` cache in your local app data folder; only
  sessions whose files changed get re-scanned. Re-indexes when the window
  regains focus, or press `r`.
- **Kept copies**: Claude Code deletes transcripts after 30 days (`cleanupPeriodDays`). Logbook
  keeps its own copy of every conversation it has read, so those sessions stay searchable and
  readable, marked "source deleted". They can't be resumed.
- **Updates**: when a new release is out, **Update** appears in the toolbar. **What's New** shows
  its release notes, and **Update now** installs it and restarts.

![Logbook's Stats view: totals, an activity map, hours, agents and repositories](docs/screenshots/stats-dark.webp)

Keyboard: `/` search · `↑`/`↓` move · `Enter` resume · `1`–`4` range · `5` stats · `Alt`+`1`–`2` agent · `Esc` close ·
`?` about · `Ctrl`+wheel zoom.

### Download

Windows installers are on the [releases page](https://github.com/etiennedelange/logbook/releases).

### Layout

| Path         | What                                                                                      |
|--------------|-------------------------------------------------------------------------------------------|
| `core/`      | Indexing and search engine (`logbook_core` lib) and `SessionService`                      |
| `src-tauri/` | Tauri shell: commands the UI calls, background indexing, single instance                  |
| `ui/`        | React 19 + TypeScript + Tailwind 4 + zustand, built with [Vite+](https://viteplus.dev)    |
| `site/`      | Marketing site: Astro, static, deployed to Cloudflare Workers (static assets)            |
| `tools/`     | `screenshots/` (Playwright, mock data) and `release/` (version sync)                      |

### Running

Requires Rust (stable), Node and pnpm. No Docker or server needed.

```bash
pnpm install
pnpm tauri dev      # dev window with hot reload
pnpm tauri build --config src-tauri/tauri.no-updater.conf.json   # installer, without the updater signing key
```

Work on the UI alone, in a browser, against fake data:

```bash
pnpm mock:web      # same as: pnpm --dir ui dev:mock
```

Screenshots (real UI, fake sessions, light and dark; one-time `pnpm --dir tools/screenshots exec playwright install chromium`):

```bash
pnpm screenshots            # every scene; the published ones land in docs/screenshots/
pnpm screenshots search     # one scene
pnpm --dir site dev         # the marketing site
```

Releasing is a version bump and a tag: see [docs/RELEASING.md](docs/RELEASING.md).

Checks:

```bash
cargo test                 # core tests
pnpm --dir ui check        # Vite+: format, lint, types
pnpm --dir ui test         # UI tests
```

Optional environment variables (handy for testing against other data):

| Variable            | Default           | Purpose                                         |
|---------------------|-------------------|-------------------------------------------------|
| `COPILOT_HOME`      | `~/.copilot`      | Root of the Copilot CLI's session data          |
| `CLAUDE_CONFIG_DIR` | `~/.claude`       | Root of Claude Code's data (`projects/` is read) |
| `LOGBOOK_CACHE_DIR` | OS local data dir | Where this app's own `index-v5.bin` cache lives |

The app never reads `~/.copilot/session-store.db`; that SQLite file belongs to the
Copilot CLI itself. It never writes to either agent's folder.
