# shellcheck shell=bash
# Starts the account's zsh in a terminal under each open-files limit a
# session could hand it, and checks the limit its commands get: a soft
# limit under 8192 rises to 8192, or to the hard limit where that's lower;
# a soft limit already higher stays; and the hard limit never moves.
# usage: the-shell-opens-enough-files.sh <home files>
set -euo pipefail

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
cd "$work"
export HOME=$work/home TERM=xterm-kitty
mkdir -p "$HOME"
ln -s "$1/.zshrc" "$1/.zshenv" "$HOME/"

# The shell started under a soft and a hard limit, as "soft hard" the way
# its commands see them. The soft limit goes first, since a hard limit
# under the soft one is refused.
given() {
  (
    ulimit -Sn "$1"
    ulimit -Hn "$2"
    script -q -c "zsh -i -c 'print \$(ulimit -Sn) \$(ulimit -Hn) > $work/seen'" \
      /dev/null < /dev/null > /dev/null
  )
  cat "$work/seen"
}

hard=$(ulimit -Hn)
if [ "$hard" != unlimited ] && [ "$hard" -lt 10000 ]; then
  echo "The sandbox's hard limit, $hard, is too low to test under" >&2
  exit 1
fi

failed=0
expect() {
  local seen
  seen=$(given "$1" "$2")
  if [ "$seen" != "$3" ]; then
    echo "Given soft $1 and hard $2, the shell's commands got $seen, not $3" >&2
    failed=1
  fi
}
# A desktop session's default, as systemd gives each user's programs.
expect 1024 10000 "8192 10000"
# A hard limit under 8192 is as far as it goes.
expect 1024 4096 "4096 4096"
# A session that gave more keeps it.
expect 9000 10000 "9000 10000"
exit "$failed"
