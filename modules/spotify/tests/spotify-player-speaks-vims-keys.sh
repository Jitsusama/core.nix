# shellcheck shell=bash
# Checks spotify-player takes the key language's keys its own lack:
# Ctrl+e and Ctrl+y a line, Ctrl+o back a page. spotify-player reads its
# keymap before anything else and stops at one it can't, so the keys are
# read from a keymap it accepted.
# usage: spotify-player-speaks-vims-keys.sh <app.toml> <keymap.toml>
set -euo pipefail

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
export HOME=$work
mkdir "$work/config" "$work/cache"
cp "$1" "$work/config/app.toml"
cp "$2" "$work/config/keymap.toml"
spotify_player -c "$work/config" -C "$work/cache" features > /dev/null

for pair in C-e:SelectNextOrScrollDown C-y:SelectPreviousOrScrollUp C-o:PreviousPage; do
  key=${pair%%:*}
  command=${pair#*:}
  given=$(yq -p toml ".keymaps[] | select(.key_sequence == \"$key\") | .command" "$work/config/keymap.toml")
  if [[ "$given" != "$command" ]]; then
    echo "spotify-player's $key doesn't do $command, as the language says: '$given'" >&2
    exit 1
  fi
done
