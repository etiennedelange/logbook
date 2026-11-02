# Changelog

All notable changes to Logbook. Add an entry under `[Unreleased]` in the same PR as the change;
`pnpm release` dates the section, and it becomes the top of the GitHub release notes. Newest first. The
format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Changed
- **A calmer look.** The sidebar, session list and transcript each get their own shade. The session pane's
  title bar is a single line with the repository and branch. **Resume**, copy and open folder sit in a dock
  under the transcript that shows the command they run. Your messages are bubbles on the right. The
  sidebar shows repositories as folders and no longer shows the index count.

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
