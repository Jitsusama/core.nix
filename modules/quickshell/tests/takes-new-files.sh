# shellcheck shell=bash
# How the shell takes a switch's files (modules/quickshell/takes-new-files.sh),
# run with bash and that script as its argument. A stand-in systemctl says
# whether the shell is running and writes down every handover and restart
# it's asked for, refusing a handover when told to, so no real shell is
# handed over or restarted; the launcher's mark is written as the
# shell writes it, its time dating the close, and Super+S's talk line's
# beside it.
set -u
script=$1
here=$PWD
mkdir -p bin run
export XDG_RUNTIME_DIR=$here/run
{
  printf '#!%s\n' "${BASH}"
  cat << 'EOF'
if [[ $* == *is-active* ]]; then exit "${SHELL_STOPPED:-0}"; fi
echo "$*" >> "${XDG_RUNTIME_DIR}/asked"
if [[ $* == *reload* ]]; then exit "${HANDOVER_REFUSED:-0}"; fi
EOF
} > bin/systemctl
chmod +x bin/systemctl
export PATH=$here/bin:$PATH
failed=0

# Runs one case from nothing asked, the shell running on files "old".
case_of() {
  rm -f run/asked run/quickshell-launcher run/quickshell-talk-line run/said
  ln -sfn old run/quickshell-files
  unset SHELL_STOPPED HANDOVER_REFUSED
}
# Whether it took the new files, handed over, or restarted once refused.
restarted() { grep -qx -- "--user reload quickshell.service" run/asked 2> /dev/null; }
restarted_after_all() { grep -qx -- "--user restart quickshell.service" run/asked 2> /dev/null; }
wrong() {
  echo "$1"
  failed=1
}

case_of
said=$(bash "${script}" old 1)
! restarted || wrong "a shell already on its files was restarted: ${said}"

case_of
said=$(SHELL_STOPPED=3 bash "${script}" new 1)
! restarted || wrong "a stopped shell was started, which its own start does: ${said}"

case_of
echo closed > run/quickshell-launcher
touch -d '-60 seconds' run/quickshell-launcher
said=$(bash "${script}" new 1)
restarted || wrong "a launcher shut a minute didn't bring the new files: ${said}"
[[ ${said} == *"launcher shut 30 s"* ]] || wrong "the restart didn't say why: ${said}"
! restarted_after_all || wrong "a handover done was followed by a restart: ${said}"

# A handover refused leaves the old shell, so it restarts onto the files.
case_of
echo closed > run/quickshell-launcher
touch -d '-60 seconds' run/quickshell-launcher
said=$(HANDOVER_REFUSED=1 bash "${script}" new 1)
restarted_after_all || wrong "a handover refused left the shell on its old files: ${said}"
[[ ${said} == *"handover refused"* ]] || wrong "the restart after a refusal didn't say why: ${said}"

case_of
echo closed > run/quickshell-launcher
started=$(date +%s)
said=$(bash "${script}" new 1)
restarted || wrong "a launcher shut a moment ago held the shell past its bound: ${said}"
[[ ${said} == *"stayed in use"* ]] ||
  wrong "a launcher shut a moment ago wasn't waited on as in use: ${said}"
ended=$(date +%s)
((ended - started >= 1)) || wrong "it didn't wait for its bound"

# Past its bound it still waits for the launcher to close, so it never
# shuts the launcher under Joel's hands, and restarts the shell as it does.
case_of
echo open > run/quickshell-launcher
bash "${script}" new 1 > run/said &
taking=$!
sleep 2
! restarted || wrong "an open launcher was shut past the bound: $(< run/said)"
echo closed > run/quickshell-launcher
wait "${taking}"
restarted || wrong "the launcher's close past the bound didn't bring the new files"
said=$(< run/said)
[[ ${said} == *"stayed in use"* ]] || wrong "the bound's restart didn't say why: ${said}"

# Super+S's line holds it as the launcher does, open or shut a moment ago.
case_of
echo closed > run/quickshell-launcher
touch -d '-60 seconds' run/quickshell-launcher
echo open > run/quickshell-talk-line
bash "${script}" new 1 > run/said &
taking=$!
sleep 2
! restarted || wrong "an open talk line was shut past the bound: $(< run/said)"
echo closed > run/quickshell-talk-line
wait "${taking}"
restarted || wrong "the talk line's close didn't bring the new files"

case_of
echo closed > run/quickshell-talk-line
said=$(bash "${script}" new 1)
[[ ${said} == *"stayed in use"* ]] ||
  wrong "a talk line shut a moment ago wasn't waited on as in use: ${said}"

case_of
rm run/quickshell-files
said=$(bash "${script}" new 1)
restarted || wrong "a shell that never said its files wasn't brought onto them: ${said}"

exit "${failed}"
