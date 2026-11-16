# shellcheck shell=bash
# Runs on a nested niri (tests/nested-niri.sh) whose PATH holds the
# session's xwayland-satellite, which bridges niri's clipboard to X11 for
# Joel's X11 programs. A copy a password manager marks secret reaches his
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
# usage: pi-fence-keeps-x11-from-secrets.sh <niri's log>
set -euo pipefail

log=$1
logs=$(dirname "$log")
exec 2> "$logs/x11.log"

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

for _ in $(seq 100); do
  display=$(grep -ao 'listening on X11 socket: :[0-9]*' "$log" | grep -o ':[0-9]*$' || true)
  [ -n "$display" ] && break
  sleep 0.1
done
[ -n "$display" ] || fail "niri opened no X11 display"

secret="secret-$RANDOM-$RANDOM"
wl-copy --sensitive "$secret"

# Joel's own X11 client, holding the cookie, reads his copy through the
# satellite. The first client starts the satellite and Xwayland behind it,
# so it may wait for them: up to a minute, in tenths.
read=
for _ in $(seq 600); do
  read=$(timeout 10 xclip -o -selection clipboard -display "$display" 2> /dev/null) || read=
  [ "$read" = "$secret" ] && break
  sleep 0.1
done
[ "$read" = "$secret" ] ||
  fail "Joel's own X11 client didn't read his copy, so a fenced one failing proves nothing"

# The cookie, for the case that hands it to a fenced command.
cookie=$PWD/handed-cookie
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
  timeout 10 "${fence[@]}" -- "${landlock[@]}" \
    env DISPLAY="$display" XAUTHORITY="$auth" xclip -o -selection clipboard
}

# Whether a fenced read is refused, saying what it read when it isn't.
refused() {
  local said
  if said=$(fenced_read "$1" "$2" 2>&1); then
    echo "the fenced command read: $said" >&2
    return 1
  fi
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
