---
name: logbook-site
description: Marketing site for Logbook, a Windows app that searches and resumes GitHub Copilot CLI sessions
colors:
  ground: "#2e3440"
  elevated: "#3b4252"
  highlight: "#434c5e"
  hairline: "#4c566a"
  fg: "#eceff4"
  fg-muted: "#d8dee9"
  fg-subtle: "#b2bccd"
  signal: "#ceb0f2"
  success: "#a3be8c"
  warning: "#ebcb8b"
  danger: "#d07079"
  mark: "#f5d90a"
  cat-teal: "#4eb9ad"
  cat-blue: "#7bd2f3"
  cat-indigo: "#86a4e4"
  cat-violet: "#d1b6f3"
  cat-rose: "#d28bb3"
typography:
  body:
    fontFamily: "IBM Plex Sans, ui-sans-serif, system-ui, sans-serif"
    fontSize: "1rem"
    fontWeight: 400
    lineHeight: 1.55
  headline:
    fontFamily: "IBM Plex Sans, ui-sans-serif, system-ui, sans-serif"
    fontSize: "1.625rem"
    fontWeight: 700
    letterSpacing: "-0.02em"
  label:
    fontFamily: "JetBrains Mono, ui-monospace, SFMono-Regular, Menlo, monospace"
    fontSize: "0.75rem"
    fontWeight: 500
    letterSpacing: "0.05em"
  numeric:
    fontFamily: "JetBrains Mono, ui-monospace, SFMono-Regular, Menlo, monospace"
    fontWeight: 700
rounded:
  md: "8px"
  lg: "10px"
---
# Design System: logbook-site

Inherits the personal brand (personal-context `design/brand.md`, tokens v2). This file records only what this project chooses; brand rules are not repeated here.

## Choices

- **Register:** Instrument: hairline borders, tonal surfaces, radii from the brand. Deliberately not Stamp: no outlines with hard shadows, no square corners.
- **Grounds:** Nord (dark), Noctis Lux (light)
- **Signal:** `violet` (curated option)
- **Categories:** the brand's five (`--cat-1` to `--cat-5`, then `--cat-other`), in order
- **Mark:** not used. The lemon mark and everything that draws it (wordmark dot, selection, stamp fills) are switched off in `src/styles/site.css`.

## Creative North Star

A ship's log kept on your own machine: it finds the entry you half remember and takes you back to it. It refuses to be a service: nothing leaves the machine, so the page makes no claim it can't back with a file name or a command.

## Project-specific rules

- **Violet signal** matches the app's own violet accent, so the screenshots and the page read as one product.
- The wordmark is plain `logbook` in JetBrains Mono 700. The favicon is the app's own icon (`src-tauri/app-icon.svg`).
- The app's UI keeps its own theme (`ui/src/theme.css`); only this site follows the brand.
- **Mono means a literal.** JetBrains Mono is for things you could type or find on disk: commands, keys,
  file names, the version. Never for labels or decoration, so there is no eyebrow above the headline.
- **Show the command, not a claim about it.** The hero shows the exact line Enter runs
  (`<folder>>copilot --resume=<id>`, as `wt -d … cmd /k` starts it), for the session open in the screenshot below it.
- **Screenshots are generated** (`pnpm screenshots`), never cropped or retouched by hand. Whole-window shots get
  a hairline and radius drawn in and no backdrop; crops (`search-detail`, `*-narrow`) carry no frame, and the page
  frames them with `--border` and `--radius-lg`. Phones get the list-pane crops, because the whole window there
  would shrink its text to about 3px.
- **One motion moment:** the search clip (`search-clip-*.webm`), recorded by the same script from the real UI.
  It plays once when scrolled into view, stays under five seconds, and has a Replay button. Phones, reduced
  motion and no-JS get the still.

