# Plan: more agents, and what comes after

**Date:** 2026-11-02 · **Status:** proposed, nothing started

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

**UI.**
- An **Agents** group in the sidebar above Repositories, shown only when more than one agent has sessions.
  It combines with the time range, repository and search, like the existing filters.
- A small agent label on each list row and in the detail metadata. Use `--cat-*` colours in order, each
  with its text label.
- About lists each source's folder.
- New mock sessions in `ui/src/dev/mock.ts` and a new screenshot scene (`tools/screenshots/capture.mjs`)
  showing both agents. Run `pnpm screenshots` and look at the images.

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

## Open decisions

1. **Claude Code deletes transcripts after 30 days** by default (`cleanupPeriodDays`). Logbook can only show
   what's on disk. Options:
   - say so in About, with the setting to change;
   - keep Logbook's own copy of the searchable text after the source file is gone. That would mean a
     session can't be resumed but can still be found, and it changes "Logbook only reads".
2. **Agent filter placement:** a sidebar group (proposed above) or a toggle in the list header.
3. **Positioning:** the name already fits more than one agent. Decide whether the site leads with "Copilot CLI
   and Claude Code" or with "coding-agent sessions" once Phase 2 ships.
