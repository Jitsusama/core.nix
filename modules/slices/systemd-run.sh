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
# A call from agents' own places in jobs.slice goes in jobs-work.slice
# instead: jobs.service, and the pi-spawner scopes, with anything below
# them. pi writes each process it starts into jobs.service, which holds the
# efficient cores, but the slice it would inherit is jobs.slice, which holds
# none, since Joel's terminal work and timed work sit under it
# (efficient-work.nix). On 22 November 92 units agents started that way,
# and 54 spawner scopes, ran on every core, and 17 of 37 threads in five
# samples of the performance cores sat in jobs.slice. A call from a slice
# inside jobs, or from one of Joel's own units in jobs.slice, sccache's
# and load-proof's, keeps inheriting. The place is read from just below
# the manager, so a path only naming jobs.slice further down, inside
# another's scope, is no agents' place.
#
# pi's own units in agents.slice, its hosts' and conversations', are
# agents' places too. A host starts each spawner's scope with systemd-run,
# naming no slice, and pi writes that systemd-run into jobs.service only
# once it's running, off its own thread, 5 to 7 ms a move (the harness's
# priority/place.ts). So where a scope landed was a race: of the 16
# spawner scopes started in this wrapper's first twelve minutes live, on
# 22 November, 11 reached jobs-work.slice, 4 jobs.slice, on every core,
# and 1 agents.slice, as the move came before this script read its
# cgroup, between that and systemd's answer, or after both. Read from
# pi's unit or from jobs.service, a call now names jobs-work.slice either
# way. The kitty windows pi opens name agents.slice, so they pass through
# unchanged.
#
# In jobs-work.slice such a unit is judged as agents' work everywhere else
# too: polkit's agents-ask-first.rules refuses it rather than asking Joel,
# the sorter clears its real-time scheduling and ranks it first for the
# OOM killer, and it sits beside any tiers' slices at the default weight,
# outside their order. That weight counts once for each unit, so 146 such
# units weigh 14,600 beside tiers weighing 620 together; before, they sat
# beside jobs-work.slice itself, at 100, tiers and all, so ranked work's
# share of CPU against them grows. An explicit --slice-inherit is the
# caller's own choice, and still inherits.
#
# A slice named as a property, -p Slice=, is named too: systemd keeps the
# last Slice= a unit is given, so an added --slice-inherit put the units
# of a night's runs back in agents.slice, made for jobs.slice that way.
#
# Only systemd-run's own options are read, as its parser reads them: up to
# `--` or the command, each option's value skipped. The words after belong
# to the command, so `curl -H` or `grep -H` there is no other host, and a
# command's own `--user`, as in `pip install --user`, is no user manager's
# call. Read as options, the first passed such a call through unsliced,
# to land among Joel's apps, and the second gave a system call a --slice
# from jobs-work.slice. Short options come bundled, as `-qu NAME`, and a
# long one may be cut short while only one name begins so, as `--desc`;
# read as whole words alone, `-qu job --user` lost its --user, and
# `-qu j --slice=x` lost its slice and was given --slice-inherit, which
# systemd joins with it into a slice of its own. Which options there are,
# and which take a value, is the wrapped systemd's word: its parser lists
# every one in --introspect-cli, hidden ones too, as the deprecated --tty,
# and the wrapper is built only once systemd-run-options.txt holds to that
# list (systemd-run-options-hold.sh), so a machine whose systemd takes
# another fails its build rather than misreading calls. A name no option
# has, or two begin, and a letter no option has, are read as an option
# taking no value, and the reading goes on. systemd-run refuses such a
# call whatever is added to it, and one a table lacked would still keep
# the agent's slice, where passed through unchanged, the --tty that --help
# leaves out would put an agent's unit among Joel's apps.
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
  its caller is an agent's, in jobs.service, a pi-spawner
  scope or a pi host's or conversation's unit. Every other
  call reaches systemd's own systemd-run unchanged.

EXAMPLES
  systemd-run --user --scope true  in the caller's slice, or
                                   jobs-work.slice from an
                                   agent's place
  systemd-run -h                   systemd's own help

EXIT
  what systemd's systemd-run exits
EOF
  exit 0
fi
# Each short letter with its long name, and every long name, a valued one
# ending in =, from systemd-run-options.txt, the wrapped systemd's own.
shorts=(@shorts@)
longs=(@longs@)

# The long name short letter $1 stands for, = and all. None, for a letter
# no option has.
short() {
  local pair
  for pair in "${shorts[@]}"; do
    if [[ "${pair%%:*}" == "$1" ]]; then
      echo "${pair#*:}"
      return
    fi
  done
}

# The long name `--$1` stands for, = and all, as systemd's own parser
# reads it (src/shared/options.c): the name itself, else the only one it
# begins. None, for one it can't be, or one two could be.
long() {
  local option found=""
  for option in "${longs[@]}"; do
    if [[ "${option%=}" == "$1" ]]; then
      echo "${option}"
      return
    fi
  done
  for option in "${longs[@]}"; do
    if [[ "${option}" == "$1"* ]]; then
      if [[ -n "${found}" ]]; then
        return
      fi
      found=${option}
    fi
  done
  echo "${found}"
}

# Whether the call names a slice of its own or another manager, read from
# the option and the value it was given.
names_its_place() {
  case "$1" in
    slice | slice-inherit | host | machine | capsule) return 0 ;;
    property) [[ "$2" == Slice=* ]] ;;
    *) return 1 ;;
  esac
}

user=false
# The option whose value the next word is, when one is owed.
owed=""
for arg in "$@"; do
  if [[ -n "${owed}" ]]; then
    names_its_place "${owed}" "${arg}" && exec "${real}" "$@"
    owed=""
    continue
  fi
  case "${arg}" in
    --) break ;;
    --*)
      word=${arg#--}
      name=$(long "${word%%=*}")
      if [[ "${word}" == *=* ]]; then
        names_its_place "${name%=}" "${word#*=}" && exec "${real}" "$@"
      elif names_its_place "${name%=}" ""; then
        exec "${real}" "$@"
      elif [[ "${name}" == *= ]]; then
        owed=${name%=}
      fi
      case "${name}" in
        user) user=true ;;
        system) user=false ;;
        *) ;;
      esac
      ;;
    -?*)
      letters=${arg#-}
      while [[ -n "${letters}" ]]; do
        name=$(short "${letters:0:1}")
        letters=${letters:1}
        # A valued letter takes the rest of the word as its value, if any.
        if [[ "${name}" == *= && -n "${letters}" ]]; then
          names_its_place "${name%=}" "${letters}" && exec "${real}" "$@"
          break
        fi
        names_its_place "${name%=}" "" && exec "${real}" "$@"
        if [[ "${name}" == *= ]]; then
          owed=${name%=}
        fi
      done
      ;;
    *) break ;;
  esac
done
cgroup=""
read -r cgroup <"@cgroup@" || true
agents='^0::/user\.slice/user-[0-9]+\.slice/user@[0-9]+\.service/'
agents+='(jobs\.slice/(jobs\.service|pi-spawner-[^/]+\.scope)'
agents+='|agents\.slice/pi-(host|conversation)-[^/]+\.service)(/|$)'
if [[ "${user}" == true && "${cgroup}" =~ ${agents} ]]; then
  exec "${real}" --slice=jobs-work.slice "$@"
fi
case "${user}:${cgroup}" in
  false:* | *:*/app.slice | *:*/app.slice/* | *:*/session.slice | *:*/session.slice/*)
    exec "${real}" "$@"
    ;;
  *:*/user@*.service/*) exec "${real}" --slice-inherit "$@" ;;
  *) exec "${real}" "$@" ;;
esac
