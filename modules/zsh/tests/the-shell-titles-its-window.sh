# shellcheck shell=bash
# Starts zsh in a terminal with only terminal.zsh read, types a few commands
# ahead, and checks the window titles it gives: its folder at each prompt
# and the command while one runs, its first line alone, with no control
# character of the command's reaching the terminal.
# usage: the-shell-titles-its-window.sh <terminal.zsh>
set -euo pipefail

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
cd "$work"
export HOME=$work/home ZDOTDIR=$work/dot TERM=xterm-kitty
mkdir -p "$HOME/src/core.nix" "$ZDOTDIR"
cp "$1" "$ZDOTDIR/.zshrc"
cd "$HOME"

# Typed ahead, each line a prompt: a folder changed, the title given a
# command holding an escape and a bell, as a paste can bring, one of two
# lines, then out. Typed, they'd meet the terminal's own line editing
# before zsh's, so the hook is given them itself.
printf '%s\r' "cd src/core.nix" "_title_running \$'a\\e]2;hijacked\\ab'" $': one\\\ntwo' \
  exit > "$work/keys"
script -q -c "stty cols 90 rows 30; zsh -i" /dev/null < "$work/keys" > "$work/said"

# Each title said, one a line, its frame taken off; none is none.
titles=$({ grep -ao $'\e]2;[^\a]*\a' "$work/said" || true; } | sed $'s/^\e]2;//; s/\a$//')
want=$(
  cat << 'EOF'
~
cd src/core.nix
~/src/core.nix
_title_running $'a\e]2;hijacked\ab'
a ]2;hijacked b
~/src/core.nix
: one\
~/src/core.nix
exit
EOF
)
if [[ $titles != "$want" ]]; then
  printf 'the shell titled its window:\n%s\nwhere it should have:\n%s\n' "$titles" "$want" >&2
  exit 1
fi
