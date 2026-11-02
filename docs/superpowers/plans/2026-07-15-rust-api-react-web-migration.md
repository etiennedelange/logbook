# Rust API + React Web App Migration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn the existing Rust terminal app into a fast, lightweight local **web app** for visualizing GitHub Copilot CLI sessions, **reusing the existing Rust indexing/search/parsing core** and adding a small React + TypeScript frontend. Resume is a copyable `copilot --resume=<id>` command (no process spawning).

## Why this shape (reevaluation summary)

The previous plan (`2026-07-15-typescript-pwa-migration.md`, now replaced) proposed a full Node/Express + React rewrite. That was rejected because:

1. **"TypeScript 7" is a build-time win, not a runtime win.** TS 7 is the native Go port of the compiler (`tsgo` / `@typescript/native-preview`): ~10x faster typechecking, but **zero** effect on how fast the running app reads/searches sessions. Vite already transpiles via esbuild, so TS 7 only matters for `typecheck`.
2. **The hard, correctness-critical code already exists in Rust.** ~480 lines of fault-tolerant parsing/indexing/search (`yaml_reader`, `events_reader`, `index`, `indexer`, `fts_query`, `models`, `paths`) are pure logic with no TUI coupling. A Node rewrite would re-implement all of it for a runtime that is **slower** at disk I/O + JSON parsing + in-memory scan.
3. **"Copy the command is fine" removes all backend spawning.** The API is pure **read-only**, so no `launcher` shell-out, no elevated permissions, no security surface.

**Chosen architecture (confirmed with user):** Rust backend (axum) reusing the existing core → tiny read-only JSON API → static React + Vite + TypeScript frontend. Frontend: React. Packaging: **both** a self-contained native binary (primary) and Docker (option). **No PWA.** Resume: copy-command.

## Tech Stack

- **Backend:** Rust, axum, tokio, tower-http (static file serving + compression + CORS in dev), `rust-embed` (embed built frontend into the binary), `open` (launch the browser), plus the existing `serde`/`serde_json`/`chrono`/`bincode`/`dirs`/`anyhow`.
- **Frontend:** TypeScript (typecheck via `@typescript/native-preview` / tsgo — "TS 7"), React 18, Vite, React Router, Tailwind CSS, Vitest + Testing Library (light coverage).
- **Packaging:** `cargo build --release` → single self-contained `.exe`/binary with embedded UI; Docker multi-stage (node build → rust build) as an alternative.

## Global Constraints

- Source of truth stays `~/.copilot/session-state/{id}/workspace.yaml` + `events.jsonl`; **never** read `~/.copilot/session-store.db`.
- Preserve fault tolerance: malformed/missing `workspace.yaml` or bad `events.jsonl` lines must not abort indexing (already handled by `yaml_reader::try_read` → `Option` and `events_reader::scan` skipping bad lines).
- Cache stays disposable/derived (`index.bin` via `paths::index_cache_path`), safe to delete/rebuild via incremental fingerprinting.
- **Reuse existing core modules unchanged wherever possible.** Only add `Serialize` where needed and build thin API DTOs; do not rewrite parsing/search logic.
- Behavior parity with the current TUI: keyword search (prefix-AND + STX/ETX snippets), date-range browse, recent browse, detail + transcript, resume.
- Keep it lightweight: no SQLite/FTS engine, no `regex`/`serde_yaml` (per repo conventions). Keep the binary small and C-dependency-free (the `x86_64-pc-windows-gnu` packaging path must still work).
- Resume = show/copy `copilot --resume=<id>`; the backend must not spawn `copilot`.
- No PWA: no service worker, manifest, or install prompt.

---

## Current code reuse map (source of truth for tasks)

Reusable **as-is** (pure logic, no TUI dependency):
- `src/models.rs` — `SessionRecord`, `TimelineEntry`, `SessionSearchResult`.
- `src/paths.rs` — `session_state_root()`, `index_cache_path()`.
- `src/yaml_reader.rs` — `try_read(dir) -> Option<SessionRecord>`.
- `src/events_reader.rs` — `scan(path) -> EventsScanResult { searchable_text, message_count, timeline }`.
- `src/fts_query.rs` — `build(raw) -> Vec<String>`.
- `src/index.rs` — `SessionIndex` (`open`, `flush`, `all`, `by_date_range`, `search_keyword`, fingerprints) + STX/ETX (`\u{2}`/`\u{3}`) snippet markers.
- `src/indexer.rs` — `run(&mut SessionIndex, on_progress) -> IndexStats`.

Replaced by the web app (TUI-only, will be removed or gated later):
- `src/main.rs`, `src/ui/*` (`picker`, `detail`, `formatting`, `mod`), `src/launcher.rs`.

Changes needed to the core:
- `TimelineEntry` derives only `Debug, Clone` today — add `Serialize` (with a `serde` rename to keep JSON camelCase optional) so transcript can be returned as JSON. `SessionSearchResult` similarly, or map to a DTO. `SessionRecord` already derives `Serialize`.
- Convert the crate from bin-only to **lib + bins** so both the TUI and the new server binary share the modules.

---

## File Structure (target)

Backend / crate:
- `Cargo.toml` — add `[lib]`, add server `[[bin]]`, add axum/tokio/tower-http/rust-embed/open deps.
- `src/lib.rs` — expose modules (`models`, `paths`, `yaml_reader`, `events_reader`, `fts_query`, `index`, `indexer`, `server`).
- `src/server/mod.rs` — module glue.
- `src/server/service.rs` — `SessionService` owning `SessionIndex` behind `RwLock`, index-at-startup, query methods, DTO mapping.
- `src/server/dto.rs` — serde response types (`SessionDto`, `SearchResultDto`, `TimelineEntryDto`, `IndexStatsDto`).
- `src/server/routes.rs` — axum `Router` with API + embedded-asset fallback.
- `src/server/assets.rs` — `rust-embed` of `web/dist`, static serving + SPA fallback to `index.html`.
- `src/bin/session-search-web.rs` — server entrypoint: build index, start axum on `127.0.0.1:<port>`, open browser.
- (existing) `src/bin/session-search.rs` or keep `src/main.rs` as the TUI bin — see Task 6 for its fate.

Frontend (`web/`):
- `web/package.json`, `web/tsconfig.json`, `web/vite.config.ts`, `web/index.html`, `web/postcss.config.js`, `web/tailwind.config.ts`.
- `web/src/main.tsx`, `web/src/App.tsx`, `web/src/styles.css`.
- `web/src/lib/api.ts` — typed fetch client.
- `web/src/lib/types.ts` — DTO types mirroring `src/server/dto.rs`.
- `web/src/lib/snippet.tsx` — render STX/ETX markers as `<mark>`.
- `web/src/lib/format.ts` — relative/absolute date helpers.
- `web/src/components/{AppShell,TopNav,SessionList,SessionCard,EmptyState,CopyButton,TranscriptView}.tsx`.
- `web/src/pages/{RecentPage,KeywordSearchPage,DateSearchPage,SessionDetailPage}.tsx`.
- `web/tests/*.test.tsx` — light Vitest coverage.

Packaging / docs:
- `Dockerfile`, `docker-compose.yml`, `.dockerignore`.
- `README.md`, `.gitignore`, `.copilot`/Copilot instructions update.

---

### Task 1: Convert crate to lib+bins and make core types serializable

**Files:**
- Modify: `Cargo.toml`
- Create: `src/lib.rs`
- Modify: `src/models.rs`
- Modify: `src/main.rs` (or move to `src/bin/session-search.rs`)

**Interfaces:**
- Consumes: existing modules.
- Produces:
  - a library crate exposing `session_search::{models, paths, yaml_reader, events_reader, fts_query, index, indexer}`.
  - `TimelineEntry` and search results are `Serialize`.
  - existing TUI still builds and runs (`cargo run --bin session-search`).

- [ ] **Step 1: Baseline — confirm current build/TUI still works**

Run: `cargo build`
Expected: PASS (baseline before restructuring).

- [ ] **Step 2: Add `[lib]` and expose modules**

```toml
# Cargo.toml (additions)
[lib]
name = "session_search"
path = "src/lib.rs"

[[bin]]
name = "session-search"
path = "src/main.rs"

[[bin]]
name = "session-search-web"
path = "src/bin/session-search-web.rs"
```

```rust
// src/lib.rs
pub mod models;
pub mod paths;
pub mod yaml_reader;
pub mod events_reader;
pub mod fts_query;
pub mod index;
pub mod indexer;
pub mod server;
```

Update `src/main.rs` (TUI) and internal `use crate::...` imports so both the bin and lib see modules consistently (bins reference `use session_search::...`; the lib owns the modules). Keep `ui` + `launcher` local to the TUI bin (they are TUI-only) or move them under the bin module tree.

- [ ] **Step 3: Add `Serialize` to timeline/search types**

```rust
// src/models.rs
#[derive(Debug, Clone, Serialize)]
pub struct TimelineEntry {
    pub role: String,
    pub timestamp: Option<DateTime<Utc>>,
    pub text: String,
}
```

(Leave `SessionSearchResult` as-is and map to a DTO in Task 2, or add `Serialize` — choose DTO mapping to keep transport types decoupled.)

- [ ] **Step 4: Verify library + TUI both compile**

Run: `cargo build`
Expected: PASS; `cargo run --bin session-search` still launches the TUI against real data.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml src/lib.rs src/models.rs src/main.rs
git commit -m "refactor: expose core as a library crate for reuse by web server"
```

---

### Task 2: Build the axum read-only JSON API reusing the index/indexer

**Files:**
- Modify: `Cargo.toml` (axum, tokio, tower-http, serde deps)
- Create: `src/server/mod.rs`
- Create: `src/server/dto.rs`
- Create: `src/server/service.rs`
- Create: `src/server/routes.rs`
- Create: `src/bin/session-search-web.rs`
- Test: `tests/api.rs` (integration test hitting the router with a temp session-state dir)

**Interfaces:**
- Consumes: `index::SessionIndex`, `indexer::run`, `events_reader::scan`, `fts_query::build`, `paths::*`.
- Produces (JSON API, all read-only):
  - `GET /api/health` → `{ ok: true }`
  - `GET /api/index/stats` → `{ total, scanned, skipped }`
  - `POST /api/index/rebuild` → re-run indexer, return stats
  - `GET /api/sessions/recent?limit=` → `SessionDto[]`
  - `GET /api/sessions/date?from=YYYY-MM-DD&to=YYYY-MM-DD&limit=` → `SessionDto[]`
  - `GET /api/sessions/search?q=&limit=` → `{ items: SearchResultDto[] }` (400 if `q` empty)
  - `GET /api/sessions/:id` → `SessionDto` (404 if unknown)
  - `GET /api/sessions/:id/transcript` → `{ timeline: TimelineEntryDto[] }`
  - `GET /api/sessions/:id/resume-command` → `{ command: "copilot --resume=<id>" }`

**Dependencies to add:**

```toml
# Cargo.toml (additions)
axum = "0.7"
tokio = { version = "1", features = ["rt-multi-thread", "macros", "net"] }
tower-http = { version = "0.6", features = ["fs", "compression-gzip", "cors"] }
rust-embed = "8"        # used in Task 5 for asset embedding
open = "5"              # used in Task 5 to open the browser
```

- [ ] **Step 1: Write a failing API integration test**

```rust
// tests/api.rs — build a temp session-state dir with one workspace.yaml + events.jsonl,
// point the service at it, exercise the router with tower::ServiceExt::oneshot.
#[tokio::test]
async fn search_returns_snippet_with_stx_etx_markers() {
    // arrange temp dir + service + router
    // GET /api/sessions/search?q=dashboard
    // assert 200 and body.items[0].snippet contains '\u{2}' ... '\u{3}'
}

#[tokio::test]
async fn unknown_session_detail_is_404() { /* GET /api/sessions/nope -> 404 */ }
```

> Note: `paths::session_state_root()` currently hard-codes `~/.copilot/session-state`. To make the service testable and Docker-mountable, thread the root/cache paths through the service (constructor args or a `COPILOT_HOME` / `SESSION_SEARCH_CACHE_DIR` env override in `paths.rs`) rather than calling the globals directly inside the service. Add these env overrides in this task.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test api`
Expected: FAIL (service/routes not implemented).

- [ ] **Step 3: Implement `paths` env overrides, DTOs, service, routes**

```rust
// src/paths.rs (additions) — env overrides, keep home-dir default
pub fn session_state_root() -> PathBuf {
    if let Ok(home) = std::env::var("COPILOT_HOME") {
        return PathBuf::from(home).join("session-state");
    }
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
        .join(".copilot").join("session-state")
}
// index_cache_path(): honor SESSION_SEARCH_CACHE_DIR when set.
```

```rust
// src/server/dto.rs
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionDto { /* mirror SessionRecord, RFC3339 strings for dates */ }
#[derive(Serialize)]
pub struct SearchResultDto { pub session: SessionDto, pub snippet: Option<String> }
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineEntryDto { pub role: String, pub timestamp: Option<String>, pub text: String }
```

```rust
// src/server/service.rs
pub struct SessionService { index: RwLock<SessionIndex>, root: PathBuf }
impl SessionService {
    pub fn bootstrap() -> anyhow::Result<Arc<Self>> { /* open cache, indexer::run, wrap */ }
    pub fn recent(&self, limit: usize) -> Vec<SessionDto> { /* index.all */ }
    pub fn by_date(&self, from: DateTime<Utc>, to: DateTime<Utc>, limit: usize) -> Vec<SessionDto> {}
    pub fn search(&self, q: &str, limit: usize) -> Vec<SearchResultDto> { /* fts_query::build + search_keyword */ }
    pub fn get(&self, id: &str) -> Option<SessionDto> {}
    pub fn transcript(&self, id: &str) -> Option<Vec<TimelineEntryDto>> { /* events_reader::scan */ }
    pub fn resume_command(&self, id: &str) -> Option<String> { /* format!("copilot --resume={id}") */ }
    pub fn rebuild(&self) -> anyhow::Result<IndexStats> {}
}
```

```rust
// src/server/routes.rs
pub fn api_router(service: Arc<SessionService>) -> Router { /* the /api/* routes above */ }
```

- [ ] **Step 4: Run API tests to verify pass**

Run: `cargo test --test api`
Expected: PASS (search/detail/transcript/date/recent/resume-command behavior).

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml src/paths.rs src/server/mod.rs src/server/dto.rs src/server/service.rs src/server/routes.rs src/bin/session-search-web.rs tests/api.rs
git commit -m "feat: add axum read-only json api over existing session index"
```

---

### Task 3: Scaffold the React + Vite + TypeScript frontend (no PWA)

**Files:**
- Create: `web/package.json`, `web/tsconfig.json`, `web/vite.config.ts`, `web/index.html`
- Create: `web/postcss.config.js`, `web/tailwind.config.ts`
- Create: `web/src/main.tsx`, `web/src/App.tsx`, `web/src/styles.css`
- Create: `web/src/lib/types.ts`, `web/src/lib/api.ts`
- Create: `web/src/components/AppShell.tsx`, `web/src/components/TopNav.tsx`
- Modify: `.gitignore` (ignore `web/node_modules`, `web/dist`)

**Interfaces:**
- Consumes: `/api/*` from Task 2 (via Vite dev proxy).
- Produces:
  - `pnpm --dir web run dev` serves the UI on `5173`, proxying `/api` to the Rust server.
  - `pnpm --dir web run build` outputs `web/dist`.
  - `pnpm --dir web run typecheck` runs TS 7 (`tsgo`) typecheck.

- [ ] **Step 1: Write failing build/typecheck check**

Run: `pnpm --dir web run build`
Expected: FAIL (no `web/package.json` yet).

- [ ] **Step 2: Scaffold Vite React TS app (no PWA plugin)**

```json
// web/package.json
{
  "name": "session-search-web",
  "private": true,
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "vite build",
    "preview": "vite preview",
    "typecheck": "tsgo --noEmit",
    "test": "vitest run"
  },
  "dependencies": { "react": "^18.3.1", "react-dom": "^18.3.1", "react-router-dom": "^6.28.0" },
  "devDependencies": {
    "@types/react": "^18.3.14", "@types/react-dom": "^18.3.5",
    "@typescript/native-preview": "latest",
    "@vitejs/plugin-react": "^4.4.1",
    "@testing-library/react": "^16.1.0", "@testing-library/user-event": "^14.5.2",
    "autoprefixer": "^10.4.20", "jsdom": "^25.0.1",
    "postcss": "^8.4.49", "tailwindcss": "^3.4.17",
    "typescript": "^5.7.2", "vite": "^6.0.6", "vitest": "^2.1.8"
  }
}
```

> `typecheck` uses `tsgo` (TS 7 native preview). Vite still transpiles via esbuild, so the app builds regardless; `typescript` (5.x) stays as an editor/CI fallback until TS 7 fully GA's.

```ts
// web/vite.config.ts
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
export default defineConfig({
  plugins: [react()],
  server: { port: 5173, proxy: { "/api": "http://127.0.0.1:4173" } },
  build: { outDir: "dist", emptyOutDir: true },
});
```

Add `AppShell` (header + main container), `TopNav` (Recent / Keyword / Date NavLinks), a typed `api.ts` client, and `types.ts` mirroring the DTOs.

- [ ] **Step 3: Run dev server against the live Rust API (manual smoke)**

Run (two shells): `cargo run --bin session-search-web` then `pnpm --dir web run dev`
Expected: `http://localhost:5173` renders the shell; network tab shows `/api/sessions/recent` returning real sessions.

- [ ] **Step 4: Verify build + typecheck pass**

Run: `pnpm --dir web run build && pnpm --dir web run typecheck`
Expected: `web/dist` produced; typecheck clean.

- [ ] **Step 5: Commit**

```bash
git add web/package.json web/tsconfig.json web/vite.config.ts web/index.html web/postcss.config.js web/tailwind.config.ts web/src .gitignore
git commit -m "feat: scaffold react+vite+ts frontend with dev proxy to rust api"
```

---

### Task 4: Implement pages, search highlighting, and copy-command resume

**Files:**
- Create: `web/src/pages/RecentPage.tsx`, `web/src/pages/KeywordSearchPage.tsx`, `web/src/pages/DateSearchPage.tsx`, `web/src/pages/SessionDetailPage.tsx`
- Create: `web/src/components/SessionList.tsx`, `web/src/components/SessionCard.tsx`, `web/src/components/EmptyState.tsx`, `web/src/components/CopyButton.tsx`, `web/src/components/TranscriptView.tsx`
- Create: `web/src/lib/snippet.tsx`, `web/src/lib/format.ts`
- Modify: `web/src/App.tsx` (routes)
- Test: `web/tests/snippet.test.tsx`, `web/tests/flows.test.tsx`

**Interfaces:**
- Consumes: all `/api/*` endpoints.
- Produces:
  - Recent (default), Keyword search, Date-range pages listing sessions.
  - Session detail with metadata + transcript + **Copy `copilot --resume=<id>`** button with copied/confirm feedback.
  - Snippet highlighting: STX (`\u0002`) / ETX (`\u0003`) markers → `<mark>`.

- [ ] **Step 1: Write failing tests for snippet rendering and search→detail flow**

```tsx
// web/tests/snippet.test.tsx
it("renders STX/ETX runs as <mark>", () => {
  render(<Snippet text={"deploy \u0002dashboard\u0003 to prod"} />);
  expect(screen.getByText("dashboard").tagName).toBe("MARK");
});
```

```tsx
// web/tests/flows.test.tsx — mock fetch; type query, submit, click result, see detail + copy button.
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `pnpm --dir web run test`
Expected: FAIL (components/pages missing).

- [ ] **Step 3: Implement pages, snippet parser, date formatting, copy-to-clipboard**

```tsx
// web/src/lib/snippet.tsx — split on \u0002..\u0003, wrap matches in <mark>.
export function Snippet({ text }: { text: string }) {
  const parts = text.split(/\u0002([^\u0003]*)\u0003/g);
  return <>{parts.map((p, i) => (i % 2 ? <mark key={i}>{p}</mark> : <span key={i}>{p}</span>))}</>;
}
```

```tsx
// web/src/components/CopyButton.tsx — navigator.clipboard.writeText(command), show "Copied!" for ~1.5s,
// fall back to a selectable <code> if clipboard API is unavailable (non-secure context).
```

Date page offers quick ranges (Today / 7d / 30d / custom) → `from`/`to` query params. Detail page fetches `/api/sessions/:id` + `/api/sessions/:id/transcript` + `/api/sessions/:id/resume-command`.

- [ ] **Step 4: Run tests + build to verify pass**

Run: `pnpm --dir web run test && pnpm --dir web run build`
Expected: PASS and `web/dist` rebuilt.

- [ ] **Step 5: Commit**

```bash
git add web/src web/tests
git commit -m "feat: session browsing, highlighted snippets, and copy-resume-command ui"
```

---

### Task 5: Embed the built UI in the binary + auto-open browser (single self-contained executable)

**Files:**
- Create: `src/server/assets.rs`
- Modify: `src/server/routes.rs` (SPA fallback to embedded assets)
- Modify: `src/bin/session-search-web.rs` (port select, `open` browser, print URL)
- Modify: `Cargo.toml` (ensure `rust-embed`, `open`)
- Modify: `.gitignore` (do not ignore whatever the embed path needs at build time)

**Interfaces:**
- Consumes: `web/dist` (built in Task 4).
- Produces:
  - `cargo build --release` → one binary that serves API + embedded UI on `127.0.0.1:<port>` and opens the browser.
  - Windows packaging still works: `cargo build --release --target x86_64-pc-windows-gnu`.

- [ ] **Step 1: Manual failing check — release binary serves no UI yet**

Run: `cargo run --release --bin session-search-web` then open the printed URL.
Expected: API works but `/` returns 404/empty (assets not embedded).

- [ ] **Step 2: Embed `web/dist` and add SPA fallback**

```rust
// src/server/assets.rs
#[derive(rust_embed::RustEmbed)]
#[folder = "web/dist"]
pub struct WebAssets;
// Serve matched files; fall back to index.html for client-side routes (non-/api paths).
```

Wire into `routes.rs` so `/api/*` hits the API and everything else resolves against `WebAssets` (with `index.html` fallback for SPA deep links like `/sessions/:id`).

- [ ] **Step 3: Auto-open browser + friendly startup**

```rust
// src/bin/session-search-web.rs
let addr = "127.0.0.1:4173"; // or pick a free port; allow PORT env override
println!("Session Search running at http://{addr}");
let _ = open::that(format!("http://{addr}"));
// serve axum
```

Ensure the build embeds the current `web/dist`: document that `pnpm --dir web run build` must run before `cargo build --release` (Docker + a `build.rs`/Makefile/README note enforce ordering).

- [ ] **Step 4: Verify self-contained binary**

Run: `pnpm --dir web run build && cargo build --release --bin session-search-web` then run it.
Expected: browser opens; Recent/Keyword/Date/Detail/Copy all work from a single process with direct `~/.copilot` access. Also verify `cargo build --release --target x86_64-pc-windows-gnu` still links (mingw path).

- [ ] **Step 5: Commit**

```bash
git add src/server/assets.rs src/server/routes.rs src/bin/session-search-web.rs Cargo.toml .gitignore
git commit -m "feat: embed react ui in the server binary and auto-open browser"
```

---

### Task 6: Docker option, docs, and TUI cleanup

**Files:**
- Create: `Dockerfile`, `docker-compose.yml`, `.dockerignore`
- Modify: `README.md`
- Modify: `.gitignore`
- Decide: keep or remove TUI bin (`src/main.rs`, `src/ui/*`, `src/launcher.rs`)
- Modify: `.github/copilot-instructions.md` (or the repo Copilot instructions) to describe the new web architecture

**Interfaces:**
- Consumes: completed app from Tasks 1–5.
- Produces:
  - `docker compose up -d --build` serving the app with `~/.copilot` mounted read-only.
  - README covering native run (primary) + Docker (option), architecture, and TS 7 note.

- [ ] **Step 1: Acceptance checklist (manual)**

```text
1) Native: `pnpm --dir web run build && cargo run --release --bin session-search-web` opens browser, all flows work.
2) Docker: `docker compose up -d --build`, open http://localhost:4173, all flows work off mounted data.
3) Resume: detail page shows and copies `copilot --resume=<id>`; backend never spawns copilot.
4) Corrupt/missing workspace.yaml or bad events.jsonl line does not break indexing.
```

- [ ] **Step 2: Multi-stage Dockerfile (node build → rust build → slim runtime)**

```dockerfile
# Stage 1: build UI
FROM node:22-alpine AS web
WORKDIR /web
RUN corepack enable && corepack prepare pnpm@latest --activate
COPY web/package.json web/pnpm-lock.yaml* ./
RUN pnpm install --frozen-lockfile
COPY web/ ./
RUN pnpm run build

# Stage 2: build server (embeds web/dist)
FROM rust:1-alpine AS server
RUN apk add --no-cache musl-dev
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY --from=web /web/dist ./web/dist
RUN cargo build --release --bin session-search-web

# Stage 3: runtime
FROM alpine:3
COPY --from=server /app/target/release/session-search-web /usr/local/bin/session-search-web
ENV COPILOT_HOME=/data/.copilot SESSION_SEARCH_CACHE_DIR=/data/cache PORT=4173
EXPOSE 4173
# In-container we can't open a browser; add a NO_BROWSER/headless flag honored by the bin.
CMD ["session-search-web"]
```

```yaml
# docker-compose.yml
services:
  session-search:
    build: .
    ports: ["4173:4173"]
    environment:
      - COPILOT_HOME=/data/.copilot
      - SESSION_SEARCH_CACHE_DIR=/data/cache
      - NO_BROWSER=1
    volumes:
      - ${USERPROFILE:-$HOME}/.copilot:/data/.copilot:ro
      - session-search-cache:/data/cache
volumes:
  session-search-cache:
```

> Add a `NO_BROWSER`/headless guard in `session-search-web.rs` so the container doesn't try to `open` a browser. Confirm `rust-embed` in Docker uses the `web/dist` copied from the node stage.

- [ ] **Step 3: Update README + Copilot instructions; resolve TUI fate**

README: native binary as primary run path, Docker as option, architecture (reused Rust core + React UI), the "TS 7 = typecheck-only" note, and "never reads session-store.db". Update the repo Copilot instructions to reflect the web architecture (the current file describes a TUI). Decide TUI: either keep `session-search` bin as a secondary entrypoint (cheap, shares the lib) or remove `src/main.rs`/`src/ui/*`/`src/launcher.rs` — recommend keeping it since it reuses the same lib at near-zero cost.

- [ ] **Step 4: Full validation**

Run: `cargo test && pnpm --dir web run test && pnpm --dir web run build && cargo build --release && docker compose up -d --build`
Expected: tests pass, native binary self-serves, container serves off mounted data.

- [ ] **Step 5: Commit**

```bash
git add Dockerfile docker-compose.yml .dockerignore README.md .gitignore .github/copilot-instructions.md src
git commit -m "docs: web app runbook, docker option, and architecture update"
```

---

## Self-Review

1. **Spec coverage:**
   - Lightweight web app: Tasks 2–5 (Rust API + React UI, single binary).
   - Fastest realistic runtime: reuses the existing Rust indexing/search core (Tasks 1–2) instead of re-implementing in a slower runtime.
   - TypeScript where it helps: frontend in TS with TS 7 (`tsgo`) typecheck (Tasks 3–4); explicitly noted it's a build-time, not runtime, gain.
   - Packaging "both": native self-contained binary (Task 5) + Docker (Task 6).
   - No PWA: no service worker/manifest anywhere.
   - Resume = copy-command: `resume-command` endpoint + `CopyButton`, backend never spawns (Tasks 2, 4).

2. **Reuse honesty:** `models`/`paths`/`yaml_reader`/`events_reader`/`fts_query`/`index`/`indexer` reused; only additions are `Serialize` on `TimelineEntry`, DTOs, env path overrides, and the axum/embed layer. No `rusqlite`/`regex`/`serde_yaml` reintroduced.

3. **Risk notes:**
   - Build ordering: `web/dist` must exist before `cargo build` embeds it — enforced in Docker and documented; consider a `build.rs` guard or Make target.
   - `paths` globals made testable/mountable via env overrides — required for both `tests/api.rs` and Docker.
   - Windows `x86_64-pc-windows-gnu` packaging must still link after adding axum/tokio (verify in Task 5).

## Execution Handoff

**Plan saved to `docs/superpowers/plans/2026-07-15-rust-api-react-web-migration.md`. Two execution options:**

**1. Subagent-Driven (recommended)** — dispatch a fresh subagent per task, review between tasks.

**2. Inline Execution** — execute tasks in this session using executing-plans, batch with checkpoints.

**Which approach?**
