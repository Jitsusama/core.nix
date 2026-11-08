# shellcheck shell=bash
# Runs xdg-open on a markdown file and a picture, with stand-ins for an
# editor that runs in a terminal, a picture viewer and the terminal itself,
# each saying what it was given, and checks each kind was opened in its
# declared app: markdown in the editor declared for text, in the terminal,
# and the picture in its viewer. gio and xdg-terminal-exec are the real ones.
# usage: xdg-open-opens-each-kind.sh <shared-mime-info>
set -euo pipefail

mime=$1
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
export HOME=$work/home XDG_CONFIG_HOME=$work/config XDG_CURRENT_DESKTOP=niri
export XDG_DATA_DIRS=$work/share:$mime/share
mkdir -p "$HOME" "$XDG_CONFIG_HOME" "$work/share/applications" "$work/bin"
export PATH=$work/bin:$PATH
opened=$work/opened

for app in editor viewer terminal; do
  printf '#!%s\necho "%s $*" >> %s\n' "$BASH" "$app" "$opened" > "$work/bin/$app"
  chmod +x "$work/bin/$app"
done

# Writes a desktop entry named for its app, with the lines given.
entry() {
  printf '[Desktop Entry]\nType=Application\nName=%s\n%s\n' "$1" "$2" \
    > "$work/share/applications/$1.desktop"
}
entry editor $'Exec=editor %F\nTerminal=true\nMimeType=text/plain;'
entry viewer $'Exec=viewer %f\nMimeType=image/png;'
entry terminal $'Exec=terminal\nCategories=System;TerminalEmulator;\nX-TerminalArgExec=--'
printf '[Default Applications]\ntext/plain=editor.desktop\nimage/png=viewer.desktop\n' \
  > "$XDG_CONFIG_HOME/niri-mimeapps.list"
echo terminal.desktop > "$XDG_CONFIG_HOME/xdg-terminals.list"

cd "$work"
echo '# Notes' > notes.md
# A picture of one pixel.
base64 -d <<< iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg== \
  > shot.png
xdg-open notes.md
xdg-open shot.png
# The terminal's stand-in may answer after xdg-open has returned.
for _ in $(seq 50); do
  [ -f "$opened" ] && [ "$(wc -l < "$opened")" -ge 2 ] && break
  sleep 0.1
done
cat "$opened"
grep -qx "terminal -- editor $work/notes.md" "$opened"
grep -qx "viewer $work/shot.png" "$opened"
