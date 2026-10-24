# shellcheck shell=bash
# Runs the switch's own step for git's global settings, the one named
# declared-last, with the account's home moved into a folder of its own,
# then writes as programs do and switches again. What's declared has to win
# over what a program wrote, a program's own include has to be found once,
# ahead of the declared settings, and a switch with nothing to do has to
# change nothing. modules/git/checks.nix says why.
# usage: git-settings-stay-writable.sh <home-files> <activate> <home> <declared user.name>
set -euo pipefail

files=$1
activate=$2
account_home=$3
declared=$4
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
cd "$work"
export HOME=$work/home GIT_CONFIG_NOSYSTEM=1
program=$HOME/.config/program/gitconfig

# Says, and fails, when git's setting isn't what it should be by now.
expect() {
  local actual
  actual=$(git config --get "$1" || true)
  if [ "$actual" != "$2" ]; then
    echo "$3: $1 is '$actual', not '$2'"
    exit 1
  fi
}

# The includes ~/.gitconfig names itself, in order, on one line.
includes() {
  git config --global --no-includes --get-all include.path | tr '\n' ' '
}

# How often ~/.gitconfig includes the program's file, counted as dev counts
# before it adds its include.
programs() {
  git config --global --no-includes --get-all include.path |
    sed "s|^~/|$HOME/|" | grep -cxF "$program" || true
}

# The switch's step, run against the moved home.
switch() {
  sh -e step
}

for linked in .gitconfig .config/git/config; do
  if [ -e "$files/$linked" ]; then
    # shellcheck disable=SC2088 # The home a person knows, said, not a path to expand.
    echo "~/$linked is a link to the store"
    exit 1
  fi
done
mkdir -p home/.config/git home/.config/program
cp "$files/.config/git/declared.config" home/.config/git/
while IFS= read -r line; do
  if [[ $line == *declared-last* ]]; then
    line=${line#run }
    printf '%s\n' "${line//"$account_home"/"$HOME"}"
  fi
done < "$activate" > step

printf '[user]\n\tname = program\n[program]\n\tsetting = 1\n' > "$program"
switch
if [ "$(includes)" != "$program $HOME/.config/git/declared.config " ]; then
  echo "after a switch: ~/.gitconfig includes $(includes)"
  exit 1
fi
expect user.name "$declared" "after a switch"
expect program.setting 1 "after a switch"

git config --global --fixed-value --unset-all include.path "$program"
# shellcheck disable=SC2088 # dev writes the tilde itself, for git to expand.
git config --global --add include.path "~/.config/program/gitconfig"
switch
if [ "$(programs)" != 1 ]; then
  echo "after a switch: the program's include is in ~/.gitconfig $(programs) times"
  exit 1
fi

printf '[user]\n\tname = included\n' > home/tool.config
git config --global --add include.path "$HOME/tool.config"
git config --global tool.setting 1
git config --global user.name tool
expect tool.setting 1 "after a program wrote it"
switch
expect user.name "$declared" "after the next switch"
expect tool.setting 1 "after the next switch"

cp home/.gitconfig before
switch
if ! cmp -s before home/.gitconfig; then
  echo "a switch with nothing to do changed ~/.gitconfig"
  exit 1
fi
