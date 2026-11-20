# shellcheck shell=bash
# Runs tuple.service's own ExecStartPost with systemctl and the daemon's log
# swapped for its own, and its wait cut to ten seconds: it must return while
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
# answer, serves daemons systemd couldn't choose between, and says so and
# leaves at once when systemd takes it for the unit's main process, as it
# does when the start forked no daemon.
#
# Given the ExecStartPost's program, the systemctl and log it names, what
# modules/health takes for a ready line (launches.rs's READY_FOLLOWED), and
# the unit's SyslogIdentifier, SyslogLevel, LogLevelMax, LogFilterPatterns
# and StandardOutput, each as Name=value, empty when unset, so a swapped
# argument fails. It refuses settings health couldn't read the unit by,
# having first judged a table of its own, since the unit's real settings
# reach few of the refusals.
set -euo pipefail

post=$1
systemctl=$2
log=$3
ready=$4
here=$PWD

# Sets `value` to the setting named `$2`, given as `$1`.
setting() {
  if [[ $1 != "$2="* ]]; then
    echo "given '$1' where $2 belongs"
    exit 1
  fi
  value=${1#"$2="}
}
setting "$5" SyslogIdentifier
named=${value}
setting "$6" SyslogLevel
level=${value}
setting "$7" LogLevelMax
ceiling=${value}
setting "$8" LogFilterPatterns
filters=${value}
setting "$9" StandardOutput
output=${value}

# Sets `n` to the syslog priority a level names, by name or by number as
# systemd takes it, or -1 for none.
priority() {
  case $1 in
  emerg) n=0 ;;
  alert) n=1 ;;
  crit) n=2 ;;
  err) n=3 ;;
  warning) n=4 ;;
  notice) n=5 ;;
  info) n=6 ;;
  debug) n=7 ;;
  [0-7]) n=$1 ;;
  *) n=-1 ;;
  esac
}

# Sets `refused` to why health couldn't read the unit by its name, level,
# ceiling, filters and output, or to nothing. Health files a ready time under
# the name its line goes out with, which must be Tuple's, not the script's,
# which changes with each build; it reads ready lines only from notice to
# information priority (health/src/journal.rs), from the journal; and it times
# a start from systemd's Stopping and Started lines, at info, which a user
# unit's LogLevelMax keeps out of the journal when it sits below info (a user
# unit under a notice ceiling lost its Started line on systemd 262,
# 2026-11-20). An info floor keeps the ready line too, whichever manager
# applies the ceiling. LogFilterPatterns works only for system services, as
# systemd.exec(5) says, but any is refused, as nothing sets one and a systemd
# that honoured it could drop the ready line. A level or ceiling that isn't
# one is a slip in the unit, refused as that. Unset, a level is info, no
# ceiling holds and output is journalled.
judged() {
  local name=$1 at=$2 most=$3 filter=$4 out=$5
  refused=
  priority "${at:-info}"
  local said_at=${n}
  priority "${most:-debug}"
  if [[ -z ${name} ]]; then
    refused="tuple.service sets no SyslogIdentifier, so its lines go out under the script's name"
  elif [[ ${name} != tuple ]]; then
    refused="tuple.service names its lines '${name}', not tuple"
  elif ((said_at < 0)); then
    refused="tuple.service sets SyslogLevel to '${at}', which isn't a level"
  elif ((said_at < 5 || said_at > 6)); then
    refused="tuple.service logs at ${at}, where health doesn't read"
  elif ((n < 0)); then
    refused="tuple.service sets LogLevelMax to '${most}', which isn't a level"
  elif ((n < 6)); then
    refused="tuple.service keeps nothing past ${most}, so not systemd's Started line"
  elif [[ -n ${filter} ]]; then
    refused="tuple.service filters its lines by '${filter}', which may drop the ready line"
  elif [[ ${out} != "" && ${out} != journal && ${out} != journal+console ]]; then
    refused="tuple.service sends its output to ${out}, not the journal"
  fi
}

# Each row: taken, or words its refusal must hold, so each is refused for its
# own reason; then a name, level, ceiling, filters and output. A row naming
# neither, or blank, fails.
while IFS='|' read -r want name at most filter out; do
  judged "${name}" "${at}" "${most}" "${filter}" "${out}"
  if [[ ${want} == taken ]]; then
    wrong=${refused}
  elif [[ -z ${want} || ${refused} != *"${want}"* ]]; then
    wrong="wanted '${want}' but got '${refused:-taken}'"
  else
    wrong=
  fi
  if [[ -n ${wrong} ]]; then
    echo "judged ${name}|${at}|${most}|${filter}|${out} wrongly: ${wrong}"
    exit 1
  fi
done <<'SETTINGS'
sets no SyslogIdentifier|||||
names its lines|tuple-says-ready||||
logs at debug|tuple|debug|||
logs at warning|tuple|warning|||
logs at 7|tuple|7|||
SyslogLevel to 'loud', which isn't|tuple|loud|||
keeps nothing past notice|tuple|notice|notice||
keeps nothing past warning|tuple||warning||
keeps nothing past 5|tuple||5||
LogLevelMax to 'quiet', which isn't|tuple||quiet||
filters its lines|tuple|||~ready|
sends its output to null|tuple||||null
sends its output to file|tuple||||file:/tmp/x
taken|tuple||||
taken|tuple|6|||journal
taken|tuple|notice|||journal+console
taken|tuple||info||
taken|tuple|notice|7||
SETTINGS

judged "${named}" "${level}" "${ceiling}" "${filters}" "${output}"
if [[ -n ${refused} ]]; then
  echo "${refused}"
  exit 1
fi

# journalctl matches health's pattern as PCRE2, as grep -P does, and whatever
# its case only while it holds no capital letter.
fold=--ignore-case
if [[ ${ready} =~ [[:upper:]] ]]; then fold=--no-ignore-case; fi

mkdir -p bin
# Answers the one ask the post makes, the unit active only once the case
# has said it's up, and refuses any other ask, so one of the wrong manager or
# unit fails here as it would say nothing on a machine. Its main process is a
# daemon's, or none when the case says its start left `many`, or the post's
# own waiter when the case says it's `alone`, as systemd takes it then. Each
# ask is counted, so a case knows the post has looked; one the case says
# hangs is never answered. The case's folder is written in quoted once, so
# any name it has is read back as itself.
printf -v quoted %q "${here}"
cat >bin/systemctl <<EOF
#!${BASH}
[[ "\$*" == "--user show --property=ActiveState,MainPID tuple.service" ]] || exit 1
here=${quoted}
echo >>"\${here}/asked"
# A systemd that never answers: far past the post's two-second bound on it.
if [[ -e \${here}/hangs ]]; then rm "\${here}/hangs"; exec sleep 10; fi
# Whether the process holds the case's log among its words, read here in
# bash, so no process this starts holds it too and looks like a waiter.
holds() {
  local word
  while IFS= read -r -d '' word; do
    [[ \${word} == "\${here}/log.txt" ]] && return 0
  done 2>/dev/null </proc/\$1/cmdline
  return 1
}
if [[ ! -e \${here}/up ]]; then printf 'MainPID=0\nActiveState=activating\n'
elif [[ -e \${here}/alone && -e \${here}/returned ]]; then
  # Alone in the unit once the case has seen the post return, the waiter is
  # what systemd takes for its main process: the topmost of the processes up
  # from this ask with the case's log among their words, however many
  # subshells sit between. Found here, on the ask, rather than by the case
  # looking over every process, which a loaded machine slows past the wait.
  # With none found, the case hears so, rather than taking what's said for
  # the post's.
  main=0 pid=\${PPID}
  for _ in 1 2 3 4 5 6 7 8; do
    if holds "\${pid}"; then main=\${pid}
    elif ((main > 0)); then break; fi
    read -r _ _ _ pid _ 2>/dev/null </proc/\${pid}/stat || break
  done
  if ((main == 0)); then echo >>"\${here}/lost"; state=activating; else state=active; fi
  printf 'MainPID=%s\nActiveState=%s\n' "\${main}" "\${state}"
elif [[ -e \${here}/alone ]]; then printf 'MainPID=0\nActiveState=activating\n'
elif [[ -e \${here}/many ]]; then printf 'MainPID=0\nActiveState=active\n'
else printf 'MainPID=7\nActiveState=active\n'; fi
EOF
chmod +x bin/systemctl

sed "s|${systemctl}|${here}/bin/systemctl|; s|${log}|${here}/log.txt|; s| 30\$| 10|" \
  "${post}" >started
chmod +x started
# Each swap must have taken, or the cases would run the post on the
# machine's systemctl, its log or its whole wait.
for swapped in "${here}/bin/systemctl" "${here}/log.txt" ' 10$'; do
  if ! grep -q -- "${swapped}" started; then
    echo "the post's wrapper didn't take the test's swap for '${swapped}':"
    echo "it says its systemctl, its log or its wait of 30 seconds otherwise now"
    exit 1
  fi
done

failed=0
# Three of the post's looks, a tenth of a second apart, any of which would say
# its line too early if it didn't wait for what the case holds back.
THREE_LOOKS=0.3
# The post waits ten seconds: they leave an ask systemd never answers, which
# takes two, and the asks a case waits on before it lets the unit be active,
# time to spare on a loaded machine.
gave_up="tuple: its signaller hadn't connected 10 s after it started"
never_active="tuple: systemd hadn't said it was active 10 s after it started"
untimed="ms after its log began, past the 10 s it times"
no_daemon="tuple: its start left no daemon of its own, so none to time"
# Nearly all of the post's ten seconds, in hundredths, which a waiter that
# gives up must have waited.
WAITED_OUT=950

# The look for the waiter takes GNU grep's flags, which the check's own grep
# has. A grep refusing them, as another first on a hand run's PATH may,
# would find no waiter, which most cases take for one already ended, so it
# ends the run instead.
look=0
grep --files-with-matches --null-data --line-regexp --fixed-strings -- x /dev/null || look=$?
if ((look > 1)); then
  echo "grep can't look for the post's waiter: it refused the flags the look takes"
  exit 1
fi

# Sets `now` to the hundredths of a second since boot, as the post counts its
# wait, so a clock stepped while a case runs can't move it.
since_boot() {
  local up _
  read -r up _ </proc/uptime
  now=$((10#${up/./}))
}

# Fails the case `what` when the post has said anything yet.
quiet() {
  if [[ -s said ]]; then
    local got
    got=$(cat said)
    echo "$1: said it $2: ${got}"
    failed=1
  fi
}

# Sets `taken` to whether journalctl, given health's ready pattern, would pick
# what the post said; health's parser then times only `X: ready in N ms`,
# which its own tests pin. A pattern grep can't read ends the run, rather
# than reading as none.
taken() {
  local found=0
  grep --quiet --perl-regexp "${fold}" -- "${ready}" said || found=$?
  case ${found} in
  0) taken=yes ;;
  1) taken=no ;;
  *)
    echo "grep couldn't read what the post said with health's ready line, ${ready}"
    exit 1
    ;;
  esac
}

# Fails the case `what` when health's pattern wouldn't pick what the post said.
readable() {
  taken
  if [[ ${taken} == no ]]; then
    echo "$1: health wouldn't read it as ready"
    failed=1
  fi
}

# Fails the case `what` when health's pattern would pick what the post said.
unready() {
  taken
  if [[ ${taken} == yes ]]; then
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
  for _ in {1..200}; do
    [[ -s said ]] && break
    # The post says nothing but its line, so its output is read again until
    # it's there, for ten seconds at most: an ask systemd never answers
    # takes two of them, and a loaded machine slows the rest.
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
# `what` when it hasn't within five seconds, which a loaded machine needs.
looked() {
  local n=$1 what=$2 count
  for _ in {1..100}; do
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
# its words, as the post, its waiter and the waiter's greps of it do.
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
  local held proc parent
  waiters=()
  # One grep reads every process's words, where a look at each from here
  # outlasts the waiter's wait on a loaded machine. A process that ends as
  # it reads is passed over.
  grep --files-with-matches --null-data --line-regexp --fixed-strings \
    -- "${here}/log.txt" /proc/[0-9]*/cmdline >holders 2>/dev/null || true
  while read -r held; do
    proc=${held%/cmdline}
    read -r _ _ _ parent _ 2>/dev/null <"${proc}/stat" || continue
    runs "/proc/${parent}"
    [[ -n ${running} ]] || waiters+=("${proc#/proc/}")
  done <holders
}

# Waits until the process `pid` has ended, looking `looks` times twenty times
# a second, and fails the case `what` when it hasn't.
gone() {
  local pid=$1 what=$2 looks=$3 look
  for ((look = 0; look < looks; look++)); do
    [[ -e /proc/${pid} ]] || return 0
    # It ends on one of its looks, each a twentieth of a second from the last.
    sleep 0.05
  done
  echo "${what}: the waiter still runs"
  failed=1
}

# Waits until the case's waiter has ended, so whatever it'd say is said, for
# fifteen seconds at most: past the ten it waits and the two its last ask of
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
  for pid in "${waiters[@]}"; do gone "${pid}" "${what}" 300; done
}

# Starts a case: the unit starting, or as named, already active (`up`), with
# daemons systemd couldn't choose between (`many`), with no daemon but the
# waiter (`alone`), or first asked about in vain (`hangs`), and what the post
# says from now in a file of the case's own, so a post still waiting from the
# case before can't speak into it. What it asks, and the stand-in's flags,
# are read by path, so every case ends its waiter before the next. Once the
# post returns, as systemd sees ExecStartPost end, the stand-in hears so; a
# post that doesn't return within POST_RETURNS is stopped, as systemd would
# stop it, killed KILL_AFTER seconds on if it won't stop, and fails the run,
# rather than holding the check without a word. One that returns at once but
# fails says so apart, since it points at the post's top half, not a hang.
POST_RETURNS=20
KILL_AFTER=5
starts() {
  local each status=0
  rm -f up many alone hangs returned lost said asked
  for each in "$@"; do touch "${each}"; done
  timeout -k "${KILL_AFTER}" "${POST_RETURNS}" ./started tuple.service >said || status=$?
  if ((status == 124 || status == 137)); then
    echo "the post didn't return within ${POST_RETURNS} seconds, as systemd would stop it"
    failed=1
  elif ((status != 0)); then
    echo "the post exited ${status} rather than returning as it should"
    failed=1
  fi
  touch returned
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
readable "a start"

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
readable "an active unit"

printf '%s\n' \
  'launching from pid 1...' \
  '10:00:00.000|1|1|info|closing fd 0' \
  '10:00:00.250|1|7|info|Signaler.cpp:181|signaler state changed: synchronizing -> connected' \
  >log.txt
starts up hangs
says_only "an ask systemd doesn't answer" "tuple: ready in 250 ms"
readable "an ask systemd doesn't answer"

starts up many
says_only "daemons systemd couldn't choose between" "tuple: ready in 250 ms"
readable "daemons systemd couldn't choose between"

# The log already says the signaller connected, so only leaving at once
# keeps it from saying it's ready, and a waiter that said so and sat out its
# wait would keep the unit up for nothing.
since_boot
began=${now}
starts up alone
says_only "a start that forked no daemon" "${no_daemon}"
since_boot
if [[ -e lost ]]; then
  echo "a start that forked no daemon: the stand-in found no waiter above the post's ask"
  failed=1
elif grep -qF "${no_daemon}" said && ((now - began >= WAITED_OUT)); then
  echo "a start that forked no daemon: the waiter sat out its wait rather than leaving"
  failed=1
fi
unready "a start that forked no daemon"

since_boot
began=${now}
starts
ended "a unit never active" 1
since_boot
if ((now - began < WAITED_OUT)); then
  echo "a unit never active: the post gave up before its ten seconds"
  failed=1
fi
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
  '10:00:11.000|1|7|info|Signaler.cpp:181|signaler state changed: synchronizing -> connected' \
  >log.txt
starts up
says_only "a clock stepped forward" "tuple: its signaller connected 11000 ${untimed}"
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
