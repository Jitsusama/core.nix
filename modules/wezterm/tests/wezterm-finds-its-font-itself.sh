# shellcheck shell=bash
# Checks WezTerm, with the account's files and no fonts of the machine's own,
# draws in the theme's monospaced face, so it finds the face in the files its
# configuration names rather than among the system's fonts, which a Mac's
# switch swaps while WezTerm reloads.
# usage: wezterm-finds-its-font-itself.sh <home files> <wezterm> <the theme's monospaced family>
set -euo pipefail

files=$1
wezterm=$2
family=$3
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
export HOME=$work/home XDG_CACHE_HOME=$work/home/.cache XDG_RUNTIME_DIR=$work
# Where WezTerm looks, so its configuration finds theme.lua beside it.
mkdir -p "$HOME/.config"
ln -s "$files/.config/wezterm" "$HOME/.config/wezterm"

status=0
"$wezterm" ls-fonts --text a > "$work/drawn" 2>&1 || status=$?
cat "$work/drawn"
if [ "$status" -ne 0 ] || grep -q 'runtime error' "$work/drawn"; then
  echo "WezTerm couldn't read its configuration." >&2
  exit 1
fi
if ! grep -q -F "wezterm.font(\"$family\"" "$work/drawn"; then
  echo "WezTerm drew in another face than the theme's, so it can't find its own." >&2
  exit 1
fi
