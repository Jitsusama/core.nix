# shellcheck shell=bash
# Runs tuple.service's own ExecStartPost with systemctl and the daemon's log
# swapped for its own, and its wait cut to three seconds: it must return while
# the unit is still starting, say nothing until the unit is active and the
# log says the signaller has connected, a connection of another kind not
# counting, and then say how long that took from the log's first timed line,
# in the words modules/health reads, across midnight too. Each log opens as
# Tuple's do, with a line that has no time. A clock that stepped back
# between the two, or a caller's name posing as the line, says nothing.
#
# Given the ExecStartPost's program, and the systemctl and log it names.
set -euo pipefail

post=$1
systemctl=$2
log=$3
here=$PWD

mkdir -p bin
# Answers the one ask the post makes, the unit active only once the case
# has said it's up, and refuses any other, so an ask of the wrong manager
# or unit fails here as it would say nothing on a machine.
cat >bin/systemctl <<EOF
#!${BASH}
[[ "\$*" == "--user show --property=ActiveState --value tuple.service" ]] || exit 1
if [[ -e ${here}/up ]]; then echo active; else echo activating; fi
EOF
chmod +x bin/systemctl

sed "s|${systemctl}|${here}/bin/systemctl|; s|${log}|${here}/log.txt|; s| 30\$| 3|" \
  "${post}" >started
chmod +x started
grep -q "${here}/bin/systemctl" started
grep -q "${here}/log.txt" started
grep -q ' 3$' started

failed=0
# Three of the post's looks, a tenth of a second apart, any of which would say
# its line too early if it didn't wait for what the case holds back.
THREE_LOOKS=0.3
# Past the three seconds the post waits, which it counts in whole seconds, so
# a case that must say nothing has seen the post give up, not just not look.
WAITED_OUT=3.5

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
  for _ in {1..40}; do
    [[ -s said ]] && break
    # The post says nothing but its line, so its output is read again until
    # it's there, for two seconds at most.
    sleep 0.05
  done
  got=$(cat said)
  if [[ ${got} != "${wanted}" ]]; then
    echo "${what}: wanted '${wanted}' but said '${got}'"
    failed=1
  fi
}

# Starts a case: the unit starting, or already active when given `up`, and
# what the post says from now in a file of the case's own, so a post still
# waiting from the case before can't speak into it.
starts() {
  rm -f up said
  [[ $# -eq 0 ]] || touch up
  ./started tuple.service >said
}

printf '%s\n' \
  'launching from pid 1...' \
  '23:59:59.500|1|1|info|closing fd 0' \
  '00:00:00.100|1|7|info|S.cpp:845|signaler realtime connection state: connecting -> connected' \
  '00:00:00.101|1|7|info|S.cpp:181|signaler state changed: connecting -> synchronizing' \
  >log.txt
starts
sleep "${THREE_LOOKS}"
echo '00:00:00.700|1|7|info|S.cpp:181|signaler state changed: synchronizing -> connected' \
  >>log.txt
sleep "${THREE_LOOKS}"
quiet "a start" "before the unit was active"
touch up
says "a start" "tuple: ready in 1200 ms"

printf '%s\n' \
  'launching from pid 1...' \
  '10:00:00.000|1|1|info|closing fd 0' \
  >log.txt
starts up
sleep "${THREE_LOOKS}"
quiet "an active unit" "before the signaller connected"
echo '10:00:00.250|1|7|info|Signaler.cpp:181|signaler state changed: synchronizing -> connected' \
  >>log.txt
says "an active unit" "tuple: ready in 250 ms"

printf '%s\n' \
  'launching from pid 1...' \
  '10:00:05.000|1|1|info|closing fd 0' \
  '10:00:00.250|1|7|info|Signaler.cpp:181|signaler state changed: synchronizing -> connected' \
  >log.txt
starts up
sleep "${WAITED_OUT}"
quiet "a clock stepped back" "at all"

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
quiet "a caller named like the line" "at all"

exit "${failed}"
