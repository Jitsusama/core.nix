# shellcheck shell=bash
# How the shell takes a switch's files (modules/quickshell/takes-new-files.sh),
# run with bash and that script as its argument. A stand-in systemctl says
# whether the shell is running and writes down every restart it's asked
# for, so no real shell is restarted; the launcher's mark is written as the
# shell writes it, its time dating the close.
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
EOF
} > bin/systemctl
chmod +x bin/systemctl
export PATH=$here/bin:$PATH
failed=0

# Runs one case from nothing asked, the shell running on files "old".
case_of() {
  rm -f run/asked run/quickshell-launcher
  ln -sfn old run/quickshell-files
  unset SHELL_STOPPED
}
restarted() { grep -qx -- "--user restart quickshell.service" run/asked 2> /dev/null; }
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

case_of
echo closed > run/quickshell-launcher
started=$(date +%s)
said=$(bash "${script}" new 1)
restarted || wrong "a launcher shut a moment ago held the shell past its bound: ${said}"
[[ ${said} == *"stayed in use"* ]] ||
  wrong "a launcher shut a moment ago wasn't waited on as in use: ${said}"
ended=$(date +%s)
((ended - started >= 1)) || wrong "it didn't wait for its bound"

case_of
echo open > run/quickshell-launcher
said=$(bash "${script}" new 1)
restarted || wrong "an open launcher held the shell past its bound: ${said}"
[[ ${said} == *"stayed in use"* ]] || wrong "the bound's restart didn't say why: ${said}"

case_of
rm run/quickshell-files
said=$(bash "${script}" new 1)
restarted || wrong "a shell that never said its files wasn't brought onto them: ${said}"

exit "${failed}"
