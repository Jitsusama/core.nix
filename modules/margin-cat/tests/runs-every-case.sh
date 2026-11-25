# shellcheck shell=bash
# Runs every QML case in the folder it starts in, each for at most a minute
# of its own time on the processor, and names each that fails or runs past
# it, as it ends and again at the close, since the build's log otherwise
# says only the exit code of the first, not which case, nor any after it,
# and its tail may lose an early case's name under a later one's output.
# Each case says how long it took of its own and of the clock's, so one
# nearing its limit shows before it goes over.
# The limit is the case's own time rather than the clock's, since a
# builder's load stretches the clock and not the work: the journeys, which
# step through 150 journeys a millisecond at a time in about 25 s, ran past
# three minutes by the clock on a builder whose every case ran slow (22
# November, the games 51 s of their 60), with nothing wrong. A case
# waiting on nothing forever uses none of its own time, so the clock still
# ends one after ten minutes, and the run stops there, since what hangs one
# case most often hangs every case after it.
# usage: QML_IMPORT_PATH=<qml modules> runs-every-case.sh <qml>
# CASE_LIMIT and CASE_GUARD set the two limits in seconds, for
# runs-every-case-test.sh, which holds what this says of each way a case
# ends.
set -uo pipefail
shopt -s nullglob

qml=$1
limit=${CASE_LIMIT:-60}
guard=${CASE_GUARD:-600}
# The processor signals a case past its limit (SIGXCPU, exit 152), and kills
# it only past this, so a kill (137) is told apart: out of memory, or a case
# that went on past the signal.
hard=$((limit + 10))
# What the shell's `time` says of each case: its own seconds, then the clock's.
TIMEFORMAT='%U %S %R'
used=$(mktemp)
trap 'rm -f "$used"' EXIT
# A case run under its limits. It isn't exec'd, since bash's `time` says
# nothing of a subshell that execs. The soft limit is set first, as a hard
# one below the soft one in force is refused.
run() {
  ulimit -St "$limit"
  ulimit -Ht "$hard"
  timeout -k 10 "$guard" "$qml" "$1"
}
# What a case's exit code says of how it ended, given its seconds by the
# clock. A kill the guard sent after its own signal went unheeded is still
# the guard's, so the clock says which a kill was.
ended() {
  local case=$1 code=$2 clock=$3
  if [ "$code" = 124 ] || { [ "$code" = 137 ] && [ "$clock" -ge "$guard" ]; }; then
    echo "$case ran past the clock's $guard s, waiting on something"
  elif [ "$code" = 137 ]; then
    echo "$case was killed: out of memory, or past its hard $hard s"
  elif [ "$code" = 152 ]; then
    echo "$case ran past its $limit s of its own time"
  else
    echo "$case failed, exit code $code"
  fi
}
cases=(*.qml)
if [ "${#cases[@]}" = 0 ]; then
  echo "no cases to run in $PWD" >&2
  exit 1
fi
failed=()
unrun=()
for case in "${cases[@]}"; do
  if [ "${#failed[@]}" != 0 ] && [[ ${failed[-1]} == *"the clock's"* ]]; then
    unrun+=("$case")
    continue
  fi
  # The case's own output goes to the log, and only the timing to `used`,
  # since `time` on a subshell writes to the subshell's stderr as it stands
  # outside the case's own redirection.
  { time (run "$case" 2>&3); } 3>&2 2>"$used"
  code=$?
  # The timing is the last line: bash says a job a signal ended above it.
  read -r user kernel clock < <(tail -n 1 "$used")
  own=$(awk -v user="$user" -v kernel="$kernel" 'BEGIN { printf "%.0f", user + kernel }')
  if [ "$code" != 0 ]; then
    said=$(ended "$case" "$code" "${clock%.*}")
    echo "$said" >&2
    failed+=("$said")
  fi
  echo "$case took $own s of its $limit, ${clock%.*} s by the clock" >&2
done
if [ "${#unrun[@]}" != 0 ]; then
  echo "left unrun after it: ${unrun[*]}" >&2
fi
if [ "${#failed[@]}" != 0 ]; then
  printf '%s of %s cases failed:\n' "${#failed[@]}" "${#cases[@]}" >&2
  printf '  %s\n' "${failed[@]}" >&2
  if [ "${#unrun[@]}" != 0 ]; then
    printf '  %s left unrun\n' "${unrun[@]}" >&2
  fi
  exit 1
fi
