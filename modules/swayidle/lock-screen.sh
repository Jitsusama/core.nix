# shellcheck shell=bash
# Locks the screen through Quickshell and returns once niri has drawn the
# lock on every screen, as Quickshell says by its marker file, or fails
# after three seconds, so a sleep held for it never shows the desktop.

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
