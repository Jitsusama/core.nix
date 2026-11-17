# shellcheck shell=bash
# Starts the account's zsh in a terminal under each open-files limit a
# session could hand it, and checks the limit its commands get, and the
# prompt's git-engine with them: a soft limit under 8192 rises to 8192, or
# to the hard limit where that's lower; a soft limit already higher stays;
# and the hard limit never moves. A limit of unlimited, which only a Mac's
# session gives, is checked with limits.zsh read under a stand-in ulimit.
# usage: the-shell-opens-enough-files.sh <home files> <limits.zsh>
set -euo pipefail

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
cd "$work"
export HOME=$work/home TERM=xterm-kitty SEEN=$work/seen
mkdir -p "$HOME"
ln -s "$1/.zshrc" "$1/.zshenv" "$HOME/"

# Read in the started shell: its own limits, and the soft limit of the
# git-engine its prompt started, found among the sandbox's few processes.
cat > ask.zsh <<'EOF'
local engine= name= limits=
local -a names
for _ in {1..100}; do
  names=()
  for process in /proc/<->(N); do
    { name=$(<$process/comm) } 2>/dev/null
    names+=($name)
    [[ $name == git-engine ]] && engine=$process
  done
  [[ -n $engine ]] && break
  sleep 0.05
done
if [[ -n $engine ]]; then
  limits=${(M)${(f)"$(<$engine/limits)"}:#Max open files*}
else
  print -u2 "No git-engine was running, only: $names"
fi
print $(ulimit -Sn) $(ulimit -Hn) ${${=limits}[4]:-none} > $SEEN
EOF

# Typed at the shell's first prompt, since git-engine starts with it.
printf '%s\r' ". $work/ask.zsh" exit > keys

# The shell started under a soft and a hard limit, as "soft hard engine".
# The soft limit goes first, since a hard limit under the soft one is
# refused. Anything the shell says on its error stream fails the case;
# its prompt and what's typed go to the terminal instead.
given() {
  rm -f "$SEEN" errors
  (
    ulimit -Sn "$1"
    ulimit -Hn "$2"
    script -q -c "stty cols 90 rows 30; zsh -i 2> $work/errors" /dev/null \
      < keys > /dev/null
  )
  if [ -s errors ]; then
    echo "Given soft $1 and hard $2, the shell said:" >&2
    cat errors >&2
  fi
  cat "$SEEN" 2> /dev/null || echo "nothing"
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
  if [ "$seen" != "$3" ] || [ -s errors ]; then
    echo "Given soft $1 and hard $2, the shell's commands got $seen, not $3" >&2
    failed=1
  fi
}
# A desktop session's default, as systemd gives each user's programs.
expect 1024 10000 "8192 10000 8192"
# A hard limit under 8192 is as far as it goes.
expect 1024 4096 "4096 4096 4096"
# A session that gave more keeps it.
expect 9000 10000 "9000 10000 9000"

# limits.zsh read under a stand-in ulimit that answers with the limits
# given and writes down any it's asked to set, as "set <soft>".
unlimited() {
  rm -f "$SEEN"
  S=$1 H=$2 zsh -f -c '
    ulimit() {
      case $1 in
        (-Sn) if (( $# == 2 )); then print set $2 > $SEEN; else print $S; fi ;;
        (-Hn) print $H ;;
      esac
    }
    . "$0"' "$3"
  cat "$SEEN" 2> /dev/null || echo "nothing set"
}
expectUnlimited() {
  local seen
  seen=$(unlimited "$1" "$2" "$limits")
  if [ "$seen" != "$3" ]; then
    echo "Given soft $1 and hard $2, limits.zsh did: $seen, not $3" >&2
    failed=1
  fi
}
limits=$2
# A Mac's session gives each program 256 under an unlimited hard limit.
expectUnlimited 256 unlimited "set 8192"
expectUnlimited unlimited unlimited "nothing set"
exit "$failed"
