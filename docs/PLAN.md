# Plan: more agents, and what comes after

**Date:** 2026-11-02 · **Status:** Phases 1–2 built (2026-11-02); Phase 2 awaits a Windows check; Phase 3 next; Phase 4 built for Claude Code (2026-11-05)

Logbook reads one agent today: the GitHub Copilot CLI. Its advantage over the CLI's own tools is instant local
search of every message, combined with repository and date filters and one-key resume, in a native Windows
window (the comparison is on the site, `site/src/pages/index.astro`, `#compared`). The closest app that does
this for several agents, [agent-sessions](https://github.com/jazzyalex/agent-sessions), is macOS-only. So
the next step is to support more agents on Windows, starting with Claude Code.

## What has to change

Every layer assumes Copilot. The parts that do:

| Where | Copilot assumption |
| --- | --- |
| `core/src/paths.rs` | One root: `~/.copilot/session-state` (or `COPILOT_HOME`) |
| `core/src/indexer.rs` | One folder per session, with `workspace.yaml` and `events.jsonl`; fingerprint is those two files |
| `core/src/yaml_reader.rs`, `events_reader.rs` | Copilot's file formats and event names (`user.message`, `assistant.message`) |
| `core/src/models.rs` | `SessionRecord` has no idea which agent wrote it; `id` is a bare UUID |
| `core/src/service.rs` `resume_command` | Always `copilot --resume=<id>` |
| `src-tauri/src/commands.rs` `resume_session` | Runs `copilot` in `wt.exe` / `cmd` |
| `ui/` | No agent filter or label; About shows one sessions folder |
| `site/`, README | "for your Copilot CLI sessions" throughout |

## Phase 1: one source behind a trait, no visible change

**Done 2026-11-02.** `core/src/source/` (trait in `mod.rs`, `copilot.rs`). Differences from the sketch:
`discover` returns `Result` (an unreadable folder fails the run rather than emptying the index), `resume`
returns `Option` (`None` when `is_safe_id` fails), and a source whose roots are all missing keeps its cached
sessions, as before. Sessions carry `agent` and `native_id`; the detail pane shows `nativeId`. Checked on a
synthetic 550-folder history (no real one on the dev machine): same counts and search output as 0.1.1.

Put Copilot behind a `Source` trait in `core/`, so a second agent is one new module:

```rust
trait Source: Sync {
    fn agent(&self) -> Agent;                       // Copilot, ClaudeCode, ...
    fn roots(&self) -> Vec<PathBuf>;                // shown in About
    fn discover(&self) -> Vec<SessionFiles>;        // one per session: its id, files, fingerprint
    fn read(&self, files: &SessionFiles) -> Option<Scan>;   // record, searchable text, first prompt, count
    fn transcript(&self, record: &SessionRecord) -> Vec<TimelineEntry>;
    fn resume(&self, record: &SessionRecord) -> Resume;     // program, args, working folder
}
```

- `SessionRecord` gains `agent`. Ids become `<agent>:<native id>` so two agents can't collide; the native id
  stays available for the resume command.
- `Fingerprint` becomes a list of (path, size, mtime), since sources differ in how many files make a session.
- `resume_session` runs `Resume { program, args, cwd }` instead of building the Copilot line itself. Keep the
  argument-list spawn (no shell string) and `is_safe_id` per source.
- Cache goes to `index-v5.bin` (postcard has no schema; see `paths.rs`).

Done when `cargo test`, `pnpm --dir ui check` and `pnpm --dir ui test` pass unchanged and a real Copilot
history indexes to the same session count as 0.1.1.

## Phase 2: Claude Code, end to end

**Built 2026-11-02**, pending the Windows check below. `core/src/source/claude.rs`; kept copies in `index.rs`
(`Scan::kept`, `retire_missing`); UI per option A; `agents` screenshot scene. Differences from the plan:
- The repository comes from the session folder's git `origin` (`core/src/git.rs`), in Copilot's form, so both
  agents share Repositories entries. Claude Code doesn't record one.
- The detail pane's **Agent** field heads the Folder/Session rows (a fourth top column truncated the dates), and
  the deleted note says "deleted ... by <date>": Logbook knows when it noticed, not when Claude Code deleted it.
- The fixture (`core/tests/fixtures/claude/synthetic.jsonl`) is a small hand-written one covering the main
  cases; more can be added to it later.
- Claude Code's docs (code.claude.com/docs/en/sessions) list what each picker row shows but not which fields the
  search matches, and add `Ctrl+W` (all worktrees); the site's table says only that.
- Agents, labels and the Agent field appear only when two agents have sessions; with one, the UI is unchanged.
- Built to scale past a handful of agents (changed 2026-11-02): the sidebar lists agents by session count, the
  top four plus "Show all" (the picked one always shows), and `Alt+1`... follow that order. Rows carry a muted
  text tag (`copilot`, `claude`) after the message metadata instead of a coloured dot and full name; colour is
  only a secondary cue on the sidebar dots, since only a few colours stay distinguishable.

Still to check on Windows: both histories show, the agent filter works, and Enter resumes a Claude session in
its folder (`wt.exe` running `claude --resume <id>`).

**Reading.** Sessions are `~/.claude/projects/<project>/<session-id>.jsonl` (`CLAUDE_CONFIG_DIR` moves
`~/.claude`; on Windows that's `%USERPROFILE%\.claude`). The folder name is the working path with
non-alphanumerics replaced by `-`, so it can't be turned back into a path: take `cwd` and `gitBranch` from the
records instead. Skip `subagents/` and `tool-results/` folders.

Anthropic's docs say the line format is internal and changes between versions. So the reader:
- keeps only user and assistant text, ignores every record type it doesn't know, and never fails a session
  over one bad line (the same tolerance as `events_reader.rs`);
- is tested against fixtures copied from real transcripts across a few Claude Code versions, in
  `core/tests/fixtures/claude/`, with names and paths scrubbed;
- uses the session's `/rename` name or generated title when present, else the first prompt.

**Resume.** `claude --resume <session-id>`, run in the session's `cwd` like Copilot. (Claude can also find the
session from any folder, but starting in the right one keeps its project context.)

**Kept copies.** When a source file disappears (Claude Code deletes transcripts after `cleanupPeriodDays`,
30 by default), Logbook keeps the session instead of dropping it from the index. It keeps the transcript too, so the
detail pane still shows it. A kept session behaves like any other in search, filters and the list. It just
can't be resumed and says its source was deleted. This changes "Logbook only reads": it reads the agents' files and
keeps its own copy. About and the site must say so.

**UI** (shaped 2026-11-02; sketches: https://claude.ai/artifact/7tfnu5V4j28Tm5jKWLpYCZ, option A).
- An **Agents** group in the sidebar between the time ranges and Repositories: dot, name and count per agent, the
  same rows as Repositories. Shown only when more than one agent has sessions. It combines with the time range,
  repository and search, like the existing filters; selecting the active agent again clears it. An agent with no
  sessions in the current range keeps its row with 0, so the sidebar doesn't jump. `Alt+1`, `Alt+2`, ... pick
  agents (`1`–`4` stay the time ranges). In the narrow icon rail, agents show as dots with names for screen readers.
- While an agent is selected, the list title is its name ("Claude Code · 23 sessions") and rows drop their agent
  label. Otherwise each row's metadata line starts with the agent label: a dot in a `--cat-*` colour (in order)
  plus the name. The detail metadata gains an **Agent** field first.
- A session whose source was deleted gets a quiet "source deleted" tag on its row. Its detail pane says when the
  agent deleted the file and that it can't be resumed, and names the setting that controls retention
  (`cleanupPeriodDays`). Resume is disabled; Copy still copies the command. No separate filter for these.
- About lists each source's folder and says Logbook keeps copies of deleted sessions.
- New mock sessions in `ui/src/dev/mock.ts` (two agents, one deleted source) and a new screenshot scene
  (`tools/screenshots/capture.mjs`) showing both agents. Run `pnpm screenshots` and look at the images.

**Words.** The README opening, `CHANGELOG.md` under `[Unreleased]`, the site's title, hero and
`#compared` table (add Claude Code's `/resume` picker: titles, summaries and first prompts; current
worktree, `Ctrl+A` for all projects; no date filter), and `site/DESIGN.md`'s description.

Done when a machine with both histories shows both, filters by agent, and Enter resumes a Claude session in
its folder.

## Phase 3: Codex CLI

Check the format against the source before building. My research could only partly confirm it.
Unconfirmed: whether sessions are JSONL "rollout" files under `~/.codex/sessions/` (with `CODEX_HOME`), and
the exact `codex resume <id>` form. Also check the report that the Codex desktop app compresses
sessions idle for 7 days into `.jsonl.zst`, which would mean adding a zstd read. Same tests and UI as Phase 2,
with a fixture per format.

## Phase 4: files a session touched, and search by session ID

**Designed 2026-11-05** (canvas: https://claude.ai/artifact/XNDJUhvNuN73qQv3FV3n2N, option A chosen over
showing changes inline in the transcript). **Built 2026-11-05 for Claude Code** (`core/src/changes.rs`,
`ui/src/components/Files.tsx`). Differences from the design:
- Copilot isn't read yet: its edit events still need checking against a real `events.jsonl`.
- "Whole session" (Claude Code's `file-history` copies) and "Show in transcript" were left out: the
  diff shows each edit. The file-history folder wasn't there to check its format against.
- An id matches from 8 characters only when the query is that one token; a pasted resume command's own
  words (`claude`, `--resume`, ...) are dropped only when the rest looks like an id (has a digit).
- Which commands "may have changed files" is a guess from their words (`rm`, `git mv`, `sed -i`,
  `> file`, formatters ...), skipping heredoc bodies; at most 30 are kept per session.

**Files.** While reading a session, collect the files the agent created or edited from its tool calls: Claude
Code's `Write`, `Edit`, `MultiEdit` and `NotebookEdit` blocks (`input.file_path`, plus the `toolUseResult`
patch for line counts); Copilot's create/edit tool events, whose names and fields must be checked against a
real `events.jsonl` first. Store per file: path (relative to the session folder when inside it), created or
edited, edit count, lines added and removed. Shell commands are listed separately, since Logbook can't
tell which files they changed. Unknown tools and bad lines are skipped, as in the message readers.

- The detail pane gets **Transcript | Files n** tabs; the facts row gains "Files: 2 created, 4 edited". The
  Files tab groups Created and Edited (path, edit count, +/−), then "Commands that may have changed files",
  then a note that it shows what changed during the session, not the files as they are now. No tab when a
  session touched no files.
- A file opens its edits as numbered diffs with times and "Show in transcript". "Whole session" (before the
  first edit vs after the last) uses Claude Code's `~/.claude/file-history/<session-id>/` copies, and is
  disabled when they're gone. Diffs are read on demand from the session file, never indexed.
- Paths are searchable: a matching row's snippet reads "Edited `core/src/indexer.rs` 4 times", and the detail
  pane opens on Files with the file marked. Only paths go in the index, not diff text (unlike "Search tool
  output" below).
- Kept copies keep the file list; diffs survive only as far as the kept transcript holds them.
- `SessionRecord` changes, so the cache moves to the next `index-v*.bin`.

**Session ID.** Search matches a session's native ID: whole, or a prefix of 8 or more characters, or a
pasted resume command (`claude --resume <id>`, `copilot --resume=<id>`), whose command words aren't searched.
The row's snippet shows "Session <id>" with the match marked, and so does the Session field in the detail pane.

Tests: fixture lines for each edit tool in `core/tests/fixtures/claude/`; `pnpm ui-check` checks for the tab,
file rows and both searches; mock data with files; new screenshot scenes (Files tab, file diff). An e2e step
for path and ID search.

## Later, on request

- **Gemini CLI / Antigravity CLI.** Individual Gemini CLI users moved to Antigravity CLI (`agy`) on
  2026-06-18. Gemini's own `/resume` already searches its sessions, so there is less to gain. Wait until
  someone asks.
- **OpenCode.** 2.x appears to keep sessions in SQLite. Logbook's promise is that it never opens another
  tool's database (it doesn't touch Copilot's `session-store.db`). Opening one read-only (`?mode=ro&immutable=1`)
  would bend that, so decide it on purpose and say so on the site.
- **Search tool output.** Today only user and assistant messages are indexed, so file paths and error
  text that appear only in tool calls don't match. This could be an option, but measure its effect on index
  size and search time on a real history first.
- **Branch filter.** `branch` is already on every Copilot record. Claude Code's picker has `Ctrl+B`. It's cheap to add
  next to Repositories.
- **macOS and Linux.** The core reads paths portably. Resume is the Windows-only part (`wt.exe`/`cmd`).
  agent-sessions covers macOS, so this matters less than more agents on Windows.

## Decisions

1. **Deleted Claude Code transcripts** (decided 2026-11-02): Logbook keeps its own copy; see Kept copies in Phase 2.
2. **Agent filter placement** (decided 2026-11-02): a sidebar group, as in Phase 2's UI.
3. **Positioning** (decided 2026-11-02): once Phase 2 ships, the site and README lead with "coding-agent sessions",
   naming Copilot CLI and Claude Code as the supported agents.
