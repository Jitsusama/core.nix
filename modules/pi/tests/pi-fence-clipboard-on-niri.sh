# shellcheck shell=bash
# shellcheck disable=SC2310 # fenced is a step, read in a condition to say which failed.
# shellcheck disable=SC2154 # tests/nested-niri.sh sets XDG_RUNTIME_DIR and WAYLAND_DISPLAY.
# The fence's clipboard door and sandboxed display on a real niri, nested in
# the build sandbox (tests/nested-niri.sh runs this with WAYLAND_DISPLAY at
# niri). The display's socket offers ordinary clients what they need and
# none of the privileged protocols niri's own does; a command bubblewrap
# fences copies and pastes both selections through the door and its shims,
# a password manager's secret is refused, and a command outside the fence
# is refused before it asks; the door's journal names the session pi gave
# the bubblewrap, and never what was copied. A nested niri, as core's
# shell-preview starts one from a fenced command, opens on the sandboxed
# display, and wtype types into it but not into the display itself.
# usage: pi-fence-clipboard-on-niri.sh <door> <display> <shims> <logs>
set -euo pipefail

door=$1
display=$2
shims=$3
logs=$4

fail() {
  echo "FAIL: $*" >&2
  for log in "$logs"/door.log "$logs"/display.log "$logs"/inner.log; do
    [ -f "$log" ] && sed "s|^|$(basename "$log"): |" "$log" >&2
  done
  exit 1
}

# Waits for a socket, or fails naming it.
appears() {
  for _ in $(seq 100); do
    [ -S "$1" ] && return 0
    sleep 0.1
  done
  fail "$1 never appeared"
}

mkdir -m 700 "$XDG_RUNTIME_DIR/pi-fence" "$XDG_RUNTIME_DIR/pi-clipboard"
filtered=$XDG_RUNTIME_DIR/pi-fence/wayland
"$display" --socket "$filtered" 2> "$logs/display.log" &
appears "$filtered"

# What each display offers, a global a line.
globals() {
  WAYLAND_DISPLAY=$1 wayland-info | sed -n "s/.*interface: '\([^']*\)'.*/\1/p" | sort -u
}
globals "$WAYLAND_DISPLAY" > "$logs/raw.txt"
globals "$filtered" > "$logs/filtered.txt"
for privileged in \
  ext_data_control_manager_v1 \
  zwlr_data_control_manager_v1 \
  zwlr_screencopy_manager_v1 \
  zwp_virtual_keyboard_manager_v1 \
  zwlr_virtual_pointer_manager_v1 \
  zwlr_foreign_toplevel_manager_v1 \
  ext_foreign_toplevel_list_v1 \
  zwlr_layer_shell_v1 \
  ext_session_lock_manager_v1 \
  zwp_input_method_manager_v2 \
  ext_idle_notifier_v1 \
  wp_security_context_manager_v1; do
  grep -qx "$privileged" "$logs/raw.txt" || fail "niri's own display lacks $privileged, so its absence proves nothing"
  ! grep -qx "$privileged" "$logs/filtered.txt" || fail "the sandboxed display offers $privileged"
done
for ordinary in wl_compositor wl_shm xdg_wm_base wl_seat; do
  grep -qx "$ordinary" "$logs/filtered.txt" || fail "the sandboxed display lacks $ordinary, so no window opens on it"
done

export PI_CLIPBOARD_DOOR=$XDG_RUNTIME_DIR/pi-clipboard/door.sock
"$door" --socket "$PI_CLIPBOARD_DOOR" 2> "$logs/door.log" &
appears "$PI_CLIPBOARD_DOOR"

# Outside the fence, the door refuses before reading anything.
if outside=$("$shims/wl-paste" 2>&1); then
  fail "a command outside the fence was answered: $outside"
fi
case $outside in
  *"the clipboard door answers only commands in pi's fence"*) ;;
  *) fail "outside the fence it said: $outside" ;;
esac

line="fence-check-$RANDOM-$RANDOM"
# A fenced command, as pi runs one: bubblewrap with a pid namespace of its
# own, its session in bubblewrap's environment, and a claim of another
# session in its own.
fenced() {
  PI_SESSION_ID=check-session bwrap --unshare-pid --dev-bind / / --proc /proc -- \
    env PI_SESSION_ID=forged PATH="$shims:$PATH" bash -euo pipefail -c "$1"
}
fenced "
  echo '$line' | wl-copy -p -n
  [ \"\$(wl-paste -p -n)\" = '$line' ] || { echo 'the primary selection did not round-trip'; exit 1; }
  wl-copy two words
  [ \"\$(wl-paste)\" = 'two words' ] || { echo 'the clipboard did not round-trip'; exit 1; }
  [ \"\$(wl-paste -l | sed -n 1p)\" = 'text/plain;charset=utf-8' ] || { echo 'text is not listed first'; exit 1; }
  if wl-paste -t image/png 2> /dev/null; then echo 'a type not offered was pasted'; exit 1; fi
  if wl-paste --watch cat 2> /dev/null; then echo '--watch was taken'; exit 1; fi
  wl-copy -c
  if wl-paste 2> /dev/null; then echo 'a cleared clipboard pasted'; exit 1; fi
  printf hunter2 | wl-copy -t x-kde-passwordManagerHint
  if said=\$(wl-paste -t x-kde-passwordManagerHint 2>&1); then echo 'a secret was read'; exit 1; fi
  case \$said in *password*) ;; *) echo \"a secret's refusal said: \$said\"; exit 1;; esac
  printf once | wl-copy -o
  [ \"\$(wl-paste -n)\" = once ] || { echo 'a copy served once was not served'; exit 1; }
  # The door lets the copy go once it's served, and the compositor hears
  # so a moment later: up to two seconds, in tenths, for it to empty.
  for _ in \$(seq 20); do wl-paste 2> /dev/null || exit 0; sleep 0.1; done
  echo 'a copy served once was served twice'; exit 1
" || fail "a fenced command's copy and paste"

grep -q "^copied session=check-session selection=primary type=text/plain;charset=utf-8 bytes=${#line}\$" \
  "$logs/door.log" || fail "the door's journal doesn't name the bubblewrap's session"
grep -q "^refused session=check-session selection=clipboard why=secret\$" "$logs/door.log" ||
  fail "the door's journal doesn't say the secret was refused"
grep -q "^refused session=unknown selection=- why=unfenced peer=unfenced\$" "$logs/door.log" ||
  fail "the door's journal doesn't say the command outside was refused"
! grep -q "forged" "$logs/door.log" || fail "the door's journal names the session a command claimed"

# shell-preview's desktop, from a fenced command: a niri of its own on the
# sandboxed display, in a runtime folder of its own and with no bus, since
# the outer niri holds the bus's names.
inner=$(mktemp -d)
: > "$logs/inner.kdl"
cat > "$logs/preview.sh" << 'EOF'
set -euo pipefail
inner=$1 filtered=$2 logs=$3
env -u DBUS_SESSION_BUS_ADDRESS -u NIRI_SOCKET XDG_RUNTIME_DIR="$inner" \
  WAYLAND_DISPLAY="$filtered" niri -c "$logs/inner.kdl" > "$logs/inner.log" 2>&1 &
# niri drawing in software takes some seconds to open its socket: up to a
# minute, in tenths, as tests/nested-niri.sh gives the outer one.
for _ in $(seq 600); do
  [ -S "$inner/wayland-1" ] && break
  kill -0 $! 2> /dev/null || { echo 'the nested niri ended'; exit 1; }
  sleep 0.1
done
[ -S "$inner/wayland-1" ] || { echo 'the nested niri never opened its socket'; exit 1; }
XDG_RUNTIME_DIR="$inner" WAYLAND_DISPLAY=wayland-1 wtype typed ||
  { echo "wtype couldn't type into the nested niri"; exit 1; }
if WAYLAND_DISPLAY="$filtered" wtype typed 2> /dev/null; then
  echo 'wtype typed on the sandboxed display'; exit 1
fi
kill $!
EOF
fenced "bash $logs/preview.sh $inner $filtered $logs" ||
  fail "shell-preview's nested niri on the sandboxed display"
! grep -qe "$line" -e hunter2 -e "two words" "$logs/door.log" || fail "the door's journal holds what was copied"
echo "the fence's clipboard and display hold on niri"
