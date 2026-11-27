# shellcheck shell=bash
# Runs a realm's node with the growing percent its conversations are given,
# as default.nix's build does, and fails saying why when the node can't be
# run, refuses the word, or fails another way. It takes the node, the word,
# what to say for each of those three, and the file a pass makes.
set -u
node=$1 word=$2 unrun=$3 refusal=$4 failed=$5 out=$6
if ! [ -x "$node" ]; then
  echo "$unrun" >&2
  exit 1
fi
# node exits 9 on an option it doesn't know, as on any argument it refuses.
"$node" "$word" -e 0
said=$?
case $said in
  0) touch "$out" ;;
  9)
    echo "$refusal" >&2
    exit 1
    ;;
  *)
    echo "$failed" >&2
    exit 1
    ;;
esac
