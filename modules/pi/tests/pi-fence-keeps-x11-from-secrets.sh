# shellcheck shell=bash
# Runs as Joel in his session on the desktop's VM (vm-desktop-keeps-copies),
# whose kernel has the Landlock the fence needs, as his machines' does and
# the builders' doesn't. The session's xwayland-satellite bridges niri's
# clipboard to X11 for Joel's X11 programs. A copy a password manager marks secret reaches his
# own X11 client through it, as before, but never a command pi fences,
# though it's given the display: the fence lays an empty folder over X's
# sockets and empties the runtime folder the cookie is kept in, and
# Landlock refuses the abstract socket. Each of two walls holds alone:
# handed the cookie, the fenced command still reaches no socket, and with
# the sockets open it still holds no cookie. The empty folder can't be
# shown alone this way, since Xlib tries the abstract socket first and
# gives up at Landlock's refusal, never trying the file socket. Only with
# both walls taken away does the same reader reach X, which shows the
# reader would. The fence here is shaped as the harness's is
# (agentic-harness.pi/lib/internal/fence/fence.ts: bubblewrap, then
# landrun with only named sockets), and the harness's preflight holds the
# fence itself to it.
#
# The walls are tried twice. First as soon as X answers Joel's own client,
# which needs no window to hold the focus, so a run whose focus never
# comes still proves them. Then with a secret copy on the clipboard, which
# the satellite is offered only while one of its windows holds the
# keyboard. Each phase says a line of its own once it holds ("WALLS HOLD"
# and "FOCUSED"), so the test reads what was tried, never only that a
# failure went unsaid. Each wait is bounded by the clock, not by tries,
# and stretched by X11_PACE, the host's busyness as the test reads it, so
# the whole run ends inside the three minutes the test gives it, paced
# the same.
# usage: X11_PACE=<whole number> pi-fence-keeps-x11-from-secrets.sh
set -euo pipefail

pace=${X11_PACE:-1}
# Seconds a wait of $1 may take on this host.
paced() {
  echo $(($1 * pace))
}
# How long one fenced read may take.
read_wait=$(paced 10)

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

# The display and the cookie as niri and the session hand them to Joel's
# programs.
session=$(systemctl --user show-environment)
display=$(sed -n 's/^DISPLAY=//p' <<< "$session")
XAUTHORITY=$(sed -n 's/^XAUTHORITY=//p' <<< "$session")
export XAUTHORITY
[ -n "$display" ] || fail "niri opened no X11 display"
[ -n "$XAUTHORITY" ] || fail "the session names no X11 cookie"
runtime="${XDG_RUNTIME_DIR:?the session names no runtime folder}"

# The cookie, for the case that hands it to a fenced command, is a copy
# outside the runtime folder. The window, the cookie and the secret copy
# all go when this ends, the trap set before the cookie is copied so no
# early end leaves one behind.
cookie=$(mktemp)
logo=
leave() {
  [ -z "$logo" ] || kill "$logo" 2> /dev/null || true
  rm -f "$cookie"
  wl-copy --clear || true
}
trap leave EXIT
cp "$XAUTHORITY" "$cookie"

# An X11 window of Joel's, as whenever he works in one. The first client
# starts the satellite and Xwayland behind it, so X may take a while to
# answer.
xlogo -display "$display" &
logo=$!

# Whether what a reader said is X refusing the connection, in Xlib's and
# xclip's words alone: xclip's line that it can't open the display, after
# Xlib's that the cookie was wanted where the socket was reached without
# one. Nothing else may be said, so no read that got through and then
# failed passes for one, whatever its text held.
unreached() {
  local lines=() line
  while IFS= read -r line; do
    [ -z "$line" ] || lines+=("$line")
  done <<< "$1"
  local refusal="Error: Can't open display"
  local unauthorised="Authorization required, but no authorization protocol specified"
  case ${#lines[@]} in
  1) [[ ${lines[0]} == "$refusal"* ]] ;;
  2) [ "${lines[0]}" = "$unauthorised" ] && [[ ${lines[1]} == "$refusal"* ]] ;;
  *) return 1 ;;
  esac
}

# Whether a reader with nothing to read reached X, which then says, in
# xclip's own words, that the clipboard holds nothing of the kind asked.
# Anything else, a reader that timed out and said nothing, or a fence or
# reader that failed before trying, reached nothing.
answered() {
  case $1 in
  *"Error: target "*" not available"*) return 0 ;;
  *) return 1 ;;
  esac
}

# Whether the clock has passed a deadline, in the shell's seconds.
past() {
  [ "$SECONDS" -ge "$1" ]
}

# Runs xclip, as a fenced command would, given the display and a cookie,
# in a runtime folder emptied as the fence empties it: $1 says whether the
# walls round X's sockets stand (walled or open), $2 which cookie it's
# pointed at (the session's, gone with the runtime folder, or handed, a
# copy outside it).
fenced_read() {
  local sockets=$1 auth=$XAUTHORITY
  if [ "$2" = handed ]; then auth=$cookie; fi
  local fence=(bwrap --unshare-pid --dev-bind / / --proc /proc --tmpfs "$runtime")
  local landlock=()
  if [ "$sockets" = walled ]; then
    fence+=(--tmpfs /tmp/.X11-unix)
    landlock=(landrun --rwx / --unrestricted-network --ignore-missing --env PATH --)
  fi
  # env by its full path: landrun 0.1.17, given --env, finds no bare
  # command named env, and the read would fail without reaching X.
  timeout "$read_wait" "${fence[@]}" -- "${landlock[@]}" \
    "$env" DISPLAY="$display" XAUTHORITY="$auth" xclip -o -selection clipboard
}
env=$(command -v env)

# Whether a fenced read is refused, saying what it read when it isn't.
# What was read is quoted onto one line wherever it's told, so nothing
# read can pass for a phase's line.
refused() {
  local said ended=0
  # shellcheck disable=SC2310 # A read that fails is the refusal sought, not an error.
  said=$(fenced_read "$1" "$2" 2>&1) || ended=$?
  if [ "$ended" -eq 0 ]; then
    printf 'the fenced command read: %q\n' "$said" >&2
    return 1
  fi
  # A read cut off says nothing, so its cause is told apart from X's refusal.
  [ "$ended" -ne 124 ] ||
    fail "the fenced read ($1, $2 cookie) ran past $read_wait s: a slow fence, or one that let it reach an X that never answered"
  # Only X refusing the connection counts, never a reader that didn't run.
  # shellcheck disable=SC2310 # unreached is a test, read in a condition.
  unreached "$said" ||
    fail "the fenced command reached X ($1, $2 cookie), or failed before trying, ending $ended: $(printf '%q' "$said")"
  echo "refused ($1, $2 cookie): $(printf '%q' "$said")" >&2
}

# Each wall, then neither: $1 names the phase in what fails.
walls() {
  # shellcheck disable=SC2310 # refused is a step, read in a condition to say which failed.
  refused walled session || fail "$1: a fenced command read Joel's clipboard through X11"
  # shellcheck disable=SC2310 # As above.
  refused walled handed || fail "$1: a fenced command handed the cookie reached X11's socket"
  # shellcheck disable=SC2310 # As above.
  refused open session ||
    fail "$1: a command with X11's sockets open read through them without the cookie"
}

# X answers Joel's own client once the satellite is up, whatever the
# clipboard holds: up to 40 seconds, paced.
said=
reached=
until=$((SECONDS + $(paced 40)))
# shellcheck disable=SC2310 # past only compares the clock, which is the loop's test.
until past "$until"; do
  # A read that succeeds reached X, though what it read was empty.
  if said=$(timeout 5 xclip -o -selection clipboard -display "$display" 2>&1); then
    reached=yes
    break
  fi
  # shellcheck disable=SC2310 # A reader X didn't answer is the wait's to retry.
  if answered "$said"; then
    reached=yes
    break
  fi
  sleep 0.1
done
[ -n "$reached" ] || fail "X never answered Joel's own X11 client on $display: $said"

walls "before the focus"
# With both walls taken away the same reader reaches X, so each refusal
# above was the fence's and not the reader's.
# shellcheck disable=SC2310 # A read X refused is said by the check below.
said=$(fenced_read open handed 2>&1) || answered "$said" ||
  fail "with no walls and the cookie, the reader still didn't reach X: $said"
echo "WALLS HOLD: a fenced command reaches no X11 on $display" >&2

# The satellite is offered niri's clipboard only while one of its windows
# holds the keyboard, as any Wayland client is, and then holds it for every
# X11 client, focused or not: up to 30 seconds, paced.
focused=
until=$((SECONDS + $(paced 30)))
# shellcheck disable=SC2310 # As above.
until past "$until"; do
  focused=$(timeout 5 niri msg focused-window 2> /dev/null || true)
  case $focused in
  *[Xx][Ll]ogo*) break ;;
  *) ;;
  esac
  sleep 0.1
done
case $focused in
*[Xx][Ll]ogo*) ;;
*) fail "niri focused no X11 window: $focused" ;;
esac
echo "FOCUSED: niri focused Joel's X11 window" >&2

secret="secret-$RANDOM-$RANDOM"
wl-copy --sensitive "$secret"

# Joel's own X11 client, holding the cookie, reads his copy through the
# satellite, once the satellite has been offered it: up to 20 seconds,
# paced.
read=
until=$((SECONDS + $(paced 20)))
# shellcheck disable=SC2310 # As above.
until past "$until"; do
  read=$(timeout 5 xclip -o -selection clipboard -display "$display" 2> /dev/null) || read=
  [ "$read" = "$secret" ] && break
  sleep 0.1
done
[ "$read" = "$secret" ] ||
  fail "Joel's own X11 client didn't read his copy, so a fenced one failing proves nothing"

walls "with a secret copy"
# shellcheck disable=SC2310 # A failed read is said by the check below.
read=$(fenced_read open handed) || read=
[ "$read" = "$secret" ] ||
  fail "with no walls and the cookie, the reader still couldn't read the copy"
echo "a fenced command reads no secret through X11 on $display"
