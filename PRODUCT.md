# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

(A Tauri 2 desktop app for Windows. The UI is React in a webview, with a unified title bar drawn by the app.)

## Users

Windows developers who run coding agents in the terminal: the GitHub Copilot CLI today, Claude Code next.
Etienne is the first user. They come to Logbook mid-work, wanting a past session back: they remember
something that was said, roughly when, or which repository, but not the session's name or id.

## Product Purpose

Find a past agent session by what was said in it, and resume it in its folder with one key. Success is going
from "I did this with Copilot last week" to the session running again in a few seconds.

## Positioning

Logbook is for coding-agent sessions, not one agent's. Instant local full-text search over every message, combined with repository and date filters and one-key
resume, in a native Windows window. The agents' own pickers search titles and first prompts, not whole
transcripts. The closest multi-agent app (agent-sessions) is macOS-only.

## Operating Context

- Opened alongside a terminal; keyboard-first (`/` search, arrows, `Enter` resume, `1`–`4` time ranges).
- Reads the agents' session files on disk; re-indexes on focus or `r`. Resume opens Windows Terminal (or
  `cmd`) in the session's working folder.
- Updates through GitHub Releases, with a What's New window.

## Capabilities and Constraints

- Indexes user and assistant messages only (not tool output).
- Reads each agent's files and never opens another tool's database.
- Planned (docs/PLAN.md): a `Source` per agent; Claude Code next, then Codex CLI.
- Decided 2026-11-02: Logbook will keep its own copy of a session's searchable text after the agent deletes
  the source file (Claude Code deletes transcripts after 30 days by default). Such a session stays findable
  but can't be resumed. This changes the "Logbook only reads" promise, so the site and About must say so.

## Brand Commitments

- Etienne de Lange's personal brand, Instrument register: IBM Plex Sans and JetBrains Mono, violet signal.
  No lemon mark and no Stamp (neobrutalist) style.
- Plain English voice. Name: Logbook.
- The title bar is part of the app surface: one unified toolbar, no native strip.

## Evidence on Hand

- Screenshots in `docs/screenshots/`, generated from `ui/src/dev/mock.ts` (fake sessions).
- Comparison with the Copilot CLI's own session tools on the site (`site/src/pages/index.astro`, `#compared`).
- No user counts, testimonials or benchmarks exist; don't invent them.

## Product Principles

1. Finding beats browsing: search over everything said, filters only narrow it.
2. One key from found to running.
3. The agents own their files; Logbook only adds an index (and, from Phase 2, a kept copy of deleted text).
4. Keyboard first, mouse never worse.
5. Each agent is a source, not a mode: filters combine, nothing forks the UI.
