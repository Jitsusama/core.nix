# shellcheck shell=bash
# One round of a Mac's name lookups, run each minute: each name is looked up
# through the system's resolver, as every program's lookup goes, and through
# each server named, and mDNSResponder's share of a core is read. A lookup
# that fails or is over the slow mark two rounds running, or is either three
# rounds running, or mDNSResponder past half a core five rounds running, is
# said as an error, which the unified log keeps and health counts, every
# round it holds; one well again is said once as a notice. A Mac waking or
# changing networks fails a round and answers the next cold, so that pair
# is never said, while a storm that fails and lags by turns is. Each line opens
# with name-lookups, since health names a Mac's line by the program that
# wrote it, which is logger. Each round's times are kept in the state
# folder's last file.
# usage: lookups.sh <state folder> <slow, in ms> <names, by commas> [servers, by commas]
set -euo pipefail

# A round that ends early says so as an error too, since quiet means well
# and bash's own words go only to the job's log, which health never reads.
# It may end before any name is looked up or after every one, at the save,
# so it says neither. A round launchd stops, as at a switch or a logout,
# isn't said: bash runs this with the status of the last command before the
# signal, and every command the round runs between its probes ends 0, so
# only a TERM landing the moment a probe ends could be said.
ended() {
  local ended=$1
  if [[ ${ended} -ne 0 ]]; then
    logger -p user.err -t name-lookups "name-lookups: a round ended early (exit ${ended})," \
      "before it had said and kept all it found; the job's log says why" || true
  fi
}
trap 'ended "$?"' EXIT

state=$1
slow=$2
IFS=, read -r -a names <<<"$3"
IFS=, read -r -a servers <<<"${4-}"

# How long one lookup may take before it counts as failed, in seconds: under
# the minute between rounds, and past the 17 s the worst storm took. A probe
# that outlives timeout's TERM is killed this many seconds on, so a round
# can't hang on one.
within=20
killed=5
# mDNSResponder's share of a core past which it's busy, in percent, and the
# rounds running it may be.
busy=50
rounds=5
mkdir -p "${state}"

# What each check was last round: how many rounds running it was its kind
# of wrong, whether it was red, which kind it was, and how many rounds
# running it was wrong of any kind, kept as
# "<check> <rounds> <red> <outcome> <wrong rounds>". A count that isn't a
# number is read as none, since bash's arithmetic would run what it holds,
# and so is a zero before other digits, as in 09, which it reads as octal
# and refuses past 7: in a round where that check is wrong, the refusal
# drops it and the names after it, though mDNSResponder is still judged and
# the round saves. A line naming no check is passed over, since bash
# refuses an empty key and the round would end before it kept anything.
declare -A running=() red=() kind=() bad=()
if [[ -r "${state}/running" ]]; then
  while read -r check count was outcome any; do
    [[ -n ${check} ]] || continue
    [[ ${count} =~ ^(0|[1-9][0-9]*)$ ]] || count=0
    [[ ${any-} =~ ^(0|[1-9][0-9]*)$ ]] || any=${count}
    running[${check}]=${count}
    red[${check}]=${was-no}
    kind[${check}]=${outcome-}
    bad[${check}]=${any}
  done <"${state}/running"
fi
declare -A now=()
last=()

# Keep a check's round, failed, slow, busy or well, as its words say it; and
# say what's wrong, every round it holds, or that it's well again, once. Each
# kind keeps its own count, so a failure then one slow round reads as one
# slow round, not two; wrong rounds of any kind are counted too, so a storm
# that fails and lags by turns is said from its third.
judge() {
  local check=$1 outcome=$2 words=$3
  local count=0 any=0
  case ${outcome} in
    failed | slow | busy)
      count=1
      if [[ ${kind[${check}]-} = "${outcome}" ]]; then
        count=$((${running[${check}]-0} + 1))
      fi
      any=$((${bad[${check}]-0} + 1))
      ;;
    *) ;;
  esac
  local wrong=no
  case ${outcome} in
    failed)
      words="${words}, ${count} rounds running"
      if [[ ${count} -ge 2 || ${any} -ge 3 ]]; then wrong=yes; fi
      ;;
    slow)
      words="${words}, past ${slow} ms ${count} rounds running"
      if [[ ${count} -ge 2 || ${any} -ge 3 ]]; then wrong=yes; fi
      ;;
    busy)
      words="${words}, past ${busy}% ${count} rounds running"
      if [[ ${count} -ge ${rounds} ]]; then wrong=yes; fi
      ;;
    *) ;;
  esac
  local at
  at=$(date '+%F %T')
  if [[ ${wrong} = yes && ${count} -lt ${any} ]]; then
    words="${words}, wrong ${any} rounds running"
  fi
  if [[ ${wrong} = yes ]]; then
    logger -p user.err -t name-lookups "name-lookups: ${words}"
    echo "${at} ${words}" >&2
  elif [[ ${red[${check}]-no} = yes && ${count} -eq 0 ]]; then
    logger -p user.notice -t name-lookups "name-lookups: ${check} is well again: ${words}"
    echo "${at} ${check} is well again: ${words}" >&2
  fi
  now[${check}]="${count} ${wrong} ${outcome} ${any}"
}

# Milliseconds since $1, a moment as EPOCHREALTIME gives it.
since() {
  local started=${1/./} ended=${EPOCHREALTIME/./}
  echo $(((ended - started) / 1000))
}

# What a probe's exit, $1, says of probe $2 when it gave no address, as
# `why`: timeout's 124 is no answer in time, as is 137, a probe it killed
# once it outlived its TERM, and 125 to 127 a program that couldn't run,
# which is no lookup at all, so it's never said as the name failing. dig
# ends 9 when the server never replied, within its own five seconds, and 10
# on an error of its own. Any other end but 0 is said as it is, an address
# printed beside it included.
why() {
  case $2:$1 in
    *:0) why="" ;;
    *:124) why="no answer within ${within} s" ;;
    *:137) why="no answer within ${within} s, killed ${killed} s on" ;;
    *:125 | *:126 | *:127) why="timeout or $2 couldn't run (exit $1), so this wasn't a lookup" ;;
    dig:9) why="no reply from the server" ;;
    dig:10) why="dig failed on its own (exit 10), so this wasn't a lookup" ;;
    *) why="$2 ended $1" ;;
  esac
}

# Whether a lookup's time is past the slow mark.
outcome() {
  if [[ $1 -gt ${slow} ]]; then
    echo slow
  else
    echo well
  fi
}

for name in ${names[@]+"${names[@]}"}; do
  started=${EPOCHREALTIME}
  # dscacheutil asks the system's resolver, as getaddrinfo does, and prints
  # an address line for each address, or nothing for a name it can't find.
  ran=0
  answer=$(timeout -k "${killed}" "${within}" dscacheutil -q host -a name "${name}" \
    2>/dev/null) || ran=$?
  took=$(since "${started}")
  check="system:${name}"
  if [[ ${ran} -eq 0 ]] && grep -qE '^ipv?6?_address:' <<<"${answer}"; then
    result=$(outcome "${took}")
    judge "${check}" "${result}" "the system looked up ${name} in ${took} ms"
  else
    why "${ran}" dscacheutil
    judge "${check}" failed "the system couldn't look up ${name} (${took} ms${why:+, ${why}})"
  fi
  last+=("${check} ${took}")
  for server in ${servers[@]+"${servers[@]}"}; do
    started=${EPOCHREALTIME}
    ran=0
    answer=$(timeout -k "${killed}" "${within}" dig +tries=1 +time=5 +noall +answer \
      "@${server}" "${name}" A 2>/dev/null) || ran=$?
    took=$(since "${started}")
    check="${server}:${name}"
    # An IPv4 address record, not only a CNAME pointing at a name that has
    # none; a server is asked for A alone, so one timing a name is one
    # answer.
    if [[ ${ran} -eq 0 ]] && grep -qE '[[:space:]]IN[[:space:]]+A[[:space:]]' <<<"${answer}"; then
      result=$(outcome "${took}")
      judge "${check}" "${result}" "${server} answered ${name} in ${took} ms"
    else
      why "${ran}" dig
      judge "${check}" failed \
        "${server} gave no IPv4 address for ${name} (${took} ms${why:+, ${why}})"
    fi
    last+=("${check} ${took}")
  done
done

# mDNSResponder's share of a core, as ps reads it, by its path, or by its
# own name alone, as ps may give another account's process; a ps that fails
# reads as none rather than ending the round before it's kept.
share=$(ps -A -o %cpu= -o comm= \
  | awk '$2 == "/usr/sbin/mDNSResponder" || $2 == "mDNSResponder" || $2 == "(mDNSResponder)" {
      share += $1
    } END { print int(share) }') \
  || share=0
if [[ ${share} -gt ${busy} ]]; then
  judge mDNSResponder busy "mDNSResponder uses ${share}% of a core"
else
  judge mDNSResponder well "mDNSResponder uses ${share}% of a core"
fi
last+=("mDNSResponder ${share}%")

for check in "${!now[@]}"; do
  echo "${check} ${now[${check}]}"
done >"${state}/running"
at=$(date '+%F %T')
printf '%s\n' "${at}" "${last[@]}" >"${state}/last"
