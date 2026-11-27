# shellcheck shell=bash
# Runs a realm's node with the growing percent its conversations are given,
# as default.nix's build does, and fails saying why when the node can't be
# run, refuses the word, fails another way, doesn't end within the seconds
# it's given, or is killed before then. It takes the node, the word, the
# seconds, what to say for each of those five, and the file a pass makes.
set -u
node=$1 word=$2 seconds=$3 unrun=$4 refusal=$5 failed=$6 slow=$7 killed=$8 out=$9
if ! [ -x "$node" ] || [ -d "$node" ]; then
  echo "$unrun" >&2
  exit 1
fi
# node exits 9 on an option it doesn't know, as on any argument it refuses,
# which conversation-checks.nix proves of node itself. timeout exits 124
# when the node outlives its seconds, and 137 when it then outlives the
# five more that follow its kill. A node can exit 124 itself, though, and
# 137 is also a node the kernel killed for memory or something outside
# killed, so either reads as slow only where timeout says it sent its
# signal. That rests on timeout's word rather than the clock, which a
# loaded builder or a clock step would make lie. Only timeout's words are
# read, in C so no locale translates them: a shell execs the node with the
# locale it was given and the build's own errors, kept on descriptor 3, so
# the node keeps the process timeout signals, runs as its unit would, says
# what it says to the log as it says it, and can't pass for timeout by
# saying timeout's words. timeout's words name that shell, not the node;
# the message after them names the node.
heard=$(mktemp)
# The locale the node is given back: whether it was set, then its value.
locale=("" "")
if [ -n "${LC_ALL+set}" ]; then
  locale=("set" "$LC_ALL")
fi
# shellcheck disable=SC2016 # The inner shell expands these, not this one.
given='if [ -n "$0" ]; then LC_ALL=$1; else unset LC_ALL; fi
shift
exec "$@" 2>&3 3>&-'
LC_ALL=C timeout --verbose --kill-after=5 "$seconds" \
  "$BASH" -c "$given" "${locale[@]}" "$node" "$word" -e 0 3>&2 2>"$heard"
said=$?
# What timeout said follows the node's on the log, as each message says
# it's above.
cat "$heard" >&2
fired=0
if grep -q '^timeout: sending signal ' "$heard"; then
  fired=1
fi
case $said:$fired in
  0:*) touch "$out" ;;
  9:*)
    echo "$refusal" >&2
    exit 1
    ;;
  124:1 | 137:1)
    echo "$slow" >&2
    exit 1
    ;;
  137:0)
    echo "$killed" >&2
    exit 1
    ;;
  *)
    echo "$failed" >&2
    exit 1
    ;;
esac
