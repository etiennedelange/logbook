# Changelog

All notable changes to Logbook. Add an entry under `[Unreleased]` in the same PR as the change;
`pnpm release` dates the section, and it becomes the top of the GitHub release notes. Newest first. The
format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

## [0.3.0] - 2026-11-02

### Added
- **Claude Code.** Logbook now reads Claude Code sessions (`~/.claude/projects`) alongside the Copilot
  CLI's, in one list and one search. With both, an **Agents** group in the sidebar filters to one
  (`Alt+1`, `Alt+2`), and each row says which agent it's from. **Resume** runs `claude --resume <id>` in
  the session's folder. Claude Code sessions are filed under the same repository as Copilot ones.
- **Kept copies.** Claude Code deletes transcripts after 30 days. Logbook keeps its own copy of each
  conversation, so a deleted session stays searchable and readable, marked "source deleted"; it can't be
  resumed.

### Fixed
- **The window's close button stays visible on hover.** It turned white on a clear background, so it
  vanished under the pointer in the light theme.
- **Session titles have more room.** The open-folder button moved next to the folder path, and in a narrow
  window **Resume** shrinks to its icon, so the title is cut short less often. The dates under it no
  longer get cut off either.

## [0.2.0] - 2026-11-02

### Added
- **Updates.** Logbook checks GitHub for new releases. When one is out, **Update** appears in the
  toolbar, and **What's New** shows its release notes. **Update now** installs it and restarts Logbook,
  which then shows what changed once. **About → Check for updates** checks right away. Coming from 0.1.1,
  download this installer once by hand; after that Logbook updates itself.

## [0.1.1] - 2026-11-02

### Fixed
- **The Windows installer builds.** 0.1.0 was tagged but its installer build failed, so it was never
  published; 0.1.1 is the first release, with everything listed under 0.1.0.

## [0.1.0] - 2026-11-02

The first release.

### Added
- **Keyword search** over session names and full transcripts, with highlighted snippets.
- **Time ranges**: all sessions, today, the last 7 days or the last 30 days (`1`–`4`).
- **Repositories**: filter to one repository from the sidebar.
- **Transcript view** with the session's metadata.
- **Resume** opens a terminal in the session's folder running `copilot --resume=<id>`, or **Copy**
  copies the command.
- **Incremental indexing** into a small cache in your local app data folder: only sessions that changed
  are scanned again. Logbook re-indexes when its window regains focus, or when you press `r`.
- **Keyboard**: `/` search, `↑`/`↓` move, `Enter` resume, `Esc` close, `?` about, `Ctrl`+wheel zoom.
