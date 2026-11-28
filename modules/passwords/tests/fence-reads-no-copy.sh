# shellcheck shell=bash
# Runs as Joel in his session on the desktop's VM (vm-desktop-keeps-copies),
# while Passwords holds a password it copied from the launcher, to show the
# clipboard door refuses it to a fenced command. A fenced command's
# wl-paste is the fence's own shim, which asks pi's clipboard door outside
# the fence (modules/pi/clipboard-door); niri tells the door a copy is a secret by
# the mark Passwords gives it, and the door refuses it. Joel's own paste
# reads it first, so it's held as the fenced read is tried, and an
# ordinary copy after it reaches the same fenced reader, so the refusal
# was the door's and not the reader's. That second copy ends Passwords'
# hold, as any copy over it does. The door's journal says it refused a
# secret, and never holds the secret. The fence is shaped as the
# harness's is for the clipboard's shims alone (bubblewrap with a pid
# namespace of its own, the shims first on PATH): it keeps Joel's runtime
# folder and niri's own socket, which the harness's fence empties, so it
# shows the door's refusal, not that no other way reaches niri from it
# (pi-fence-clipboard-on-niri holds that). X11, the other way to a copy,
# is held by pi-fence-keeps-x11-from-secrets in the same VM.
# usage: fence-reads-no-copy.sh <shims> <secret>
set -euo pipefail

shims=$1
secret=$2

fail() {
  echo "FAIL: $*" >&2
  [ -f "$logs/door.log" ] && sed 's|^|door.log: |' "$logs/door.log" >&2
  exit 1
}

logs=$(mktemp -d)
export PI_CLIPBOARD_DOOR=$logs/door.sock
# What this leaves running holds no output of the caller's, which waits for
# it to close: the door until the trap ends it, and wl-copy's server till
# the next copy.
pi-clipboard-door --socket "$PI_CLIPBOARD_DOOR" > /dev/null 2> "$logs/door.log" &
door=$!
trap 'kill "$door" 2> /dev/null || true; rm -rf "$logs"' EXIT
for _ in $(seq 100); do
  [ -S "$PI_CLIPBOARD_DOOR" ] && break
  sleep 0.1
done
[ -S "$PI_CLIPBOARD_DOOR" ] || fail "the clipboard door never listened"

# Joel's own paste, outside the fence.
[ "$(wl-paste --no-newline 2> /dev/null || true)" = "$secret" ] ||
  fail "Joel's own paste didn't read Passwords' copy, so a fenced one failing proves nothing"

# A command fenced as pi fences one, its session in bubblewrap's
# environment.
fenced() {
  PI_SESSION_ID=preflight bwrap --unshare-pid --dev-bind / / --proc /proc -- \
    env PATH="$shims:$PATH" bash -euo pipefail -c "$1"
}

# The door hears of a copy from niri a moment after it's made, and refuses
# for another reason until then: up to two seconds, in tenths, for the
# refusal that names a password.
said=
refused=no
for _ in $(seq 20); do
  # shellcheck disable=SC2310 # A read that fails is the refusal sought, not an error.
  if said=$(fenced "wl-paste --no-newline" 2>&1); then
    fail "a fenced command read Passwords' copy: $said"
  fi
  case $said in
  *password*)
    refused=yes
    break
    ;;
  *) ;;
  esac
  sleep 0.1
done
[ "$refused" = yes ] || fail "a fenced read was refused, but not as a secret: $said"

word="ordinary-$RANDOM-$RANDOM"
wl-copy "$word" > /dev/null 2>&1
read=
for _ in $(seq 20); do
  # shellcheck disable=SC2310 # A paste refused before niri tells the door is asked again.
  read=$(fenced "wl-paste --no-newline" 2> /dev/null) || read=
  [ "$read" = "$word" ] && break
  sleep 0.1
done
[ "$read" = "$word" ] || fail "an ordinary copy didn't reach the fenced reader, so its refusal proves nothing"

grep -q "^refused session=preflight selection=clipboard why=secret\$" "$logs/door.log" ||
  fail "the door's journal doesn't say it refused a secret"
! grep -qF "$secret" "$logs/door.log" || fail "the door's journal holds the secret"
echo "a fenced command reads no password Passwords copied"
