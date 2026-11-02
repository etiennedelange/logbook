#!/usr/bin/env bash
#
# Dev container postCreateCommand. webkit2gtk/gtk3/etc. are Tauri build-time
# deps (pkg-config, linked by the tao/wry/webkit2gtk crates) — needed for
# `cargo build`/`cargo check` even though this container never runs or
# displays the GUI itself.
#
set -euo pipefail

cd "$(dirname "$0")/.."

sudo apt-get update
sudo apt-get install -y \
	libwebkit2gtk-4.1-dev \
	libgtk-3-dev \
	librsvg2-dev \
	libayatana-appindicator3-dev \
	libssl-dev \
	libxdo-dev \
	patchelf \
	pkg-config

# pnpm, OpenCode, the GitHub CLI and Wrangler come from the dev-setup
# feature.
pnpm install

# Headless Chromium and its system libraries, for `pnpm screenshots`.
pnpm --dir tools/screenshots exec playwright install --with-deps chromium

# The Playwright MCP plugin launches the "chrome" channel, which it expects at
# /opt/google/chrome/chrome. Google ships no Chrome for Linux ARM64, so point
# that path at Playwright's own Chromium (Chrome for Testing) when it's absent.
if [ ! -e /opt/google/chrome/chrome ]; then
	chromium="$(cd tools/screenshots && node -e "console.log(require('playwright').chromium.executablePath())")"
	sudo mkdir -p /opt/google/chrome
	sudo ln -sf "$chromium" /opt/google/chrome/chrome
fi
