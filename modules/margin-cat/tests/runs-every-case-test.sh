# shellcheck shell=bash
# Holds what runs-every-case.sh says of each way a case can end, over
# stand-in cases a stand-in qml plays out, under limits of a second of the
# processor's and three of the clock's: a pass, a failure, a case past its
# own time, one killed from outside, a hang that stops the run, and one
# left unrun after it.
# usage: runs-every-case-test.sh <runs-every-case.sh>
set -euo pipefail

runner=$1
here=$(mktemp -d)
trap 'rm -rf "$here"' EXIT
cd "$here"

# The stand-in qml plays out what its case's one line says, in the shell
# running this, as a build has no /usr/bin/env.
echo "#!$BASH" >qml
cat >>qml <<'EOF'
read -r how <"$1"
case $how in
  pass) exit 0 ;;
  fail) exit 2 ;;
  spin) while :; do :; done ;;
  die) kill -KILL $$ ;;
  # A hang is a case that waits on nothing past the runner's wall limit, using no CPU.
  hang) exec sleep 60 ;;
esac
EOF
chmod +x qml
echo pass >a-passes.qml
echo fail >b-fails.qml
echo spin >c-spins.qml
echo die >d-dies.qml
echo hang >e-hangs.qml
echo pass >f-follows.qml

code=0
said=$(CASE_LIMIT=1 CASE_GUARD=3 "$BASH" "$runner" ./qml 2>&1) || code=$?
echo "$said"

fails=0
holds() {
  if ! grep -qxF -- "$1" <<<"$said"; then
    echo "FAIL: it never said: $1" >&2
    fails=$((fails + 1))
  fi
}
[ "$code" = 1 ] || {
  echo "FAIL: it ended $code, not 1" >&2
  fails=$((fails + 1))
}
holds "b-fails.qml failed, exit code 2"
holds "c-spins.qml ran past its 1 s of its own time"
holds "d-dies.qml was killed: out of memory, or past its hard 11 s"
holds "e-hangs.qml ran past the clock's 3 s, waiting on something"
holds "left unrun after it: f-follows.qml"
holds "4 of 6 cases failed:"
holds "  f-follows.qml left unrun"
# Every case run says its time in whole seconds, the killed one's too,
# whose job notice bash writes above its timing.
for case in a-passes b-fails c-spins d-dies e-hangs; do
  if ! grep -qE "^$case\.qml took [0-9]+ s of its 1, [0-9]+ s by the clock$" <<<"$said"; then
    echo "FAIL: $case.qml never said its time" >&2
    fails=$((fails + 1))
  fi
done
if grep -q "a-passes.qml failed\|f-follows.qml took" <<<"$said"; then
  echo "FAIL: a pass was failed, or a case ran after the hang" >&2
  fails=$((fails + 1))
fi
[ "$fails" = 0 ]
