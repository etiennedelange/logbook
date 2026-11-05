# Changelog

All notable changes to Logbook. Add an entry under `[Unreleased]` in the same PR as the change;
`pnpm release` dates the section, and it becomes the top of the GitHub release notes. Newest first. The
format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added
- **The files a session changed.** A Claude Code session that created or edited files gets a **Files**
  tab next to its transcript: each file with how many times it was edited and the lines added and
  removed, and the shell commands that may have changed other files. Click a file to see each edit as a
  diff. Search matches these paths too, so `indexer.rs` finds the sessions that changed it and opens them
  on that file. Copilot CLI sessions will follow once their edit records are checked.
- **Search by session id.** Paste a session's id, its first 8 characters, or a whole resume command
  (`claude --resume …`) into search to find that session.

## [0.6.3] - 2026-11-05

### Added
- **Your last message stays in sight.** Scroll down a long reply and the message you sent that it
  answers stays pinned as one line at the top of the transcript. Click it to read more of it, then
  **Jump to message** to go back to it.

## [0.6.2] - 2026-11-03

### Added
- **Stats.** **Stats** in the sidebar (`5`) shows your sessions, messages, active days and current streak
  for the last 7 days, 30 days or 26 weeks, with a day-by-day activity map, the hours you start sessions,
  your agents and top repositories, and a few highlights. Click a repository or an agent to see its
  sessions, or your chattiest session to open it. Sessions found while it's open arrive with a **+1**.

### Fixed
- **A restored Claude Code session can be resumed again.** A transcript put back exactly as it was (from the
  Recycle Bin, say) stayed marked "source deleted", so Logbook wouldn't resume it.

## [0.6.1] - 2026-11-03

### Added
- **See where your sessions come from.** **2 sources** at the bottom of the sidebar lists the folder Logbook
  reads for each agent, how many sessions came from it and when Logbook last read it, with a button to open
  it. A folder that doesn't exist yet (an agent you haven't installed) says **Not found**.

### Fixed
- **Check for updates says what it found.** In About, its answer ("Logbook is up to date", or why the
  check failed) was hidden behind the About window, so the button seemed to do nothing.

## [0.6.0] - 2026-11-02

### Fixed
- **Update now works behind a shared address.** It could fail with a 403 error, from GitHub's limit on
  downloads through its API per address (an office network or a VPN reaches it quickly). Updates now
  download from the release's ordinary link.
- **Re-index shows what it found.** After **Re-index** (`r`) on a small set of sessions, the list could
  stay as it was until the next change: a new session didn't appear and a deleted one wasn't marked.
- **The window no longer slides up.** Picking a session or opening a search match could scroll the whole
  app upwards, leaving a blank strip at the bottom and a stray scroll bar (#23).

## [0.4.0] - 2026-11-02

### Changed
- **A calmer look.** The sidebar, session list and transcript each get their own shade. The session pane's
  title bar is a single line with the branch. **Resume**, copy and open folder sit in a dock under the
  transcript that shows the command they run (`copilot` or `claude`) and where. Your messages are bubbles
  on the right. The sidebar shows repositories as folders and no longer shows the index count.

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
