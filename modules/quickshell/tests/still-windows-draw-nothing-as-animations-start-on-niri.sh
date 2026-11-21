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
# The shell's own setting (home.nix's Environment, and its reason there):
# Qt's default takes a window as no longer shown once niri's frame callback
# is 100 ms late. On a loaded builder Qt then found one and two windows
# shown at a stop and a start, and the still window drew one frame over
# the ten starts (21 November), by its return or by the timer's stop
# asking the one window left shown. With none, every window stays shown.
export QT_WAYLAND_FRAME_CALLBACK_TIMEOUT=0

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

# Without these, the case is another loop's, or fewer windows', or its
# starts never reached the patched branch, which it says nothing about.
# The patch acts in animationStarted(), which Qt runs only as its driver
# starts again after stopping, and where it starts its timer: a start
# whose driver never stopped passes untested, and one with a window not
# shown draws nothing patched or not. Qt's line starting its timer with
# all three windows shown, inside a start, shows that start reached the
# branch with all three shown. So each of the case's starts, from its
# own line to the next start's or the count's, is asked for that line,
# and at least half of the starts the case counted must have one. Not
# every start: on a loaded builder Qt logged it 8 times over a whole run
# of ten starts, one stop finding a window not shown, likely a late frame
# callback (2026-11-21, before the timeout above), and how many of those fell within the starts that
# log doesn't say; a passing run logged 11 over the run and 10 within
# them. The timer's line is Qt's own wording, which a Qt bump may change.
if ! grep -q 'threaded render loop' "$said"; then
  echo "The shell didn't draw through Qt's threaded render loop." >&2
  exit 1
fi
# Prints the starts the case logged, the starts it counted, those with
# Qt's line, and those without it, each with the windows Qt found shown
# as it started its timer there, or "none" where it didn't start it.
counts=$(awk '
  /The case starts its squares moving, start / { inside = 1; starts++; next }
  /Over [0-9]+ starts the still window drew/ {
    inside = 0
    counted = $0
    sub(/.*Over /, "", counted)
    sub(/ .*/, "", counted)
  }
  inside && /Starting system \(not vsync-based\) animation timer \(exposedWindows=/ {
    shown = $0
    sub(/.*exposedWindows=/, "", shown)
    sub(/ .*/, "", shown)
    seen[starts] = seen[starts] (seen[starts] == "" ? "" : ",") shown
    if (shown == 3) {
      timed[starts] = 1
    }
  }
  END {
    n = 0
    missing = ""
    for (s = 1; s <= starts; s++) {
      if (s in timed) {
        n++
      } else {
        missing = missing " " s "(" (s in seen ? seen[s] : "none") ")"
      }
    }
    print starts + 0, counted + 0, n, missing
  }' "$said")
read -r starts counted timed missing <<< "$counts"
if [ "$counted" -eq 0 ]; then
  echo "The log holds no 'Over N starts the still window drew' line the script can read." >&2
  exit 1
fi
if [ "$starts" -ne "$counted" ]; then
  echo "The case counted $counted starts but logged 'The case starts its squares moving'" \
    "for $starts, so the log can't say which of Qt's lines came in which start." >&2
  exit 1
fi
echo "Qt started its animation timer with all three windows shown in $timed of $starts starts."
if [ $((timed * 2)) -lt "$starts" ]; then
  echo "That's fewer than half: 'Starting system (not vsync-based) animation timer" \
    "(exposedWindows=3 ...)' never came in starts $missing, the windows Qt found" \
    "shown there in brackets." >&2
  exit 1
fi
