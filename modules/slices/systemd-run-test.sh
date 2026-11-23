# shellcheck shell=bash
# Proves systemd-run.sh adds --slice-inherit for an agent's --user call
# alone, and jobs-work.slice for one from agents' own places: jobs.service
# and the spawner scopes directly inside jobs, and pi's host and
# conversation units in agents.slice, reading a call's options as
# systemd-run's parser does by the table it's built from, which is held
# to the wrapped systemd's own as it's built, and that the hold refuses a
# table short of an option. Usage:
# systemd-run-test <wrapper> <options table> <the hold> <systemd's systemd-run>,
# where the wrapper was built in front of a stand-in that echoes what it's
# given and reads its caller's cgroup from ./cgroup.
set -euo pipefail
run=$1
table=$2
hold=$3
systemd_run=$4
rows=$(grep -v '^#' "${table}")
manager=0::/user.slice/user-1000.slice/user@1000.service

# The hold passes the table and refuses one short of its last option.
bash "${hold}" "${table}" "${systemd_run}" >/dev/null
sed '$d' "${table}" >short
if bash "${hold}" short "${systemd_run}" >/dev/null; then
  echo "the hold passed a table short of an option"
  exit 1
fi

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
# So does one from pi's own units in agents.slice, a host's or a
# conversation's: pi starts a host's spawner scope with systemd-run and
# writes it into jobs.service only once it's running, so the wrapper reads
# the unit pi runs in.
for place in jobs.slice/jobs.service jobs.slice/jobs.service/sub \
  jobs.slice/jobs.service/x.slice/y jobs.slice/pi-spawner-9-a1.scope \
  jobs.slice/pi-spawner-9-a1.scope/sub agents.slice/pi-host-work-attended.service \
  agents.slice/pi-host-work-attended.service/sub \
  agents.slice/pi-conversation-work-background@01a1f76b-7f2d.service; do
  says "${manager}/${place}" "--slice=jobs-work.slice --user --scope true" \
    --user --scope true
done
# An explicit --slice-inherit is the caller's own choice, and still inherits;
# so does a slice named, as the kitty windows a host opens name agents.slice.
host="${manager}/agents.slice/pi-host-work-attended.service"
says "${host}" "--user --slice-inherit true" --user --slice-inherit true
says "${host}" "--user --scope --slice=agents.slice kitty" \
  --user --scope --slice=agents.slice kitty
# Beside pi's units in agents.slice, the rest keep inheriting: the windows'
# kitty, the node, the scopes it holds sessions in, a near name or another
# kind of unit, and a pi unit's name further down, inside another's scope.
for place in agents.slice/sessions-kitty.service agents.slice/mycelium.service \
  agents.slice/mycelium-pi-3f2a.scope agents.slice/pi-hostess.service \
  agents.slice/pi-host-a.scope agents.slice/kitty-1-3.scope/agents.slice/pi-host-a.service \
  background.slice/pi-host-down@x.service; do
  says "${manager}/${place}" "--slice-inherit --user --scope true" --user --scope true
done
# Only systemd-run's own options are read: the command's words after them,
# or after --, name no host and no manager, and an option's value is no
# option of systemd-run's.
says "${host}" "--slice=jobs-work.slice --user --scope curl -H X:y" \
  --user --scope curl -H X:y
says "${host}" "--slice=jobs-work.slice --user -- grep --slice=x" --user -- grep --slice=x
says "${manager}/agents.slice/kitty-1-3.scope" "--slice-inherit --user ssh -M h" \
  --user ssh -M h
says "${host}" "--scope pip install --user x" --scope pip install --user x
says "${host}" "--slice=jobs-work.slice --user --description --host true" \
  --user --description --host true
says "${host}" "--slice=jobs-work.slice --user -- -Hx" --user -- -Hx
says "${host}" "-u --user true" -u --user true
says "${host}" "--user -C c true" --user -C c true
# Each option in the table is read as taking a value or not, as systemd-run
# does: a word after a valued one is its value, whatever it looks like, and
# one after a bare one is read for itself. Those naming a slice or another
# manager pass the call through.
while read -r letter name; do
  passes=false
  case "${name%=}" in
    slice | slice-inherit | host | machine | capsule | help) passes=true ;;
    *) ;;
  esac
  for option in "--${name%=}" "-${letter}"; do
    [[ "${option}" == -- ]] && continue
    if [[ "${passes}" == true ]]; then
      if [[ "${name}" != help ]]; then
        says "${host}" "--user ${option} v true" --user "${option}" v true
      fi
    elif [[ "${name}" == *= ]]; then
      says "${host}" "${option} --user true" "${option}" --user true
      says "${host}" "--slice=jobs-work.slice ${option} v --user true" \
        "${option}" v --user true
    elif [[ "${name}" != system ]]; then
      says "${host}" "--slice=jobs-work.slice ${option} --user true" \
        "${option}" --user true
    fi
  done
done <<<"${rows}"
# Short options come bundled, the last letter's value the next word or the
# rest of it, and a long name may be cut short while it's still the only
# one it could be. A name two could be, or none, and a letter none has,
# are refused by systemd-run itself, and read as taking no value, so the
# call keeps its slice; so does the hidden --tty, which systemd-run takes.
# The last of --user and --system is obeyed, and a valued option's value
# may be --, which ends nothing.
kitty="${manager}/agents.slice/kitty-1-3.scope"
says "${kitty}" "--user -qu j --slice=a.slice true" --user -qu j --slice=a.slice true
says "${host}" "--slice=jobs-work.slice -qu job --user --scope true" \
  -qu job --user --scope true
says "${host}" "--user -tM h true" --user -tM h true
says "${kitty}" "--user -qpSlice=a.slice true" --user -qpSlice=a.slice true
says "${kitty}" "--user -qp Slice=a.slice true" --user -qp Slice=a.slice true
says "${kitty}" "--slice-inherit --user -qpNice=5 true" --user -qpNice=5 true
says "${host}" "--slice=jobs-work.slice -qpNice=5 --user true" -qpNice=5 --user true
says "${host}" "--slice=jobs-work.slice -uHx --user true" -uHx --user true
says "${host}" "--slice=jobs-work.slice --us true" --us true
says "${host}" "--slice=jobs-work.slice --desc x --user true" --desc x --user true
says "${host}" "--user --slice-inh true" --user --slice-inh true
says "${host}" "--user --prop Slice=a.slice true" --user --prop Slice=a.slice true
says "${host}" "--user --prop=Slice=a.slice true" --user --prop=Slice=a.slice true
says "${host}" "--slice=jobs-work.slice --user --prop Nice=5 --scope true" \
  --user --prop Nice=5 --scope true
says "${host}" "--slice=jobs-work.slice --user --sl true" --user --sl true
says "${host}" "--slice=jobs-work.slice --user --frobnicate a.slice true" \
  --user --frobnicate a.slice true
says "${host}" "--slice=jobs-work.slice --user -Z true" --user -Z true
says "${host}" "--slice=jobs-work.slice --user --tty true" --user --tty true
says "${host}" "--user --system true" --user --system true
says "${host}" "--slice=jobs-work.slice --system --user true" --system --user true
says "${host}" "--slice=jobs-work.slice -u -- --user true" -u -- --user true
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
