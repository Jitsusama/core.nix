# shellcheck shell=bash
# Runs tuple.service's own ExecStartPost with systemctl and the daemon's log
# swapped for its own, and its wait cut to four seconds: it must return while
# the unit is still starting, say nothing until the unit is active and the
# log says the signaller has connected, a connection of another kind not
# counting, and then say how long that took from the log's first timed line,
# in the words modules/health reads, across midnight too. Each log opens as
# Tuple's do, with a line that has no time. A clock that stepped either way
# between the two says nothing; a caller's name posing as the line is waited
# out, and said to be. It asks systemd only until the unit is active, gives
# up on an ask systemd doesn't answer, and leaves a unit whose start forked
# no daemon at once.
#
# Given the ExecStartPost's program, and the systemctl and log it names.
set -euo pipefail

post=$1
systemctl=$2
log=$3
here=$PWD

mkdir -p bin
# Answers the one ask the post makes, the unit active only once the case
# has said it's up, with no main process when the case says its start forked
# none, and refuses any other ask, so one of the wrong manager or unit fails
# here as it would say nothing on a machine. Each ask is counted, so a case
# knows the post has looked; one the case says hangs is never answered.
cat >bin/systemctl <<EOF
#!${BASH}
[[ "\$*" == "--user show --property=ActiveState,MainPID tuple.service" ]] || exit 1
echo >>${here}/asked
# A systemd that never answers: far past the post's two-second bound on it.
if [[ -e ${here}/hangs ]]; then rm ${here}/hangs; exec sleep 10; fi
if [[ ! -e ${here}/up ]]; then printf 'MainPID=0\nActiveState=activating\n'
elif [[ -e ${here}/alone ]]; then printf 'MainPID=0\nActiveState=active\n'
else printf 'MainPID=7\nActiveState=active\n'; fi
EOF
chmod +x bin/systemctl

sed "s|${systemctl}|${here}/bin/systemctl|; s|${log}|${here}/log.txt|; s| 30\$| 4|" \
  "${post}" >started
chmod +x started
grep -q "${here}/bin/systemctl" started
grep -q "${here}/log.txt" started
grep -q ' 4$' started

failed=0
# Three of the post's looks, a tenth of a second apart, any of which would say
# its line too early if it didn't wait for what the case holds back.
THREE_LOOKS=0.3
# Past the four seconds the post waits, which it counts in whole seconds, so
# a case that must say nothing has seen the post give up, not just not look.
# Those whole seconds can run up to one ahead, so four leave an ask systemd
# never answers, which takes two, time for another.
WAITED_OUT=4.5
gave_up="tuple: its signaller hadn't connected 4 s after it started"

# Fails the case `what` when the post has said anything yet.
quiet() {
  if [[ -s said ]]; then
    local got
    got=$(cat said)
    echo "$1: said it $2: ${got}"
    failed=1
  fi
}

# Waits for the post to say its line, and fails the case when it isn't `wanted`.
says() {
  local what=$1 wanted=$2 got
  for _ in {1..80}; do
    [[ -s said ]] && break
    # The post says nothing but its line, so its output is read again until
    # it's there, for four seconds at most: an ask systemd never answers
    # takes two of them.
    sleep 0.05
  done
  got=$(cat said)
  if [[ ${got} != "${wanted}" ]]; then
    echo "${what}: wanted '${wanted}' but said '${got}'"
    failed=1
  fi
}

# How many times the post has asked systemd in this case.
asks() {
  if [[ -e asked ]]; then wc -l <asked; else echo 0; fi
}

# Waits until the post has asked systemd `n` times in all, and fails the case
# `what` when it hasn't within two seconds.
looked() {
  local n=$1 what=$2 count
  for _ in {1..40}; do
    count=$(asks)
    ((count >= n)) && return
    # Each of the post's asks lands a tenth of a second after the last.
    sleep 0.05
  done
  echo "${what}: the post asked systemd ${count} times, not ${n}"
  failed=1
}

# Fails the case `what` when the post has asked systemd other than `n` times.
asked() {
  local n=$1 what=$2 count
  count=$(asks)
  if ((count != n)); then
    echo "${what}: the post asked systemd ${count} times, not ${n}"
    failed=1
  fi
}

# Starts a case: the unit starting, or as named, already active (`up`), with
# no main process (`alone`), or first asked about in vain (`hangs`), and what
# the post says and asks from now in files of the case's own, so a post still
# waiting from the case before can't speak into it.
starts() {
  local each
  rm -f up alone hangs said asked
  for each in "$@"; do touch "${each}"; done
  ./started tuple.service >said
}

printf '%s\n' \
  'launching from pid 1...' \
  '23:59:59.500|1|1|info|closing fd 0' \
  '00:00:00.100|1|7|info|S.cpp:845|signaler realtime connection state: connecting -> connected' \
  '00:00:00.101|1|7|info|S.cpp:181|signaler state changed: connecting -> synchronizing' \
  >log.txt
starts
looked 3 "a start"
echo '00:00:00.700|1|7|info|S.cpp:181|signaler state changed: synchronizing -> connected' \
  >>log.txt
looked 6 "a start"
quiet "a start" "before the unit was active"
touch up
says "a start" "tuple: ready in 1200 ms"

printf '%s\n' \
  'launching from pid 1...' \
  '10:00:00.000|1|1|info|closing fd 0' \
  >log.txt
starts up
looked 1 "an active unit"
sleep "${THREE_LOOKS}"
quiet "an active unit" "before the signaller connected"
asked 1 "an active unit"
echo '10:00:00.250|1|7|info|Signaler.cpp:181|signaler state changed: synchronizing -> connected' \
  >>log.txt
says "an active unit" "tuple: ready in 250 ms"

printf '%s\n' \
  'launching from pid 1...' \
  '10:00:00.000|1|1|info|closing fd 0' \
  '10:00:00.250|1|7|info|Signaler.cpp:181|signaler state changed: synchronizing -> connected' \
  >log.txt
starts up hangs
says "an ask systemd doesn't answer" "tuple: ready in 250 ms"

# The log already says the signaller connected, so only leaving at once
# keeps it quiet.
starts up alone
looked 1 "a start that forked no daemon"
sleep "${THREE_LOOKS}"
quiet "a start that forked no daemon" "for a unit it alone held up"
asked 1 "a start that forked no daemon"

printf '%s\n' \
  'launching from pid 1...' \
  '10:00:05.000|1|1|info|closing fd 0' \
  '10:00:00.250|1|7|info|Signaler.cpp:181|signaler state changed: synchronizing -> connected' \
  >log.txt
starts up
sleep "${WAITED_OUT}"
quiet "a clock stepped back" "at all"

printf '%s\n' \
  'launching from pid 1...' \
  '10:00:00.000|1|1|info|closing fd 0' \
  '11:00:00.000|1|7|info|Signaler.cpp:181|signaler state changed: synchronizing -> connected' \
  >log.txt
starts up
sleep "${WAITED_OUT}"
quiet "a clock stepped forward" "at all"

# What a caller would name themselves to pose as the line, on a line with a
# place and one without.
posing='signaler state changed: a -> connected'

printf '%s\n' \
  'launching from pid 1...' \
  '10:00:00.000|1|1|info|closing fd 0' \
  "10:00:00.100|1|9|info|C:4|incoming call: 00:00:00.000|1|1|info|${posing}" \
  "10:00:00.200|1|9|info|incoming call: x|${posing}" \
  >log.txt
starts up
sleep "${WAITED_OUT}"
says "a caller named like the line" "${gave_up}"

exit "${failed}"
