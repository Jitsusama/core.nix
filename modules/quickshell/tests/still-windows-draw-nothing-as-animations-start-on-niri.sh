# shellcheck shell=bash
# Starts a shell of two windows (still-windows-draw-nothing-as-animations-start.qml,
# as the shell's root), one still and one whose square starts moving ten
# times, through Qt's threaded render loop as the laptop's shell draws. Qt
# 6.12.0 asks every window for a frame as animations start, and with more
# than one window shown a timer advances them, so the still window drew a
# frame showing nothing new each time; the shell's six windows did on the
# laptop (2026-11-20). The shell itself says what each window drew.
# usage: still-windows-draw-nothing-as-animations-start-on-niri.sh <shell folder> <logs>
set -euo pipefail

shell=$1
logs=$2
# Qt's threaded loop, where the frames were asked, rather than the basic
# one or Qt's software renderer, which draw on the GUI thread.
export QT_QUICK_BACKEND=rhi QSG_RHI_BACKEND=opengl QSG_RENDER_LOOP=threaded QSG_INFO=1
# The loop says when it starts the timer that advances animations.
export QT_LOGGING_RULES=qt.scenegraph.renderloop.debug=true

said=$logs/shell.log
if ! timeout 120 qs -p "$shell" > "$said" 2>&1; then
  echo "The shell failed, or didn't finish in 120 s:" >&2
  grep -v 'qt.scenegraph.renderloop' "$said" >&2
  exit 1
fi
grep 'drew' "$said"

# Without both, the case is another loop's, which it says nothing about.
if ! grep -q 'threaded render loop' "$said"; then
  echo "The shell didn't draw through Qt's threaded render loop." >&2
  exit 1
fi
if ! grep -q 'Starting system (not vsync-based) animation timer' "$said"; then
  echo "Qt's timer never advanced the animations, so no frame was asked of the still window." >&2
  exit 1
fi
