# shellcheck shell=bash
# Proves systemd-run.sh adds --slice-inherit for an agent's --user call
# alone. Usage: systemd-run-test <wrapper>, where the wrapper was built in
# front of a stand-in that echoes what it's given and reads its caller's
# cgroup from ./cgroup.
set -euo pipefail
run=$1
manager=0::/user.slice/user-1000.slice/user@1000.service

# says <cgroup> <what the real one should be given> <the call...>
says() {
  echo "$1" >cgroup
  local want=$2
  shift 2
  local got
  got=$("${run}" "$@")
  if [[ "${got}" != "${want}" ]]; then
    echo "from $(<cgroup), $* became '${got}', not '${want}'"
    exit 1
  fi
}

says "${manager}/jobs.slice/jobs.service" "--slice-inherit --user -q true" --user -q true
says "${manager}/agents.slice/kitty-1-3.scope" "--slice-inherit --user --scope true" \
  --user --scope true
says "${manager}/app.slice/kitty-1-0.scope" "--user -u run-u1 true" --user -u run-u1 true
says "${manager}/session.slice/shell.service" "--user true" --user true
says "${manager}/jobs.slice/jobs.service" "--user --slice=a.slice true" --user --slice=a.slice true
says "${manager}/jobs.slice/jobs.service" "-q true" -q true
