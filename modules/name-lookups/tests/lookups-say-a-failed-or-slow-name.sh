# shellcheck shell=bash
# Checks that a round of the Mac's name lookups says nothing while every name
# answers quickly, says each failed lookup at once, a slow one only from its
# second round running, a busy mDNSResponder only from its fifth, and a check
# well again once. dscacheutil, dig, ps and logger are stand-ins answering as
# the test sets, and the slow mark is set to 0 ms rather than a lookup made
# to wait, so no round sleeps.
# usage: lookups-say-a-failed-or-slow-name.sh <lookups.sh>
set -euo pipefail

lookups=$1
work=$(mktemp -d)
trap 'rm -rf "${work}"' EXIT
stand=${work}/stand
state=${work}/state
said=${work}/said
mkdir -p "${stand}" "${work}/answers"

# Each stand-in answers from the answers folder: a name's file holds "fails"
# for a name it can't find, and mDNSResponder's its share of a core.
cat >"${stand}/dscacheutil" <<EOF
#!${BASH}
[ "\$(cat "${work}/answers/\$5" 2>/dev/null)" = fails ] && exit 0
printf 'name: %s\nip_address: 192.0.2.1\n\n' "\$5"
EOF
cat >"${stand}/dig" <<EOF
#!${BASH}
server=\${5#@}
[ "\$(cat "${work}/answers/\${server}-\$6" 2>/dev/null)" = fails ] && exit 9
printf '%s.\t60\tIN\tA\t192.0.2.1\n' "\$6"
EOF
cat >"${stand}/ps" <<EOF
#!${BASH}
printf '  %s /usr/sbin/mDNSResponder\n  3.0 /usr/libexec/other\n' \
  "\$(cat "${work}/answers/mDNSResponder" 2>/dev/null || echo 0.1)"
EOF
cat >"${stand}/logger" <<EOF
#!${BASH}
echo "\$*" >>"${said}"
EOF
chmod +x "${stand}"/*
export PATH=${stand}:${PATH}

# One round, at the slow mark given, of example.org and example.net, the
# second also asked of 192.0.2.53; what it said, in $said, is read afresh.
round() {
  : >"${said}"
  bash "${lookups}" "${state}" "$1" example.org,example.net 192.0.2.53 2>/dev/null
}

# Whether the round said nothing, or said a line holding each of the words.
quiet() {
  if [[ -s ${said} ]]; then
    echo "$1: $(<"${said}")" >&2
    exit 1
  fi
}
says() {
  local what=$1
  shift
  local words
  for words in "$@"; do
    if ! grep -qF -- "${words}" "${said}"; then
      echo "${what}: no line says \"${words}\": $(<"${said}")" >&2
      exit 1
    fi
  done
}

round 5000
quiet "a round with every name answering quickly said something"
for check in system:example.org system:example.net 192.0.2.53:example.net mDNSResponder; do
  if ! grep -q "^${check} " "${state}/last"; then
    echo "a round didn't keep ${check}'s time: $(<"${state}/last")" >&2
    exit 1
  fi
done

echo fails >"${work}/answers/example.net"
round 5000
says "a failed lookup" "-p user.err" "the system couldn't look up example.net"
round 5000
says "a lookup still failing" "couldn't look up example.net"
rm "${work}/answers/example.net"
round 5000
says "a lookup well again" "-p user.notice" "system:example.net is well again"
round 5000
quiet "a lookup well for two rounds said something"

echo fails >"${work}/answers/192.0.2.53-example.net"
round 5000
says "a server giving no address" "-p user.err" "192.0.2.53 gave no address for example.net"
rm "${work}/answers/192.0.2.53-example.net"
round 5000

round 0
quiet "a lookup slow for one round said something"
round 0
says "a lookup slow two rounds running" "-p user.err" \
  "the system looked up example.org in" "past 0 ms 2 rounds running" \
  "192.0.2.53 answered example.net in"
round 5000
says "a slow lookup quick again" "system:example.org is well again"

echo 80.0 >"${work}/answers/mDNSResponder"
for count in 1 2 3 4; do
  round 5000
  quiet "mDNSResponder busy ${count} rounds running was said"
done
round 5000
says "mDNSResponder busy five rounds running" "-p user.err" \
  "mDNSResponder uses 80% of a core, past 50% 5 rounds running"
echo 20.0 >"${work}/answers/mDNSResponder"
round 5000
says "mDNSResponder quiet again" "mDNSResponder is well again"
