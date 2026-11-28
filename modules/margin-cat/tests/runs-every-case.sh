# shellcheck shell=bash
# Runs every QML case in the folder it starts in, each for at most two
# minutes of its own time on the processor, and names each that fails or runs past
# it, as it ends and again at the close, since the build's log otherwise
# says only the exit code of the first, not which case, nor any after it,
# and its tail may lose an early case's name under a later one's output.
# Each case says how long it took of its own and of the clock's, so one
# nearing its limit shows before it goes over.
# The limit is the case's own time rather than the clock's, since a
# builder's load stretches the clock and not the work: the journeys, which
# step through 150 journeys a millisecond at a time in about 25 s, ran past
# three minutes by the clock on a builder whose every case ran slow (22
# November, the games 51 s of their 60), with nothing wrong. A busy
# builder's shared cores stretch a case's own time too, if less: the
# journeys took from 24 s to past 60 s of it (26 November), so the limit
# is twice the slowest seen whole, still ending a case that spins. A case
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
limit=${CASE_LIMIT:-120}
guard=${CASE_GUARD:-600}
# A limit that isn't whole seconds is refused once, for the whole run,
# rather than by ulimit as each case is run.
if ! [[ $limit =~ ^[1-9][0-9]*$ && $guard =~ ^[1-9][0-9]*$ ]]; then
  echo "CASE_LIMIT and CASE_GUARD must be whole seconds, not '$limit' and '$guard'" >&2
  exit 2
fi
# The processor signals a case past its limit (SIGXCPU, exit 152), and kills
# it only past this, so a kill (137) is told apart: out of memory, or a case
# that went on past the signal.
hard=$((limit + 10))
# What the shell's `time` says of each case: its own seconds, then the clock's.
TIMEFORMAT='%U %S %R'
# Without its two scratch files the runner can't say a case's time or the
# builder's reason, so it stops here rather than blaming the builder below.
# They're let go as it ends from before either is made, so the first is
# let go too when only the second can't be made.
used=""
why=""
trap 'rm -f -- ${used:+"$used"} ${why:+"$why"}' EXIT
if ! used=$(mktemp) || ! why=$(mktemp); then
  echo "the runner couldn't make its scratch files" >&2
  exit 1
fi
# A builder that holds the run below the hard limit a case needs refuses it
# for every case alike, since each inherits the runner's, so it's said once,
# before any case, with the figure, what would fit and ulimit's own reason,
# rather than as each case fails unrun.
if ! (ulimit -St "$limit" && ulimit -Ht "$hard") 2>"$why"; then
  held=$(ulimit -Ht)
  fits="no CASE_LIMIT fits"
  if [[ $held =~ ^[0-9]+$ ]] && [ "$held" -gt 10 ]; then
    fits="CASE_LIMIT must be at most $((held - 10))"
  fi
  reason=$(sed -n '1{s/^.*: ulimit: //;p}' "$why")
  echo "the builder holds each case to $held s of its own time, short of the $hard s" \
    "each needs, so $fits; ulimit said: $reason" >&2
  echo "left every case unrun" >&2
  exit 1
fi
# A case run under its limits. It isn't exec'd, since bash's `time` says
# nothing of a subshell that execs. The soft limit is set first, as a hard
# one below the soft one in force is refused. A case its limit signals leaves
# no core file in the build's folder, though a host that pipes cores to a
# collector is still handed one.
run() {
  ulimit -c 0
  # The limits were shown to fit before any case, so a refusal here is
  # past foreseeing; the case still fails unrun rather than running with
  # none. It ends by 125, the code timeout gives for a failure of its own,
  # so it reads as the runner's and not the case's.
  ulimit -St "$limit" && ulimit -Ht "$hard" || return 125
  timeout -k 10 "$guard" "$qml" "$1"
}
# Whether the guard ended a case, given its exit code and its seconds by the
# clock: by its own signal (124), or by the kill it sent after that went
# unheeded, which the clock tells from any other kill.
hung() {
  [ "$1" = 124 ] || { [ "$1" = 137 ] && [ -n "$2" ] && [ "$2" -ge "$guard" ]; }
}
# What a case's exit code says of how it ended, given its seconds by the
# clock, or none where they couldn't be read.
ended() {
  local case=$1 code=$2 clock=$3
  if hung "$code" "$clock"; then
    echo "$case ran past the clock's $guard s, waiting on something"
  elif [ "$code" = 137 ] && [ -z "$clock" ]; then
    echo "$case was killed, and its time couldn't be read to say by what"
  elif [ "$code" = 137 ]; then
    echo "$case was killed: out of memory, or past its hard $hard s"
  elif [ "$code" = 152 ]; then
    echo "$case ran past its $limit s of its own time"
  elif [ "$code" = 125 ]; then
    echo "$case never ran: its limits couldn't be set, or timeout itself failed (125)"
  else
    echo "$case failed, exit code $code"
  fi
}
# A figure `time` gave, as it's meant. Bash rounds a fraction of .9995 s or
# more up to a thousand thousandths without carrying the second, and writes
# the digit past 9 as ':', so a case its limit signalled at 1.000 s of its
# own reads 0.:00; that's the next whole second.
carried() {
  if [[ $1 =~ ^([0-9]+)\.:0*$ ]]; then
    echo "$((BASH_REMATCH[1] + 1)).000"
  else
    echo "$1"
  fi
}
cases=(*.qml)
if [ "${#cases[@]}" = 0 ]; then
  echo "no cases to run in $PWD" >&2
  exit 1
fi
failed=()
unrun=()
stopped=
for case in "${cases[@]}"; do
  if [ -n "$stopped" ]; then
    unrun+=("$case")
    continue
  fi
  # The case's own output goes to the log, and only the timing to `used`,
  # since `time` on a subshell writes to the subshell's stderr as it stands
  # outside the case's own redirection.
  { time (run "$case" 2>&3); } 3>&2 2>"$used"
  code=$?
  # The timing is the last line: bash says a job a signal ended above it.
  timing=$(tail -n 1 "$used")
  read -r user kernel clock <<<"$timing"
  user=$(carried "$user")
  kernel=$(carried "$kernel")
  clock=$(carried "$clock")
  seconds=
  if [[ $user =~ ^[0-9.]+$ && $kernel =~ ^[0-9.]+$ && $clock =~ ^[0-9]+(\.[0-9]+)?$ ]]; then
    seconds=${clock%.*}
  fi
  if [ "$code" != 0 ]; then
    said=$(ended "$case" "$code" "$seconds")
    echo "$said" >&2
    failed+=("$said")
    if hung "$code" "$seconds"; then
      stopped=$case
    fi
  fi
  # A case that never ran took nothing worth saying.
  if [ "$code" = 125 ]; then
    continue
  fi
  if [ -z "$seconds" ]; then
    echo "$case's time couldn't be read: '$timing'" >&2
    continue
  fi
  own=$(awk -v user="$user" -v kernel="$kernel" 'BEGIN { printf "%.0f", user + kernel }')
  echo "$case took $own s of its $limit, $seconds s by the clock" >&2
done
if [ "${#unrun[@]}" != 0 ]; then
  echo "left unrun after $stopped: ${unrun[*]}" >&2
fi
if [ "${#failed[@]}" != 0 ]; then
  printf '%s of %s cases failed:\n' "${#failed[@]}" "${#cases[@]}" >&2
  printf '  %s\n' "${failed[@]}" >&2
  if [ "${#unrun[@]}" != 0 ]; then
    printf '  %s left unrun\n' "${unrun[@]}" >&2
  fi
  exit 1
fi
