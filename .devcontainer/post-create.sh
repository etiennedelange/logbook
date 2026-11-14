#!/usr/bin/env bash
#
# Dev container postCreateCommand. The first group is what GPUI links against (xkbcommon, xcb, freetype);
# the second lets this container run the app in a virtual display and take screenshots of it
# (`pnpm screenshots`, app/tools/drive.sh): a software Vulkan driver, Xvfb, xdotool, ImageMagick and ffmpeg.
#
set -euo pipefail

cd "$(dirname "$0")/.."

sudo apt-get update
sudo apt-get install -y \
	libxkbcommon-dev \
	libxkbcommon-x11-dev \
	libxcb1-dev \
	libfreetype-dev \
	pkg-config \
	mesa-vulkan-drivers \
	libvulkan1 \
	xvfb \
	xdotool \
	imagemagick \
	webp \
	ffmpeg

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
