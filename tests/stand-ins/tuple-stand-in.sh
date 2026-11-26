# shellcheck shell=bash disable=SC2154 # The check's session sets XDG_RUNTIME_DIR (people.nix).
# tuple, for the desktop's check of Tuple's call (the-desktop-hears-tuples-call
# in modules/quickshell/checks/people.nix), as tuple-call runs it: its stream
# says each line of $XDG_RUNTIME_DIR/stream as it's written, `new` opens a
# room, and every other command is kept a line each in $XDG_RUNTIME_DIR/asked.
#
# Each camera command is refused, as Tuple refuses one, from the second time
# it's asked on, which the list of asks says without a race, its reason
# holding what markup reads. From the third, it's refused only once the test
# writes $XDG_RUNTIME_DIR/go-<start or stop>-<time asked>, so the test changes
# the call while Tuple has yet to answer.

if [ "${3:-} ${4:-}" = "state follow" ]; then
  exec tail -n +1 -f "$XDG_RUNTIME_DIR/stream"
fi
asked="$XDG_RUNTIME_DIR/asked"
echo "$*" >>"$asked"
if [ "$*" = new ]; then
  echo "Joining https://tuple.app/c/opened"
fi
case "$*" in
"webcam stop" | "webcam start")
  n=$(grep -cx -- "$*" "$asked")
  if [ "$n" -gt 1 ]; then
    until [ "$n" -lt 3 ] || [ -e "$XDG_RUNTIME_DIR/go-$2-$n" ]; do
      # Looks for the test's go about as often as a frame is drawn.
      sleep 0.05
    done
    echo "no webcam to $2 <&>" >&2
    exit 1
  fi
  ;;
*)
  # Anything else Tuple does at once, saying nothing.
  ;;
esac
