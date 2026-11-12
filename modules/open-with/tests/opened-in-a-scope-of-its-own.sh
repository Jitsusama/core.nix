# shellcheck shell=bash
# Opens a picture through gio twice, with GIO_LAUNCH_DESKTOP naming the
# launched wrapper and a stand-in systemd-run that says what it was asked
# and runs the program: once with the account's systemd reachable, where the
# viewer must start in a scope of its own named for its entry, and once
# without, where it must start as it is. Either way the viewer must be the
# process the entry says it started.
# usage: opened-in-a-scope-of-its-own.sh <shared-mime-info> <python>
set -euo pipefail

mime=$1
python=$2
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
export HOME=$work/home XDG_CONFIG_HOME=$work/config XDG_CURRENT_DESKTOP=niri
export XDG_DATA_DIRS=$work/share:$mime/share XDG_RUNTIME_DIR=$work/runtime
export LAUNCHED_LOG=$work/asked
mkdir -p "$HOME" "$XDG_CONFIG_HOME" "$work/share/applications" "$work/bin" \
  "$XDG_RUNTIME_DIR/systemd"
export PATH=$work/bin:$PATH
: > "$LAUNCHED_LOG"
opened=$work/opened

# The viewer says its pid, the pid the entry says it started, and its file.
{
  echo "#!$BASH"
  cat << 'viewer'
echo "viewer $$ ${GIO_LAUNCHED_DESKTOP_FILE_PID:-none} $*" >> "$OPENED"
viewer
} > "$work/bin/viewer"
export OPENED=$opened
chmod +x "$work/bin/viewer"
printf '[Desktop Entry]\nType=Application\nName=Viewer\nExec=viewer %%f\nMimeType=image/png;\n' \
  > "$work/share/applications/viewer.desktop"
printf '[Default Applications]\nimage/png=viewer.desktop\n' > "$XDG_CONFIG_HOME/niri-mimeapps.list"

cd "$work"
# A picture of one pixel.
base64 -d <<< iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg== \
  > shot.png

# How many lines file $1 holds.
lines() {
  local count
  count=$(wc -l < "$1")
  echo "$count"
}

# Opens the picture and waits for the viewer's line number $1, as the
# viewer may answer after gio has returned, leaving it in `said`.
open_picture() {
  local count=0
  gio open shot.png
  for _ in {1..50}; do
    [ -f "$opened" ] && count=$(lines "$opened")
    [ "$count" -ge "$1" ] && break
    sleep 0.1
  done
  said=$(sed -n "$1p" "$opened")
}

"$python" -c 'import socket, sys; socket.socket(socket.AF_UNIX).bind(sys.argv[1])' \
  "$XDG_RUNTIME_DIR/systemd/private"
open_picture 1
read -r _ pid entry_pid file <<< "$said"
cat "$LAUNCHED_LOG"
[ "$pid" = "$entry_pid" ] || { echo "the entry names $entry_pid, not $pid" >&2; exit 1; }
[ "$file" = "$work/shot.png" ] || { echo "the viewer was given $file" >&2; exit 1; }
grep -qx "systemd-run --user --scope --collect --quiet --slice=app.slice \
--unit=app-gio-viewer-$pid -- $work/bin/viewer $work/shot.png" "$LAUNCHED_LOG" \
  || { echo "the viewer wasn't given a scope of its own" >&2; exit 1; }

rm "$XDG_RUNTIME_DIR/systemd/private"
open_picture 2
read -r _ pid entry_pid _ <<< "$said"
[ "$pid" = "$entry_pid" ] || { echo "unscoped, the entry names $entry_pid, not $pid" >&2; exit 1; }
asked=$(lines "$LAUNCHED_LOG")
[ "$asked" -eq 1 ] \
  || { echo "systemd-run was asked with no systemd to reach" >&2; exit 1; }
