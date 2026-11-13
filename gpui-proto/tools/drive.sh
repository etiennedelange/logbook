#!/bin/bash
# Drives the app in a virtual display and takes screenshots, to see a change working without a Windows box.
# Needs Xvfb, xdotool and ImageMagick's `import`.
#
#   tools/drive.sh <fixtures-dir> <out-dir> step...
#
# steps:  c:X,Y click · t:text type · k:key press (xdotool names, e.g. k:Down k:ctrl+0) · w:SECONDS wait
#         wheel:X,Y,up|down,N scroll · z:up|down,N Ctrl+scroll (zoom) · s:name screenshot to <out-dir>/name.png
# Coordinates are absolute in the 1400x850 screen. BIN=release uses the release build; extra env
# (LOGBOOK_TRACE=1, LOGBOOK_DEV_UPDATE=1) passes through to the app.
set -u
FX=$(realpath "$1"); OUT=$(mkdir -p "$2" && realpath "$2"); shift 2
HERE=$(cd "$(dirname "$0")/.." && pwd)
export COPILOT_HOME=$FX/copilot CLAUDE_CONFIG_DIR=$FX/claude LOGBOOK_CACHE_DIR=$FX/cache DISPLAY=:97
Xvfb :97 -screen 0 1400x850x24 >/dev/null 2>&1 & XV=$!
sleep 1
"$HERE/target/${BIN:-debug}/logbook-gpui-proto" > "$OUT/app.log" 2>&1 & APP=$!
sleep 9
xdotool mousemove 700 400
for step in "$@"; do
  case "$step" in
    k:*) xdotool key --clearmodifiers "${step#k:}"; sleep 0.6;;
    t:*) xdotool type --delay 60 "${step#t:}"; sleep 0.8;;
    c:*) IFS=, read x y <<< "${step#c:}"; xdotool mousemove "$x" "$y"; sleep 0.2; xdotool click 1; sleep 0.8;;
    w:*) sleep "${step#w:}";;
    s:*) import -window root "$OUT/${step#s:}.png";;
    z:*) IFS=, read dir n <<< "${step#z:}"; b=4; [ "$dir" = down ] && b=5
         xdotool keydown ctrl; for _ in $(seq "$n"); do xdotool click $b; sleep 0.15; done; xdotool keyup ctrl; sleep 1;;
    wheel:*) IFS=, read x y dir n <<< "${step#wheel:}"; xdotool mousemove "$x" "$y"; b=4; [ "$dir" = down ] && b=5
             for _ in $(seq "$n"); do xdotool click $b; done; sleep 0.8;;
  esac
done
echo "rss: $(ps -o rss= -p $APP | awk '{printf "%.0f MB", $1/1024}')  (a software-rendered GPU inflates this)"
kill $APP $XV
