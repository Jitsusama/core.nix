# shellcheck shell=bash
# How the shell is handed over to a new one (modules/quickshell/shell-hands-over.sh),
# run with bash and that script as its argument. A stand-in Quickshell is both shells: the
# old one, started in a group of its own as the unit's would be, and the new one the script
# starts held, which says it's ready and kept home and writes Super+S's mark as it loads, as
# the real one does. Each answers the launcher's IPC call from what the case says. Stand-in
# systemctl and systemd-notify write down what they're asked, so nothing real is touched.
set -u
script=$1
here=$PWD
mkdir -p bin run
export XDG_RUNTIME_DIR=$here/run
{
  printf '#!%s\n' "${BASH}"
  cat << 'EOF'
runtime=${XDG_RUNTIME_DIR}
if [[ ${1:-} == ipc ]]; then
  if [[ -e ${runtime}/held-$3 ]]; then
    echo '{"kept":true,"open":false}'
  else
    printf '{"open":%s}\n' "$(cat "${runtime}/old-open" 2> /dev/null || echo false)"
  fi
  exit 0
fi
trap 'exit 0' TERM
if [[ -n ${QS_HELD:-} ]]; then
  [[ ${STANDIN_FAILS:-0} == 1 ]] && exit 3
  touch "${runtime}/held-$$"
  echo closed > "${runtime}/quickshell-talk-line"
  # The old shell's line, come up while the new one loaded.
  [[ ${STANDIN_OLD_TALKS:-0} == 1 ]] && echo open > "${runtime}/quickshell-talk-line"
  trap 'echo released > "${runtime}/released"' USR1
  echo ready > "${QS_HELD}"
fi
while :; do sleep 0.05; done
EOF
} > bin/quickshell
{
  printf '#!%s\n' "${BASH}"
  cat << 'EOF'
echo "$*" >> "${XDG_RUNTIME_DIR}/asked"
EOF
} > bin/systemctl
{
  printf '#!%s\n' "${BASH}"
  cat << 'EOF'
echo "$*" >> "${XDG_RUNTIME_DIR}/notified"
EOF
} > bin/systemd-notify
chmod +x bin/quickshell bin/systemctl bin/systemd-notify
export PATH=$here/bin:$PATH
# What went wrong, said once every case has run.
failures=()

# Starts one case from nothing said, the old shell running as the unit's main process.
case_of() {
  rm -f run/*
  unset STANDIN_FAILS STANDIN_OLD_TALKS
  setsid "${here}/bin/quickshell" &
  MAINPID=$!
  export MAINPID
}
# Ends whatever a case left running.
over() {
  pkill -TERM -f "${here}/bin/quickshell" 2> /dev/null || true
  wait 2> /dev/null || true
}
# The old shell is this test's child, so once ended it stays a zombie until waited on.
alive() {
  local state
  state=$(ps -o stat= -p "$1" 2> /dev/null || true)
  [[ -n ${state} && ${state} != Z* ]]
}
new_shell() {
  local held
  for held in run/held-*; do
    [[ -e ${held} ]] && echo "${held##*-}"
  done
}
handed_over() {
  local new
  new=$(new_shell)
  [[ -n ${new} ]] && grep -qx -- "--pid=${new}" run/notified 2> /dev/null &&
    ! alive "${MAINPID}" && [[ -e run/released ]]
}
# Runs the script in the background, as the unit's reload would, under the options
# writeShellApplication gives it.
hand_over() {
  bash -o errexit -o nounset -o pipefail "${script}" "${here}/bin/quickshell" > run/said 2>&1 &
  handing=$!
}
# Waits up to 5 s for the handover to be done.
done_within() {
  local i
  for ((i = 0; i < 50; i++)); do
    handed_over && return 0
    sleep 0.1
  done
  return 1
}

case_of
hand_over
wait "${handing}" || failures+=("a handover with nothing open failed: $(< run/said)")
done_within || failures+=("nothing open, the shell wasn't handed over: $(< run/said)")
over

# The old launcher open, it waits for Joel to close it.
case_of
echo true > run/old-open
hand_over
# The script looks every 50 ms, so it would have ended the old shell by now were it not
# waiting: these 2 s are what's proven.
sleep 2
! handed_over || failures+=("the old launcher was ended open")
echo false > run/old-open
done_within || failures+=("the old launcher's close didn't hand the shell over: $(< run/said)")
over

# Super+S's line come up on the old shell while the new one loaded holds it the same.
case_of
export STANDIN_OLD_TALKS=1
hand_over
# As above, the 2 s in which the old shell must not be ended.
sleep 2
! handed_over || failures+=("the old shell was ended with Super+S's line up")
echo closed > run/quickshell-talk-line
done_within || failures+=("the talk line's close didn't hand the shell over: $(< run/said)")
over

case_of
echo locked > run/quickshell-locked
hand_over
wait "${handing}"
grep -qx -- "--user --no-block restart quickshell.service" run/asked 2> /dev/null ||
  failures+=("a locked screen didn't restart the shell: $(< run/said)")
held=$(new_shell)
[[ -z ${held} ]] || failures+=("a locked screen started a held shell")
over

# A new shell that ends as it loads leaves the old one the unit's.
case_of
export STANDIN_FAILS=1
hand_over
if wait "${handing}"; then failures+=("a new shell that failed was taken as handed over"); fi
alive "${MAINPID}" || failures+=("a new shell that failed ended the old one")
[[ ! -e run/notified ]] || failures+=("a new shell that failed was made the unit's")
over

((${#failures[@]} == 0)) || {
  printf '%s\n' "${failures[@]}"
  exit 1
}
