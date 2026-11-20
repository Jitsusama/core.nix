# shellcheck shell=bash
# Runs tuple.service's own ExecStartPost with systemctl and the daemon's log
# swapped for its own, and its wait cut to five seconds: it must return while
# the unit is still starting, say nothing until the unit is active and the
# log says the signaller has connected, a connection of another kind not
# counting, and then say how long that took from the log's first timed line,
# in the words modules/health reads, across midnight too. Each log opens as
# Tuple's do, with a line that has no time. A clock that stepped either way
# between the two says only that the signaller connected, and when; a unit
# never active, and a caller's name posing as the line, are waited out and
# said to be; each in words health doesn't take for a ready line. Every case
# waits for the waiter to end, so a second line can't follow unseen. It asks
# systemd only until the unit is active, gives up on an ask systemd doesn't
# answer, serves daemons systemd couldn't choose between, and leaves at once
# when systemd takes it for the unit's main process, as it does when the
# start forked no daemon.
#
# Given the ExecStartPost's program, and the systemctl and log it names.
set -euo pipefail

post=$1
systemctl=$2
log=$3
here=$PWD

mkdir -p bin
# Answers the one ask the post makes, the unit active only once the case
# has said it's up, and refuses any other ask, so one of the wrong manager or
# unit fails here as it would say nothing on a machine. Its main process is a
# daemon's, or none when the case says its start left `many`, or, once the
# case has found it, the post's own waiter when the case says it's `alone`,
# as systemd takes it then. Each ask is counted, so a case knows the post has
# looked; one the case says hangs is never answered.
cat >bin/systemctl <<EOF
#!${BASH}
[[ "\$*" == "--user show --property=ActiveState,MainPID tuple.service" ]] || exit 1
echo >>${here}/asked
# A systemd that never answers: far past the post's two-second bound on it.
if [[ -e ${here}/hangs ]]; then rm ${here}/hangs; exec sleep 10; fi
if [[ ! -e ${here}/up ]]; then printf 'MainPID=0\nActiveState=activating\n'
elif [[ -e ${here}/alone && -e ${here}/main ]]; then
  printf 'MainPID=%s\nActiveState=active\n' "\$(cat ${here}/main)"
elif [[ -e ${here}/alone ]]; then printf 'MainPID=0\nActiveState=activating\n'
elif [[ -e ${here}/many ]]; then printf 'MainPID=0\nActiveState=active\n'
else printf 'MainPID=7\nActiveState=active\n'; fi
EOF
chmod +x bin/systemctl

sed "s|${systemctl}|${here}/bin/systemctl|; s|${log}|${here}/log.txt|; s| 30\$| 5|" \
  "${post}" >started
chmod +x started
grep -q "${here}/bin/systemctl" started
grep -q "${here}/log.txt" started
grep -q ' 5$' started

failed=0
# Three of the post's looks, a tenth of a second apart, any of which would say
# its line too early if it didn't wait for what the case holds back.
THREE_LOOKS=0.3
# The post waits five seconds, which it counts in whole seconds that can run
# up to one ahead: five leave an ask systemd never answers, which takes two,
# and the asks a case waits on before it lets the unit be active, time to
# spare on a loaded machine.
gave_up="tuple: its signaller hadn't connected 5 s after it started"
never_active="tuple: systemd hadn't said it was active 5 s after it started"
untimed="ms after its log began, past the 5 s it times"
# What modules/health takes for a ready line (launches.rs's READY_FOLLOWED),
# which journalctl matches whatever its case, as it's written in lower case.
ready='\b(ready|loaded)\b.* in [0-9]+ ms\b'

# Fails the case `what` when the post has said anything yet.
quiet() {
  if [[ -s said ]]; then
    local got
    got=$(cat said)
    echo "$1: said it $2: ${got}"
    failed=1
  fi
}

# Fails the case `what` when health would read what the post said as ready.
unready() {
  if grep -qiE "${ready}" said; then
    echo "$1: health would read it as ready"
    failed=1
  fi
}

# Waits for the post to say its line and for its waiter to end, and fails the
# case `what` when all it said isn't `wanted`.
says_only() {
  says "$1" "$2"
  ended "$1" 0
  says "$1" "$2"
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

# Sets `running` when the process at `proc` runs with this case's log among
# its words, as the post, its waiter and the waiter's greps of it do. It
# forks nothing, so a look over every process ends while the waiter waits.
runs() {
  local words word
  running=""
  mapfile -d '' words 2>/dev/null <"$1/cmdline" || return 0
  for word in "${words[@]}"; do
    [[ ${word} != "${here}/log.txt" ]] || running=yes
  done
}

# Sets `waiters` to the post's waiters, once the post has returned: each
# process running with the case's log whose parent doesn't, as the waiter's
# own greps of the log do.
waiter() {
  local proc parent
  waiters=()
  for proc in /proc/[0-9]*; do
    runs "${proc}"
    [[ -n ${running} ]] || continue
    read -r _ _ _ parent _ 2>/dev/null <"${proc}/stat" || continue
    runs "/proc/${parent}"
    [[ -n ${running} ]] || waiters+=("${proc#/proc/}")
  done
}

# Waits until the process `pid` has ended, looking `looks` times twenty times
# a second, and fails the case `what` when it hasn't.
gone() {
  local pid=$1 what=$2 looks=$3 look
  for ((look = 0; look < looks; look++)); do
    [[ -e /proc/${pid} ]] || return 0
    # It ends on one of its looks, each a tenth of a second from the last.
    sleep 0.05
  done
  echo "${what}: the waiter still runs"
  failed=1
}

# Waits until the case's waiter has ended, so whatever it'd say is said, for
# ten seconds at most: past the five it waits and the two its last ask of
# systemd may take. It fails the case `what` on more than one waiter, or on
# none when `least` is 1, as a case that must find it still waiting is, so it
# can't pass before the waiter has looked.
ended() {
  local what=$1 least=$2 pid
  waiter
  if ((${#waiters[@]} < least || ${#waiters[@]} > 1)); then
    echo "${what}: found ${#waiters[@]} waiters"
    failed=1
  fi
  for pid in "${waiters[@]}"; do gone "${pid}" "${what}" 200; done
}

# Starts a case: the unit starting, or as named, already active (`up`), with
# daemons systemd couldn't choose between (`many`), with no daemon but the
# waiter (`alone`), or first asked about in vain (`hangs`), and what the post
# says from now in a file of the case's own, so a post still waiting from the
# case before can't speak into it. What it asks, and the stand-in's flags,
# are read by path, so every case ends its waiter before the next.
starts() {
  local each
  rm -f up many alone main hangs said asked
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
looked 2 "a start"
echo '00:00:00.700|1|7|info|S.cpp:181|signaler state changed: synchronizing -> connected' \
  >>log.txt
looked 4 "a start"
quiet "a start" "before the unit was active"
touch up
says_only "a start" "tuple: ready in 1200 ms"

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
says_only "an active unit" "tuple: ready in 250 ms"

printf '%s\n' \
  'launching from pid 1...' \
  '10:00:00.000|1|1|info|closing fd 0' \
  '10:00:00.250|1|7|info|Signaler.cpp:181|signaler state changed: synchronizing -> connected' \
  >log.txt
starts up hangs
says_only "an ask systemd doesn't answer" "tuple: ready in 250 ms"

starts up many
says_only "daemons systemd couldn't choose between" "tuple: ready in 250 ms"

# The log already says the signaller connected, so only leaving at once
# keeps it quiet.
starts up alone
waiter
if ((${#waiters[@]} == 1)); then
  # Written whole before the stand-in can read it, so it never reads it empty.
  echo "${waiters[0]}" >main.new
  mv main.new main
  gone "${waiters[0]}" "a start that forked no daemon" 40
else
  echo "a start that forked no daemon: found ${#waiters[@]} waiters, not one"
  failed=1
fi
quiet "a start that forked no daemon" "as the unit's main process"

starts
ended "a unit never active" 1
says "a unit never active" "${never_active}"
unready "a unit never active"

printf '%s\n' \
  'launching from pid 1...' \
  '10:00:05.000|1|1|info|closing fd 0' \
  '10:00:00.250|1|7|info|Signaler.cpp:181|signaler state changed: synchronizing -> connected' \
  >log.txt
starts up
says_only "a clock stepped back" "tuple: its signaller connected 86395250 ${untimed}"
unready "a clock stepped back"

# Beside the clock stepped back, which the wrap past midnight turns into most
# of a day, a second past the wait is what holds the bound to the wait, not
# to a day or the thirty seconds Tuple waits.
printf '%s\n' \
  'launching from pid 1...' \
  '10:00:00.000|1|1|info|closing fd 0' \
  '10:00:06.000|1|7|info|Signaler.cpp:181|signaler state changed: synchronizing -> connected' \
  >log.txt
starts up
says_only "a clock stepped forward" "tuple: its signaller connected 6000 ${untimed}"
unready "a clock stepped forward"

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
ended "a caller named like the line" 1
says "a caller named like the line" "${gave_up}"
unready "a caller named like the line"

exit "${failed}"
