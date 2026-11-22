# shellcheck shell=bash
# Proves systemd-run.sh adds --slice-inherit for an agent's --user call
# alone, and jobs-work.slice for one from directly inside jobs. Usage:
# systemd-run-test <wrapper>, where the wrapper was built in front of a
# stand-in that echoes what it's given and reads its caller's cgroup from
# ./cgroup.
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

# A call from agents' own places in jobs, where pi writes its commands and
# a host makes its spawners' scopes, and anything below them, goes in
# jobs-work.slice: jobs.slice holds no cores, so the slice it would
# inherit lets the unit onto every one.
for place in jobs.service jobs.service/sub jobs.service/x.slice/y pi-spawner-9-a1.scope \
  pi-spawner-9-a1.scope/sub; do
  says "${manager}/jobs.slice/${place}" "--slice=jobs-work.slice --user --scope true" \
    --user --scope true
  # An explicit --slice-inherit is the caller's own choice, and still inherits.
  says "${manager}/jobs.slice/${place}" "--user --slice-inherit true" --user --slice-inherit true
done
# One from a slice inside jobs keeps that slice: Joel's terminal work and
# timed work stay free, and pi's sorted work stays where it was sorted. So
# do Joel's own services sitting in jobs.slice, and a place that only names
# jobs.slice further down, inside another's scope.
for place in jobs.slice/jobs-awaited.slice/awaited.service \
  jobs.slice/jobs-timed.slice/timed.service \
  jobs.slice/jobs-work.slice/jobs-work-unranked.slice/work-1.scope \
  jobs.slice/jobs-work.slice/work-1.scope jobs.slice/sccache.service \
  agents.slice/kitty-1-3.scope/a.service/jobs.slice/jobs.service; do
  says "${manager}/${place}" "--slice-inherit --user true" --user true
done
says "${manager}/jobs.slice/jobs.service" "--user -p Slice=a.slice true" \
  --user -p Slice=a.slice true
says "${manager}/agents.slice/kitty-1-3.scope" "--slice-inherit --user --scope true" \
  --user --scope true
says "${manager}/app.slice/kitty-1-0.scope" "--user -u run-u1 true" --user -u run-u1 true
says "${manager}/session.slice/shell.service" "--user true" --user true
says "${manager}/jobs.slice/jobs.service" "--user --slice=a.slice true" --user --slice=a.slice true
says "${manager}/jobs.slice/jobs.service" "-q true" -q true
# A slice named as a property is named too: systemd keeps the last Slice=,
# so an added --slice-inherit would put the unit back in its caller's.
for named in "-p Slice=jobs.slice" "-pSlice=jobs.slice" "--property=Slice=jobs.slice" \
  "--property Slice=jobs.slice"; do
  # shellcheck disable=SC2086 # split on purpose, into the call's own words.
  says "${manager}/agents.slice/kitty-1-3.scope" "--user ${named} true" --user ${named} true
done
# --help alone is its own help, never the real one's; -h, and --help with
# another word, are systemd's.
help=$("${run}" --help)
if [[ "${help}" != "systemd-run  "* ]]; then
  echo "--help said '${help%%$'\n'*}', not its own help"
  exit 1
fi
says "${manager}/jobs.slice/jobs.service" "-h" -h
says "${manager}/jobs.slice/jobs.service" "--help --no-pager" --help --no-pager
# Another property is no slice.
says "${manager}/agents.slice/kitty-1-3.scope" "--slice-inherit --user -p Nice=5 true" \
  --user -p Nice=5 true
