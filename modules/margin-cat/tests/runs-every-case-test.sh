# shellcheck shell=bash
# Holds what runs-every-case.sh says of each way a case can end, over
# stand-in cases a stand-in qml plays out, under a limit of a second of the
# processor's: a pass, a failure, a case past its own time and one killed
# from outside with a pass after it, under a guard of a minute by the clock, so a loaded builder
# slowing the spinning case never makes it a hang; then, under a guard of
# three seconds, a hang that stops the run and one that ignores the guard's
# signal until its kill, each with a case left unrun after it; and limits
# and guards that aren't whole seconds, refused.
# usage: runs-every-case-test.sh <runs-every-case.sh>
set -euo pipefail

runner=$1
here=$(mktemp -d)
trap 'rm -rf "$here"' EXIT

# The stand-in qml plays out what its case's one line says, in the shell
# running this, as a build has no /usr/bin/env.
echo "#!$BASH" >"$here/qml"
cat >>"$here/qml" <<'EOF'
read -r how <"$1"
case $how in
  pass) exit 0 ;;
  fail) exit 2 ;;
  spin) while :; do :; done ;;
  die) kill -KILL $$ ;;
  # A hang is a case that waits on nothing past the runner's wall limit, using no CPU.
  hang) exec sleep 60 ;;
  deaf)
    trap '' TERM
    # A deaf hang ignores the guard's signal and outlasts its kill, so only the kill ends it.
    exec sleep 120
    ;;
esac
EOF
chmod +x "$here/qml"

fails=0
failing() {
  echo "FAIL: $1" >&2
  fails=$((fails + 1))
}

# Runs the runner over a folder of its own, with a stand-in case for each
# `name:how` given, under `limit` seconds of the processor's and `guard` of
# the clock's, keeping what it said and how it ended.
runs() {
  local folder=$here/$1 limit=$2 guard=$3 each
  shift 3
  mkdir "$folder"
  for each in "$@"; do
    echo "${each#*:}" >"$folder/${each%%:*}.qml"
  done
  code=0
  said=$(cd "$folder" && CASE_LIMIT=$limit CASE_GUARD=$guard "$BASH" "$runner" "$here/qml" 2>&1) ||
    code=$?
  echo "$said"
}
ends() {
  [ "$code" = "$1" ] || failing "it ended $code, not $1"
}
holds() {
  grep -qxF -- "$1" <<<"$said" || failing "it never said: $1"
}
never() {
  if grep -qF -- "$1" <<<"$said"; then
    failing "it said: $1"
  fi
}
# That a case said its time, its own seconds and the clock's each matching
# a pattern.
took() {
  grep -qE "^$1\.qml took ($2) s of its 1, ($3) s by the clock$" <<<"$said" ||
    failing "$1.qml never said its time as $2 of its own and $3 by the clock"
}

runs ends 1 60 a-passes:pass b-fails:fail c-spins:spin d-dies:die e-after:pass
ends 1
holds "b-fails.qml failed, exit code 2"
holds "c-spins.qml ran past its 1 s of its own time"
holds "d-dies.qml was killed: out of memory, or past its hard 11 s"
holds "3 of 5 cases failed:"
never "a-passes.qml failed"
never "e-after.qml failed"
never "left unrun"
took a-passes 0 "[0-9]+"
took b-fails 0 "[0-9]+"
# The spinning case's own second is all the processor's, however slow the
# clock ran.
took c-spins "1|2" "[0-9]+"
# The killed one says its time too, though bash writes its job notice above
# its timing.
took d-dies 0 "[0-9]+"
# A kill from outside is no hang, so the run goes on past it.
took e-after 0 "[0-9]+"

runs hangs 1 3 e-hangs:hang f-follows:pass
ends 1
holds "e-hangs.qml ran past the clock's 3 s, waiting on something"
holds "left unrun after e-hangs.qml: f-follows.qml"
holds "1 of 2 cases failed:"
holds "  f-follows.qml left unrun"
never "f-follows.qml took"
took e-hangs 0 "[3-9]|[1-9][0-9]+"

# The guard kills it ten seconds after the signal it ignored, and a kill
# that late by the clock is still the guard's, so the run stops there too.
runs deaf 1 3 g-ignores:deaf h-follows:pass
ends 1
holds "g-ignores.qml ran past the clock's 3 s, waiting on something"
holds "left unrun after g-ignores.qml: h-follows.qml"
never "g-ignores.qml was killed"
# Its stand-in would end by itself at 120 s, so a run reaching that has
# lost the guard's kill.
took g-ignores 0 "1[3-9]|[2-5][0-9]"

runs refused soon 60 a-passes:pass
ends 2
holds "CASE_LIMIT and CASE_GUARD must be whole seconds, not 'soon' and '60'"
never "a-passes.qml took"

runs refused-guard 1 0 a-passes:pass
ends 2
holds "CASE_LIMIT and CASE_GUARD must be whole seconds, not '1' and '0'"
never "a-passes.qml took"

[ "$fails" = 0 ]
