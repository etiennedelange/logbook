# UI Redesign — Linear aesthetic, Ctrl+K palette, dark mode, PWA

**Date:** 2026-07-15  
**Scope:** `web/` frontend only — no Rust/API changes.

---

## Goals

1. Modern, sleek Linear-inspired aesthetic with OS-preference dark/light mode.
2. Full keyboard-driven command palette (Ctrl+K) for search + navigation.
3. Installable PWA (offline app-shell, installable from browser).

---

## Theme & Design System

- Tailwind extended with CSS custom properties defined on `:root` (light) and a `dark` media query variant.
- **Light palette**: background `#ffffff`, surface `#f5f5f7`, border `#e4e4e7`, text `#09090b`, muted `#71717a`, accent `#7c3aed`.
- **Dark palette**: background `#09090b`, surface `#141417`, border `#27272a`, text `#fafafa`, muted `#71717a`, accent `#8b5cf6`.
- Mode selection: `@media (prefers-color-scheme: dark)` — no manual toggle, follows OS.
- Typography: system font stack; monospace for IDs, code, resume command.

---

## Layout

- **Sidebar nav** replaces the top nav bar: fixed left, 220 px wide on desktop, collapses to icon-only on mobile (bottom tab bar).
- Sidebar items: 🕐 Recent · 🔍 Search · 📅 Date — with `⌘K` hint at the bottom.
- **Content pane**: `max-w-3xl`, centred, `px-6 py-8`.
- `AppShell` updated to flex-row; `TopNav` replaced by `Sidebar` component.

---

## Components changed

| Component | Change |
|---|---|
| `AppShell` | Flex-row layout wrapping `Sidebar` + content |
| `TopNav` | **Removed** → replaced by `Sidebar` |
| `Sidebar` | **New** — nav links, ⌘K hint, dark/light indicator |
| `SessionCard` | Borderless card with surface background, hover ring, pill badges for repo/branch, relative timestamp |
| `EmptyState` | Centered illustration + text, no dashed border |
| `TranscriptView` | Chat bubbles: user = accent-tinted right-aligned, assistant = surface left-aligned |
| `CopyButton` | Icon-only button with tooltip |

---

## Command Palette (`CommandPalette` — new component)

- Library: `cmdk` (zero-JS-framework dependency, accessible).
- Opens: `Ctrl+K` (Windows/Linux) and `⌘K` (Mac) — global `keydown` listener in `App.tsx`.
- Closes: `Escape`, click-outside, item selection.
- **Groups:**
  - **Navigate** — Recent, Search, Date (always shown)
  - **Sessions** — live search via `GET /api/sessions/search?q=<input>`, debounced 200 ms, top 8 results shown; each item navigates to `/sessions/:id`
- Keyboard: arrow keys, Enter to select — handled by `cmdk` natively.
- Rendered in a `<dialog>` portal with backdrop blur overlay.

---

## PWA

- Plugin: `vite-plugin-pwa` (add to `web/package.json` devDependencies).
- `manifest.webmanifest`: `name: "Session Search"`, `short_name: "Sessions"`, `theme_color: "#7c3aed"`, `background_color: "#09090b"`, icon set (192×192, 512×512 — SVG-generated PNGs).
- Service worker strategy: `generateSW` with `NavigateFallback: index.html` (SPA shell). API calls (`/api/*`) are **not** cached — pass-through only.
- Offline behaviour: app shell loads; list/search pages show a "You're offline" notice when fetch fails.

---

## Dependencies added

| Package | Role |
|---|---|
| `cmdk` | Command palette primitives |
| `vite-plugin-pwa` | PWA manifest + service worker generation |

No other new runtime or dev dependencies.

---

## Out of scope

- No Rust/API changes.
- No manual dark/light toggle (OS preference only).
- No `copilot` process spawning — resume is copy-command only (unchanged).
- No new routes or API endpoints.
