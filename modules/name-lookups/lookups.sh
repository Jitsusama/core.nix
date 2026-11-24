# shellcheck shell=bash
# One round of a Mac's name lookups, run each minute: each name is looked up
# through the system's resolver, as every program's lookup goes, and through
# each server named, and mDNSResponder's share of a core is read. A lookup
# that fails, one over the slow mark two rounds running, or mDNSResponder past
# half a core five rounds running is said as an error, which the unified log
# keeps and health counts, every round it holds; one well again is said once
# as a notice. Each round's times are kept in the state folder's last file.
# usage: lookups.sh <state folder> <slow, in ms> <names, by commas> [servers, by commas]
set -euo pipefail

state=$1
slow=$2
IFS=, read -r -a names <<<"$3"
IFS=, read -r -a servers <<<"${4-}"

# How long one lookup may take before it counts as failed, in seconds: under
# the minute between rounds, and past the 17 s the worst storm took.
within=20
# mDNSResponder's share of a core past which it's busy, in percent, and the
# rounds running it may be.
busy=50
rounds=5
mkdir -p "${state}"

# What each check was last round: how many rounds running it was slow or
# busy, and whether it was red, kept as "<check> <rounds> <red>".
declare -A running=() red=()
if [[ -r "${state}/running" ]]; then
  while read -r check count was; do
    running[${check}]=${count}
    red[${check}]=${was}
  done <"${state}/running"
fi
declare -A now=()
last=()

# Keep a check's round, failed, slow, busy or well, as its words say it; and
# say what's wrong, every round it holds, or that it's well again, once.
judge() {
  local check=$1 outcome=$2 words=$3
  local count=${running[${check}]-0}
  case ${outcome} in
    failed | slow | busy) count=$((count + 1)) ;;
    *) count=0 ;;
  esac
  local wrong=no
  case ${outcome} in
    failed) wrong=yes ;;
    slow)
      words="${words}, past ${slow} ms ${count} rounds running"
      [[ ${count} -ge 2 ]] && wrong=yes
      ;;
    busy)
      words="${words}, past ${busy}% ${count} rounds running"
      [[ ${count} -ge ${rounds} ]] && wrong=yes
      ;;
    *) ;;
  esac
  local at
  at=$(date '+%F %T')
  if [[ ${wrong} = yes ]]; then
    logger -p user.err -t name-lookups "${words}"
    echo "${at} ${words}" >&2
  elif [[ ${red[${check}]-no} = yes && ${count} -eq 0 ]]; then
    logger -p user.notice -t name-lookups "${check} is well again: ${words}"
    echo "${at} ${check} is well again: ${words}" >&2
  fi
  now[${check}]="${count} ${wrong}"
}

# Milliseconds since $1, a moment as EPOCHREALTIME gives it.
since() {
  local started=${1/./} ended=${EPOCHREALTIME/./}
  echo $(((ended - started) / 1000))
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
  answer=$(timeout "${within}" dscacheutil -q host -a name "${name}" 2>/dev/null) || answer=""
  took=$(since "${started}")
  check="system:${name}"
  if grep -qE '^ipv?6?_address:' <<<"${answer}"; then
    result=$(outcome "${took}")
    judge "${check}" "${result}" "the system looked up ${name} in ${took} ms"
  else
    judge "${check}" failed "the system couldn't look up ${name} (${took} ms)"
  fi
  last+=("${check} ${took}")
  for server in ${servers[@]+"${servers[@]}"}; do
    started=${EPOCHREALTIME}
    answer=$(timeout "${within}" dig +tries=1 +time=5 +noall +answer "@${server}" "${name}" A \
      2>/dev/null) || answer=""
    took=$(since "${started}")
    check="${server}:${name}"
    if grep -qE '^[^;[:space:]]' <<<"${answer}"; then
      result=$(outcome "${took}")
      judge "${check}" "${result}" "${server} answered ${name} in ${took} ms"
    else
      judge "${check}" failed "${server} gave no address for ${name} (${took} ms)"
    fi
    last+=("${check} ${took}")
  done
done

# mDNSResponder's share of a core, as ps reads it.
share=$(ps -A -o %cpu= -o comm= \
  | awk '$2 ~ /mDNSResponder$/ { share += $1 } END { print int(share) }')
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
