# shellcheck shell=bash
# Starts a shell of three windows (still-windows-draw-nothing-as-animations-start.qml,
# as the shell's root), one still and two whose squares start moving ten
# times, one by a NumberAnimation and one by an XAnimator, through Qt's
# threaded render loop as the laptop's shell draws. Qt 6.12.0 asks every
# window for a frame as animations start, and while its system timer
# advances them, as with more than one window shown, the still window drew
# a frame showing nothing new each time; the shell's six windows did on the
# laptop (2026-11-20). The shell itself says what each window drew.
# usage: still-windows-draw-nothing-as-animations-start-on-niri.sh <shell folder> <logs>
set -euo pipefail

shell=$1
logs=$2
# Qt's threaded loop, where the frames were asked, rather than the basic
# one or Qt's software renderer, which draw on the GUI thread.
export QT_QUICK_BACKEND=rhi QSG_RHI_BACKEND=opengl QSG_RENDER_LOOP=threaded QSG_INFO=1
# The loop says when it starts the timer that advances animations, and
# which window it asks for each frame; they stay in the log for whoever
# reads a failure.
export QT_LOGGING_RULES=qt.scenegraph.renderloop.debug=true

said=$logs/shell.log
ran=0
timeout 120 qs -p "$shell" > "$said" 2>&1 || ran=$?
# The case exits 2 for a verdict of its own, and anything else is the shell.
if [ "$ran" -eq 2 ]; then
  echo "The case failed (Qt's render-loop lines are in $said):" >&2
  grep -v 'qt.scenegraph.renderloop' "$said" >&2
  exit 1
fi
if [ "$ran" -ne 0 ]; then
  echo "The shell failed (exit $ran), or didn't finish in 120 s (124):" >&2
  grep -v 'qt.scenegraph.renderloop' "$said" >&2
  exit 1
fi
grep 'drew' "$said"
# Qt's own lines on its timer, which say how many windows it found shown.
grep 'animation timer' "$said" | sed 's/.*\*\*\* //' | sort | uniq -c

# Without both, the case is another loop's, or fewer windows', which it
# says nothing about. The second is Qt's own wording, which a Qt bump may
# change.
if ! grep -q 'threaded render loop' "$said"; then
  echo "The shell didn't draw through Qt's threaded render loop." >&2
  exit 1
fi
if ! grep -q 'Starting system (not vsync-based) animation timer (exposedWindows=3 ' "$said"; then
  echo "Qt never said it started its animation timer with all three windows shown:" \
    "'Starting system (not vsync-based) animation timer (exposedWindows=3 ...)'." >&2
  exit 1
fi
