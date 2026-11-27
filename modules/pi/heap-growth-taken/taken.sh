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
# loaded builder or a clock step would make lie. Its words are read in C,
# so no locale translates them.
heard=$(mktemp)
LC_ALL=C timeout --verbose --kill-after=5 "$seconds" "$node" "$word" -e 0 2>"$heard"
said=$?
# What the node said goes on to the log, as each message says it's above.
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
