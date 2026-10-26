# shellcheck shell=bash
# Starts the account's zsh in a terminal, with a run of prompts typed ahead,
# under strace, and checks that starting it runs only direnv, git-engine and
# the mkfifo git-engine talks through, and that its prompts run only
# direnv. The first start makes the completion dump the second reads.
# usage: the-shell-starts-nothing.sh <home files>
set -euo pipefail

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
cd "$work"
export HOME=$work/home TERM=xterm-kitty
mkdir -p "$HOME"
ln -s "$1/.zshrc" "$1/.zshenv" "$HOME/"

# Typed ahead, each line a prompt; the two reads mark where to count.
touch begin end
printf '%s\r' ": < $work/begin" true true true ": < $work/end" exit > keys

# Starts zsh in a terminal, behind the command given, reading what's typed.
start() {
  script -q -c "stty cols 90 rows 30; $1 zsh -i" /dev/null < keys > /dev/null
}
# The names of the programs a trace's lines ran, one space after each; a
# PATH lookup's misses fail, so they're left out.
names() {
  grep 'execve("' | grep -v '= -1 ' | sed 's|.*execve("\([^"]*\)".*|\1|; s|.*/||' \
    | sort -u | tr '\n' ' '
}

start ""
start "strace -f -qq -o trace -e trace=process,openat"
shell=$(head -1 trace | cut -d' ' -f1)
# The line where the shell itself first opened a file.
mark() {
  grep -n "^$shell .*$1" trace | head -1 | cut -d: -f1
}
started=$(head -n "$(mark "$work/begin")" trace | names)
prompts=$(sed -n "$(mark "$work/begin"),$(mark "$work/end")p" trace | names)
if [ "$started" != "direnv git-engine mkfifo zsh " ]; then
  echo "Starting the shell ran: $started" >&2
  exit 1
fi
if [ "$prompts" != "direnv " ]; then
  echo "Its prompts ran: $prompts" >&2
  exit 1
fi
