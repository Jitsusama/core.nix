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
# The shell runs with no frame callback timeout, as Joel's does, given by
# checks/helpers.nix from its unit: Qt's default takes a window as no
# longer shown once niri's frame callback is 100 ms late, and on a loaded
# builder Qt then found one and two windows shown at a stop and a start,
# and the still window drew one frame over the ten starts (21 November).
: "${QT_WAYLAND_FRAME_CALLBACK_TIMEOUT:?set by checks/helpers.nix from the shell unit}"

said=$logs/shell.log
ran=0
# The case may take up to 60 s settling, about 6.1 s for each of its ten
# starts to go quiet and 2 s for the still window's change, about 123 s,
# plus the shell's own start on a loaded builder; this leaves room for
# that, so a case that waits out every start still gives its own verdict
# rather than this.
timeout 180 qs -p "$shell" > "$said" 2>&1 || ran=$?
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
  echo "The shell failed (exit $ran), or didn't finish in 180 s (124):" >&2
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
# branch with all three shown, but only where it's the loop's next line
# on the GUI thread after its own "- animationStarted()": the loop starts
# its timer as a window is exposed too, and a start's timer line from
# that would say nothing of the branch. So each of the case's starts, from
# its own line to the next start's or the count's, is asked for that line,
# and at least half of the starts the case counted must have one. Not
# every start: on a loaded builder Qt logged a timer line with all three
# shown 8 times over a whole run of ten starts, one stop finding a window
# not shown, likely a late frame callback (2026-11-21, counted before
# this script asked for the line straight after animationStarted(), and
# before the timeout above). Counted this way, a run on a lender on
# 2026-11-22 found it in 10 of 10 starts. In Qt 6.12.0's
# qsgthreadedrenderloop.cpp, every other caller that starts the timer
# logs a line of the loop's own on the GUI thread first (animationStopped,
# handleExposure, handleObscurity, windowDestroyed, polishAndSync), so
# one can't pass for animationStarted()'s. The one exception is a polish
# that starts an animation in the same polishAndSync that then finds bad
# vsync: it says so first on qt.scenegraph.general, which QSG_INFO turns
# on (qsgrhisupport.cpp), so that line ends the start's too. Its timer
# line's badVSync is no mark of it, since it counts the windows ever
# found so (a flag Qt never clears), and would say 1 on every later
# start's line. Nor can that line say all three shown: with three,
# animationStarted() has already started the timer, and Qt starts it
# only once. The timer's line, the animationStarted() line before it,
# the broken vsync line and the (RT) mark on the render thread's lines
# are Qt's own wording, which a Qt bump may change: a
# render-thread line without the mark, or a new line from the GUI thread
# between the two, marks real starts -other and fails the case, never
# passes it.
if ! grep -q 'threaded render loop' "$said"; then
  echo "The shell didn't draw through Qt's threaded render loop." >&2
  exit 1
fi
# Prints the starts the case logged, the starts it counted, those with
# Qt's line, and those without it, each with the windows Qt found shown
# as it started its timer there, marked -other where that wasn't straight
# after animationStarted(), or "none" where it didn't start it.
counts=$(awk '
  /The case starts its squares moving, start / { inside = 1; started = 0; starts++; next }
  /Over [0-9]+ starts the still window drew/ {
    inside = 0
    counted = $0
    sub(/.*Over /, "", counted)
    sub(/ .*/, "", counted)
  }
  # The timer line is judged only straight after animationStarted()
  # says itself. Any other line from the loop on the GUI thread ends
  # that; those from the render thread, marked (RT), come between.
  inside && /- animationStarted\(\)/ { started = 1; next }
  started && /is determined to have broken vsync throttling/ { started = 0 }
  started && /qt\.scenegraph\.renderloop/ && !/\(RT\)/ &&
    !/Starting system \(not vsync-based\) animation timer/ { started = 0 }
  inside && /Starting system \(not vsync-based\) animation timer \(exposedWindows=/ {
    shown = $0
    sub(/.*exposedWindows=/, "", shown)
    sub(/ .*/, "", shown)
    ours = started
    seen[starts] = seen[starts] (seen[starts] == "" ? "" : ",") shown (ours ? "" : "-other")
    if (ours && shown == 3) {
      timed[starts] = 1
    }
    started = 0
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
    "(exposedWindows=3 ...)' never came straight after '- animationStarted()' in" \
    "starts $missing, the windows Qt found shown at that start's timer lines in" \
    "brackets, -other where one came from elsewhere." >&2
  exit 1
fi
