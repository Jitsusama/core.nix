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
# ends one after ten minutes.
# usage: QML_IMPORT_PATH=<qml modules> runs-every-case.sh <qml>
set -uo pipefail
shopt -s nullglob

qml=$1
limit=60
guard=600
# What the shell's `time` says of each case: its own seconds, then the clock's.
TIMEFORMAT='%U %S %R'
used=$(mktemp)
# A case run under its limit, the processor's signal ending it past it.
# It isn't exec'd, since bash's `time` says nothing of a subshell that
# execs.
run() {
  ulimit -t "$limit"
  timeout "$guard" "$qml" "$1"
}
cases=(*.qml)
if [ "${#cases[@]}" = 0 ]; then
  echo "no cases to run in $PWD" >&2
  exit 1
fi
failed=()
for case in "${cases[@]}"; do
  # The case's own output goes to the log, and only the timing to `used`,
  # since `time` on a subshell writes to the subshell's stderr as it stands
  # outside the case's own redirection.
  { time (run "$case" 2>&3); } 3>&2 2>"$used"
  code=$?
  read -r user kernel clock <"$used"
  own=$(awk -v user="$user" -v kernel="$kernel" 'BEGIN { printf "%.0f", user + kernel }')
  if [ "$code" != 0 ]; then
    case $code in
      124) said="$case ran past the clock's $guard s, waiting on something" ;;
      137 | 152) said="$case ran past its $limit s" ;;
      *) said="$case failed, exit code $code" ;;
    esac
    echo "$said" >&2
    failed+=("$said")
  fi
  echo "$case took $own s of its $limit, ${clock%.*} s by the clock" >&2
done
if [ "${#failed[@]}" != 0 ]; then
  printf '%s of %s cases failed:\n' "${#failed[@]}" "${#cases[@]}" >&2
  printf '  %s\n' "${failed[@]}" >&2
  exit 1
fi
