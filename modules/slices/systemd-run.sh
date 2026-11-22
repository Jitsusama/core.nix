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
# A call from a unit sitting in jobs.slice itself goes in jobs-work.slice
# instead. pi writes each command it starts into jobs.service, which holds
# the efficient cores, but the slice it would inherit is jobs.slice, which
# holds none, since Joel's terminal work and timed work sit under it
# (efficient-work.nix). On 22 November 92 units agents started that way,
# and the 54 scopes hosts start their commands' spawners in, ran on every
# core, and 17 of 37 threads in five samples of the performance cores sat
# in jobs.slice. A call from a slice inside jobs keeps inheriting it.
#
# A slice named as a property, -p Slice=, is named too: systemd keeps the
# last Slice= a unit is given, so an added --slice-inherit put the units
# of a night's runs back in agents.slice, made for jobs.slice that way.
#
# Its help takes the shape every command of ours does (docs/commands.md), by
# hand. It answers --help alone; -h, or --help with anything beside it,
# reaches systemd's own, since that's where its flags are.
real="@real@"
if [[ $# -eq 1 && $1 == --help ]]; then
  cat << 'EOF'
systemd-run  run a unit, an agent's kept in its own slice

  A --user call naming no slice, from anywhere but
  app.slice or session.slice, runs in its caller's slice
  rather than among the apps, or in jobs-work.slice when
  its caller sits in jobs.slice itself. Every other call
  reaches systemd's own systemd-run unchanged.

EXAMPLES
  systemd-run --user --scope true  in the caller's slice
  systemd-run -h                   systemd's own help

EXIT
  what systemd's systemd-run exits
EOF
  exit 0
fi
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
  *:*/user@*.service/jobs.slice/*.slice/*) exec "${real}" --slice-inherit "$@" ;;
  *:*/user@*.service/jobs.slice/*) exec "${real}" --slice=jobs-work.slice "$@" ;;
  *:*/user@*.service/*) exec "${real}" --slice-inherit "$@" ;;
  *) exec "${real}" "$@" ;;
esac
