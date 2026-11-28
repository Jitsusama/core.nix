# shellcheck shell=bash
# Makes a Mac's local name, name.local, answer the other machines on its
# network, on the one link they reach it by. macOS answers for its own name
# only while some service names the Mac itself as its host, and then on
# every interface, its hundreds of loopback aliases too, each a record its
# name service probes and fights over (the module's header says when that
# spun a core). So this registers one address record of its own, for the
# name, with dns-sd's -P, which gives the record itself rather than leaving
# it to macOS's own host records, on one link with -i: one record, on one
# link, whatever else the Mac has, so long as macOS keeps a record whose
# host is spelled as its own apart from its own, which a Mac has yet to
# show. dns-sd refuses a link it can't find rather than taking every one.
# The link is the first wired or Wi-Fi one (en*) macOS lists for IPv4, in
# its order of service, with its address: a tunnel, as a VPN's is, or a
# virtual machines' bridge isn't where the network finds the Mac. Every few
# seconds it reads them again, and when either changes, or the answer
# ends, it ends, and the answer with it, for launchd to start it again on
# the link as it is then. With no such link it waits for one, and a read
# scutil can't give changes nothing, so a working answer outlasts it. What
# it does goes to its log, with when, and so does what dns-sd reports, such
# as a name another machine holds, which ends it.
# usage: answer.sh <name> <seconds between reads>
set -euo pipefail

name=$1
every=$2

# Says $1, with the time it's said.
say() {
  local when
  when=$(date -u +%Y-%m-%dT%H:%M:%SZ)
  echo "${when} $1" >&2
}

# Sets link and address to the first en* link macOS lists for IPv4 and its
# address, from scutil --nwi, or to nothing when it lists none with one,
# and read to whether scutil could say. When it can't, both are left as
# they were, so nothing changes: scutil's own words are left out, since
# they'd come at every read, fifteen seconds apart, and macOS's network
# list says no more than that it couldn't be read.
pick() {
  local line section="" at="" listed
  if ! listed=$(scutil --nwi 2>/dev/null); then
    read=failed
    return 0
  fi
  read=listed
  link=""
  address=""
  while IFS= read -r line; do
    case ${line} in
      "IPv4 network interface information"*) section=ipv4 ;;
      "IPv6 network interface information"*) section="" ;;
      *)
        [ "${section}" = ipv4 ] || continue
        if [[ ${line} =~ ^[[:space:]]*([a-z]+[0-9]+)[[:space:]]+:[[:space:]]+flags ]]; then
          at=${BASH_REMATCH[1]}
        elif [[ ${line} =~ ^[[:space:]]*address[[:space:]]+:[[:space:]]+([0-9.]+)[[:space:]]*$ ]] &&
          [[ ${at} == en* ]] && [ -z "${link}" ]; then
          link=${at}
          address=${BASH_REMATCH[1]}
        fi
        ;;
    esac
  done <<<"${listed}"
}

# The answer's process, ended and waited for as this ends, so launchd never
# starts the next while the last still answers.
answering=""
finish() {
  if [ -n "${answering}" ]; then
    kill "${answering}" 2>/dev/null || true
    wait "${answering}" 2>/dev/null || true
  fi
}
trap finish EXIT

link=""
address=""
pick
# Why it waits, said once each time the reason changes.
waiting=""
while [ -z "${link}" ]; do
  why="no wired or Wi-Fi link has an IPv4 address, so ${name}.local waits for one"
  if [ "${read}" = failed ]; then
    why="scutil couldn't list the links, so ${name}.local waits to read them"
  fi
  if [ "${why}" != "${waiting}" ]; then
    say "${why}"
    waiting=${why}
  fi
  # Reading again is how a link coming up is seen: macOS tells a script of
  # a change only through a framework a shell can't reach.
  sleep "${every}"
  pick
done
# Offered, not yet answering: dns-sd's own line, which comes after this
# one, says when it's registered, that another machine holds the name or
# that it couldn't start.
say "${name}.local is offered as ${address} on ${link}"
dns-sd -i "${link}" -P "${name}" _workstation._tcp local 9 "${name}.local" "${address}" >&2 &
answering=$!
while true; do
  # The same: a changed address is seen only by reading it again.
  sleep "${every}"
  if ! kill -0 "${answering}" 2>/dev/null; then
    ended=0
    wait "${answering}" || ended=$?
    answering=""
    say "the answer for ${name}.local on ${link} ended (exit ${ended}), so it starts again"
    exit 1
  fi
  was_link=${link}
  was_address=${address}
  pick
  if [ "${link}" != "${was_link}" ] || [ "${address}" != "${was_address}" ]; then
    say "${name}.local moves from ${was_address} on ${was_link} to ${address:-nothing} on ${link:-no link}"
    exit 0
  fi
done
