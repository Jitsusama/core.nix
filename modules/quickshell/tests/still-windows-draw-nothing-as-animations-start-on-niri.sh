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
# What each window drew, and Qt's own lines on its timer, which say how
# many windows it found shown, said before every verdict below so a
# failure reads beside them, since the log goes with a failed build;
# neither ends the script when it's missing.
grep 'drew' "$said" || true
grep 'animation timer' "$said" | sed 's/.*\*\*\* //' | sort | uniq -c || true
# The case exits 2 for a verdict of its own, and anything else is the shell.
if [ "$ran" -eq 2 ]; then
  echo "The case failed:" >&2
  grep -v 'qt.scenegraph.renderloop' "$said" >&2
  exit 1
fi
if [ "$ran" -ne 0 ]; then
  echo "The shell failed (exit $ran), or didn't finish in 120 s (124):" >&2
  grep -v 'qt.scenegraph.renderloop' "$said" >&2
  exit 1
fi

# Without these, the case is another loop's, or fewer windows', or starts
# that never reached the patched branch, which it says nothing about: Qt
# goes there only as its animations start again after stopping, so its
# timer must have started with all three windows shown at least as often
# as the case starts them. Counted over the whole run, which starts it
# beyond the ten as well, so one start missing it can pass; that start's
# still window would then draw for the patch's undoing anyway. The
# timer's line is Qt's own wording, which a Qt bump may change.
if ! grep -q 'threaded render loop' "$said"; then
  echo "The shell didn't draw through Qt's threaded render loop." >&2
  exit 1
fi
timed=$(grep -c 'Starting system (not vsync-based) animation timer (exposedWindows=3 ' "$said" \
  || true)
if [ "$timed" -lt 10 ]; then
  echo "Qt started its animation timer with all three windows shown $timed times, fewer" \
    "than the ten starts: 'Starting system (not vsync-based) animation timer" \
    "(exposedWindows=3 ...)'." >&2
  exit 1
fi
