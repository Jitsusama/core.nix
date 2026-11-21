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
# that never reached the patched branch, which it says nothing about. A
# start in which Qt starts its timer with all three windows shown reaches
# the patch's early return, where unpatched Qt asks every window for a
# frame, so the still window drawing nothing then shows the patch held.
# A start whose timer was still going may reach it too, but says nothing
# Qt logs, and one under a single window's loop never does. So each of
# the case's starts, from its own line to the next start's or the count's,
# is asked for Qt's timer line, and at least half its starts must have
# one. Not every start: on a loaded builder Qt logged its timer start 8
# times over a run of ten starts, one stop finding a window not shown,
# its cause not known (2026-11-21). A window shown again inside a start
# could log the line too, which this can't tell from the start's own.
# The timer's line is Qt's own wording, which a Qt bump may change.
if ! grep -q 'threaded render loop' "$said"; then
  echo "The shell didn't draw through Qt's threaded render loop." >&2
  exit 1
fi
read -r starts timed < <(awk '
  /The case starts its squares moving, start / { inside = 1; starts++; next }
  /Over [0-9]+ starts the still window drew/ { inside = 0 }
  inside && /Starting system \(not vsync-based\) animation timer \(exposedWindows=3 / {
    timed[starts] = 1
  }
  END { n = 0; for (each in timed) n++; print starts + 0, n }' "$said")
echo "Qt started its animation timer with all three windows shown in $timed of $starts starts."
if [ "$starts" -eq 0 ] || [ $((timed * 2)) -lt "$starts" ]; then
  echo "That's fewer than half the case's starts: 'Starting system (not vsync-based)" \
    "animation timer (exposedWindows=3 ...)' after its 'The case starts its squares" \
    "moving' lines." >&2
  exit 1
fi
