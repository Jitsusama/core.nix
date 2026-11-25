# shellcheck shell=bash
# Checks that a round of the Mac's name lookups says nothing while every name
# answers quickly, says a failed or slow lookup only from its second round
# running, each kind counted apart, and one that fails and lags by turns
# from its third, a busy mDNSResponder only from its fifth, by its path or
# its name, and a check well again once; a server answering only a CNAME,
# or failing, gave no address, and a state file with a blank line, a count
# that isn't a number or one opening with a zero is read past. dscacheutil, dig, ps and logger
# are stand-ins answering as the test sets, and the slow mark is set to 0 ms
# rather than a lookup made to wait, so no round sleeps.
# usage: lookups-say-a-name-that-fails-or-takes-long.sh <lookups.sh>
set -euo pipefail

lookups=$1
work=$(mktemp -d)
trap 'rm -rf "${work}"' EXIT
stand=${work}/stand
state=${work}/state
said=${work}/said
mkdir -p "${stand}" "${work}/answers"

# Each stand-in answers from the answers folder: a name's file holds "fails"
# for a name it can't find, or for a server "cname" for an alias whose name
# has no address, and mDNSResponder's its share of a core. dig fails as the
# real one does, saying why on stdout.
cat >"${stand}/dscacheutil" <<EOF
#!${BASH}
[ "\$(cat "${work}/answers/\$5" 2>/dev/null)" = fails ] && exit 0
printf 'name: %s\nip_address: 192.0.2.1\n\n' "\$5"
EOF
cat >"${stand}/dig" <<EOF
#!${BASH}
server=\${5#@}
case "\$(cat "${work}/answers/\${server}-\$6" 2>/dev/null)" in
  fails)
    printf ';; communications error to %s#53: timed out\n' "\${server}"
    exit 9
    ;;
  cname) printf '%s.\t60\tIN\tCNAME\tgone.invalid.\n' "\$6" ;;
  *) printf '%s.\t60\tIN\tA\t192.0.2.1\n' "\$6" ;;
esac
EOF
cat >"${stand}/ps" <<EOF
#!${BASH}
printf '  %s %s\n  3.0 /usr/libexec/other\n' \
  "\$(cat "${work}/answers/mDNSResponder" 2>/dev/null || echo 0.1)" \
  "\$(cat "${work}/answers/mDNSResponder-name" 2>/dev/null || echo /usr/sbin/mDNSResponder)"
EOF
cat >"${stand}/logger" <<EOF
#!${BASH}
echo "\$*" >>"${said}"
EOF
chmod +x "${stand}"/*
export PATH=${stand}:${PATH}

# One round, at the slow mark given, of example.org and example.net, each
# asked of the system and of 192.0.2.53; what it said, in $said, is read
# afresh.
round() {
  : >"${said}"
  bash "${lookups}" "${state}" "$1" example.org,example.net 192.0.2.53 2>/dev/null
}

# Whether the round said nothing, or said one line holding all the words.
quiet() {
  if [[ -s ${said} ]]; then
    echo "$1: $(<"${said}")" >&2
    exit 1
  fi
}
says() {
  local what=$1
  shift
  local line words holds
  while IFS= read -r line; do
    holds=yes
    for words in "$@"; do
      [[ ${line} == *"${words}"* ]] || holds=no
    done
    [[ ${holds} = yes ]] && return 0
  done <"${said}"
  echo "${what}: no line says all of $*: $(<"${said}")" >&2
  exit 1
}

round 5000
quiet "a round with every name answering quickly said something"
for check in system:example.org system:example.net 192.0.2.53:example.org \
  192.0.2.53:example.net mDNSResponder; do
  if ! grep -q "^${check} " "${state}/last"; then
    echo "a round didn't keep ${check}'s time: $(<"${state}/last")" >&2
    exit 1
  fi
done

echo fails >"${work}/answers/example.net"
round 5000
quiet "a lookup failing for one round, as at a wake, said something"
round 5000
says "a failed lookup" "-p user.err" "name-lookups: the system couldn't look up example.net" \
  "2 rounds running"
round 5000
says "a lookup still failing" "-p user.err" "couldn't look up example.net" "3 rounds running"
rm "${work}/answers/example.net"
round 5000
says "a lookup well again" "-p user.notice" "name-lookups: system:example.net is well again"
round 5000
quiet "a lookup well for two rounds said something"

for answer in fails cname; do
  echo "${answer}" >"${work}/answers/192.0.2.53-example.net"
  round 5000
  round 5000
  says "a server answering with ${answer}" "-p user.err" \
    "192.0.2.53 gave no address for example.net"
  rm "${work}/answers/192.0.2.53-example.net"
  round 5000
  says "a server answering again after ${answer}" "-p user.notice" \
    "192.0.2.53:example.net is well again"
done

# A failure, then one cold answer, as after a wake, is one slow round.
echo fails >"${work}/answers/example.org"
round 5000
rm "${work}/answers/example.org"
round 0
if grep -qF example.org "${said}"; then
  echo "a failure then one slow round was said: $(<"${said}")" >&2
  exit 1
fi
# A third wrong round, failing again, is a storm that fails and lags by
# turns.
echo fails >"${work}/answers/example.org"
round 5000
says "a lookup failing and lagging by turns" "-p user.err" \
  "couldn't look up example.org" "wrong 3 rounds running"
rm "${work}/answers/example.org"
round 5000
says "a lookup well after failing and lagging by turns" "-p user.notice" \
  "system:example.org is well again"
round 5000

# One slow round, then one failing, is two wrong rounds, still quiet.
round 0
echo fails >"${work}/answers/example.org"
round 5000
quiet "a slow round then a failing one said something"
rm "${work}/answers/example.org"
round 5000

# A blank line, or a count that isn't a number, in the state is read past:
# the round still runs and keeps its state, and counts the check failing
# again afresh rather than running what the count holds.
printf "\nsystem:example.net x[\$(:>%s/ran)] yes failed\n" "${work}" >"${state}/running"
echo fails >"${work}/answers/example.net"
round 5000
rm "${work}/answers/example.net"
if [[ -e ${work}/ran ]] || ! grep -q '^system:example.net 1 no failed 1$' "${state}/running"; then
  echo "a state file with a blank line or a word for a count wasn't read past:" \
    "$(<"${state}/running")" >&2
  exit 1
fi

# A count opening with a zero, which bash's arithmetic reads as octal and
# refuses past 7, is read as none too: the round runs, and keeps the check
# failing again afresh.
echo "system:example.net 09 no failed 09" >"${state}/running"
echo fails >"${work}/answers/example.net"
ended=0
bash "${lookups}" "${state}" 5000 example.org,example.net 192.0.2.53 2>/dev/null || ended=$?
if [[ ${ended} -ne 0 ]] || ! grep -q '^system:example.net 1 no failed 1$' "${state}/running"; then
  echo "a state file with a count opening with a zero ended the round (${ended}) or was kept:" \
    "$(<"${state}/running")" >&2
  exit 1
fi
rm "${work}/answers/example.net"

round 0
quiet "a lookup slow for one round said something"
round 0
says "a lookup slow two rounds running" "-p user.err" \
  "the system looked up example.org in" "past 0 ms 2 rounds running"
says "a server slow two rounds running" "-p user.err" \
  "192.0.2.53 answered example.net in" "past 0 ms 2 rounds running"
round 5000
says "a slow lookup quick again" "-p user.notice" "system:example.org is well again"

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

# ps may give another account's process by its name alone.
echo '(mDNSResponder)' >"${work}/answers/mDNSResponder-name"
echo 80.0 >"${work}/answers/mDNSResponder"
for count in 1 2 3 4 5; do
  round 5000
done
says "mDNSResponder busy five rounds running, named alone" "-p user.err" \
  "mDNSResponder uses 80% of a core, past 50% 5 rounds running"
