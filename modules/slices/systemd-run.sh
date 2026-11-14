# shellcheck shell=bash
# Stands in front of systemd-run, so a unit an agent asks the account's
# systemd for runs in the agent's own slice rather than among Joel's apps.
#
# The account's manager puts a unit started without a slice in app.slice,
# at an app's weight and memory floor, whoever asked. On 11 November at
# 00:06 an agent's check chain, started that way from its shell in
# jobs.slice, ran at 248% of a processor beside Joel's programs. So a
# --user call from anywhere but app.slice or session.slice, naming no
# slice of its own, is given --slice-inherit: the unit goes in the slice
# its caller runs in. Calls from Joel's own programs, the system manager's
# and those naming a slice or another machine pass through unchanged.
#
# A slice named as a property, -p Slice=, is named too: systemd keeps the
# last Slice= a unit is given, so an added --slice-inherit put the units
# of a night's runs back in agents.slice, made for jobs.slice that way.
real="@real@"
user=false
before=""
for arg in "$@"; do
  if [[ "${before}" == -p || "${before}" == --property ]] && [[ "${arg}" == Slice=* ]]; then
    exec "${real}" "$@"
  fi
  case "${arg}" in
    --slice | --slice=* | --slice-inherit | -pSlice=* | --property=Slice=* | -M* | --machine* | \
      -H* | --host*)
      exec "${real}" "$@"
      ;;
    --user) user=true ;;
    *) ;;
  esac
  before=${arg}
done
cgroup=""
read -r cgroup <"@cgroup@" || true
case "${user}:${cgroup}" in
  false:* | *:*/app.slice | *:*/app.slice/* | *:*/session.slice | *:*/session.slice/*)
    exec "${real}" "$@"
    ;;
  *:*/user@*.service/*) exec "${real}" --slice-inherit "$@" ;;
  *) exec "${real}" "$@" ;;
esac
