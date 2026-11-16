# shellcheck shell=bash
# Runs as Joel in his session on the desktop's VM (vm-desktop-keeps-copies),
# whose kernel has the Landlock the fence needs, as his machines' does and
# the builders' doesn't. The session's xwayland-satellite bridges niri's
# clipboard to X11 for Joel's X11 programs. A copy a password manager marks secret reaches his
# own X11 client through it, as before, but never a command pi fences,
# though it's given the display: the fence lays an empty folder over X's
# sockets and empties the runtime folder the cookie is kept in, and
# Landlock refuses the abstract socket. Each wall holds alone: handed the
# cookie, the fenced command still reaches no socket, and with the sockets
# open it still holds no cookie. Only with both taken away does it read
# the secret, which shows the reader would. The fence here is shaped as
# the harness's is (lib/internal/fence.ts: bubblewrap, then landrun with
# only named sockets), and the harness's preflight holds the fence
# itself to it.
# usage: pi-fence-keeps-x11-from-secrets.sh
set -euo pipefail

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

# The satellite is offered niri's clipboard only while one of its windows
# holds the keyboard, as any Wayland client is, and then holds it for every
# X11 client, focused or not. So an X11 window of Joel's is open and
# focused, as it is whenever he works in one. The first client starts the
# satellite and Xwayland behind it, so it may wait for them: up to a
# minute, in tenths.
# The cookie, for the case that hands it to a fenced command, is a copy
# outside the runtime folder. The window, the cookie and the secret copy
# all go when this ends.
cookie=$(mktemp)
xlogo -display "$display" &
logo=$!
trap 'kill "$logo" 2> /dev/null || true; rm -f "$cookie"; wl-copy --clear || true' EXIT
for _ in $(seq 600); do
  focused=$(niri msg focused-window 2> /dev/null || true)
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

secret="secret-$RANDOM-$RANDOM"
wl-copy --sensitive "$secret"

# Joel's own X11 client, holding the cookie, reads his copy through the
# satellite, once the satellite has been offered it: up to a minute, in
# tenths.
read=
for _ in $(seq 600); do
  read=$(timeout 10 xclip -o -selection clipboard -display "$display" 2> /dev/null) || read=
  [ "$read" = "$secret" ] && break
  sleep 0.1
done
[ "$read" = "$secret" ] ||
  fail "Joel's own X11 client didn't read his copy, so a fenced one failing proves nothing"

cp "$XAUTHORITY" "$cookie"

# Runs xclip, as a fenced command would, given the display and a cookie,
# in a runtime folder emptied as the fence empties it: $1 says whether the
# walls round X's sockets stand (walled or open), $2 which cookie it's
# pointed at (the session's, gone with the runtime folder, or handed, a
# copy outside it).
fenced_read() {
  local sockets=$1 auth=$XAUTHORITY
  if [ "$2" = handed ]; then auth=$cookie; fi
  local fence=(bwrap --unshare-pid --dev-bind / / --proc /proc --tmpfs "$XDG_RUNTIME_DIR")
  local landlock=()
  if [ "$sockets" = walled ]; then
    fence+=(--tmpfs /tmp/.X11-unix)
    landlock=(landrun --rwx / --unrestricted-network --ignore-missing --env PATH --)
  fi
  # env by its full path: landrun 0.1.17, given --env, finds no bare
  # command named env, and the read would fail without reaching X.
  timeout 10 "${fence[@]}" -- "${landlock[@]}" \
    "$env" DISPLAY="$display" XAUTHORITY="$auth" xclip -o -selection clipboard
}
env=$(command -v env)

# Whether a fenced read is refused, saying what it read when it isn't.
refused() {
  local said
  if said=$(fenced_read "$1" "$2" 2>&1); then
    echo "the fenced command read: $said" >&2
    return 1
  fi
  # Only X refusing the connection counts, never a reader that didn't run.
  case $said in
  *"Can't open display"*) ;;
  *) fail "the fenced read failed before it reached X ($1, $2 cookie): $said" ;;
  esac
  echo "refused ($1, $2 cookie): $said" >&2
}

# shellcheck disable=SC2310 # refused is a step, read in a condition to say which failed.
refused walled session || fail "a fenced command read Joel's secret through X11"
# shellcheck disable=SC2310 # As above.
refused walled handed || fail "a fenced command handed the cookie reached X11's socket"
# shellcheck disable=SC2310 # As above.
refused open session || fail "a command with X11's sockets open read through them without the cookie"
# Both taken away, the same reader reads it, so each refusal above was
# the fence's and not the reader's.
[ "$(fenced_read open handed)" = "$secret" ] ||
  fail "with no walls and the cookie, the reader still couldn't read the copy"
echo "a fenced command reads no secret through X11 on $display"
