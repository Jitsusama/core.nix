# shellcheck shell=bash
# Locks the screen through Quickshell and returns once niri has drawn the
# lock on every screen, as Quickshell says by its marker file, or fails
# after three seconds, so a sleep held for it never shows the desktop.
#
# It says why it locks, as --reason gives it, to the journal of what
# asked, since the lock itself can't tell an idle lock from a lid's. The
# reason is a flag rather than the first word, so that --help or a typo is
# refused rather than taken for a reason: it once locked Joel's screen for
# `lock-screen --help` (2026-11-13). Its help and refusals take the shape
# every command of ours does (docs/commands.md), by hand.

reason=asked
while [ "$#" -gt 0 ]; do
  case $1 in
    -h | --help)
      cat << 'EOF'
lock-screen  locks the screen, once it's drawn on each one

EXAMPLES
  lock-screen                      lock it now
  lock-screen --reason "lid shut"  say why, to the journal

EXIT
  0   locked, and drawn on every screen
  1   the lock wasn't drawn within three seconds
  64  asked wrongly
EOF
      exit 0
      ;;
    --reason)
      if [ "$#" -lt 2 ]; then
        printf '%s\n' "lock-screen --reason needs the reason" "  try  lock-screen --help" >&2
        exit 64
      fi
      reason=$2
      shift 2
      ;;
    *)
      printf '%s\n' "lock-screen doesn't know $1" "  try  lock-screen --help" >&2
      exit 64
      ;;
  esac
done

echo "locking: $reason" >&2

marker=$XDG_RUNTIME_DIR/quickshell-locked
drawn() { [ "$(cat "$marker" 2> /dev/null)" = locked ]; }

# Watched before asking, so the lock can't be drawn unseen in between.
exec 3< <(exec inotifywait -m -e moved_to,close_write --format %f "$XDG_RUNTIME_DIR" 2>&1)
watcher=$!
trap 'kill "$watcher" 2> /dev/null || true' EXIT
while read -r line <&3 && [ "$line" != "Watches established." ]; do :; done

qs ipc call lock lock
deadline=$((SECONDS + 3))
until drawn; do
  left=$((deadline - SECONDS))
  if [ "$left" -le 0 ] || ! read -r -t "$left" _ <&3; then
    drawn && exit 0
    echo "the lock screen wasn't drawn in time" >&2
    exit 1
  fi
done
