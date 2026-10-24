# shellcheck shell=bash
# Checks bat, with the account's files, colours code in the account's own
# theme without a word of warning, and that delta, reading bat's themes,
# finds it too, so the theme reaches both from a cache nothing rebuilds.
# usage: bat-reads-its-theme.sh <home files> <bat> <delta> <keyword colour, as #rrggbb>
set -euo pipefail

files=$1
bat=$2
delta=$3
colour=$4
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
export HOME=$work/home XDG_CONFIG_HOME=$work/home/.config XDG_CACHE_HOME=$work/home/.cache
# As kitty says, so bat writes its colours whole rather than the nearest of 256.
export COLORTERM=truecolor
mkdir -p "$XDG_CONFIG_HOME"
ln -s "$files/.config/bat" "$XDG_CONFIG_HOME/bat"
ln -s "$files/.cache" "$XDG_CACHE_HOME"

# A keyword as bat writes it in the colour given: an escape naming its red,
# green and blue.
printf -v keyword '38;2;%d;%d;%dm' "0x${colour:1:2}" "0x${colour:3:2}" "0x${colour:5:2}"

printf 'fn main() {}\n' > "$work/a.rs"
"$bat" --color=always --paging=never "$work/a.rs" > "$work/coloured" 2> "$work/said"
if [ -s "$work/said" ]; then
  echo "bat said: $(cat "$work/said")" >&2
  exit 1
fi
if ! grep -qF "$keyword" "$work/coloured"; then
  echo "bat didn't colour the keyword $colour, so it isn't in the account's theme." >&2
  exit 1
fi

printf '%s\n' '--- a/a.rs' '+++ b/a.rs' '@@ -1 +1 @@' '-fn main() {}' '+fn main() { }' > "$work/change"
"$delta" --syntax-theme=Jitsusama --no-gitconfig < "$work/change" > /dev/null 2> "$work/said"
if [ -s "$work/said" ]; then
  echo "delta said: $(cat "$work/said")" >&2
  exit 1
fi
