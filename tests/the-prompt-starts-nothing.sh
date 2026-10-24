# shellcheck shell=bash
# Starts zsh with only the account's prompt, under strace, and types ahead a
# run of prompts in and out of a repository that keeps its refs in files and
# one that keeps them in reftable. It checks the shell ran only git-engine
# and the mkfifo it talks through, and, between the two marks, forked and
# executed nothing while its reads of HEAD and tables.list show the prompts
# ran.
# usage: the-prompt-starts-nothing.sh <prompt file>
set -euo pipefail

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
cd "$work"
export HOME=$work/home TERM=xterm-256color
mkdir -p "$HOME/zsh" repo/a/b
echo "source $1" > "$HOME/zsh/.zshrc"
git -C repo init -q -b main
git init -q --ref-format=reftable -b main table
touch repo/a/b/file

# Typed ahead, each line a prompt; the two reads mark where to count.
printf '%s\r' 'cd repo' ': < /dev/null' 'cd a/b' true false 'cd ../../../table' true \
  'cd ..' ': < /dev/zero' exit > keys
ZDOTDIR=$HOME/zsh script -q -c \
  "stty cols 90 rows 30; strace -f -qq -o trace -e trace=process,openat zsh -i" \
  /dev/null < keys > /dev/null
shell=$(head -1 trace | cut -d' ' -f1)
# The shell's own lines between the two marks.
awk -v shell="$shell" '
  $1 == shell && /\/dev\/null/ { counting = 1 }
  $1 == shell && /\/dev\/zero/ { counting = 0 }
  counting && $1 == shell
' trace > prompts

# Each program the shell ran, by name; a PATH lookup's misses fail.
ran=$(grep 'execve("' trace | grep -v '= -1 ' | sed 's|.*execve("\([^"]*\)".*|\1|; s|.*/||' \
  | sort -u | tr '\n' ' ')
fail() {
  echo "$1" >&2
  exit 1
}
[ "$ran" = "git-engine mkfifo zsh " ] || fail "The shell ran: $ran"
grep -q HEAD prompts || fail "No prompt read the repository."
grep -q tables.list prompts || fail "No prompt read the reftable."
if grep -E 'clone|fork|execve' prompts; then
  fail "A prompt started a process."
fi
