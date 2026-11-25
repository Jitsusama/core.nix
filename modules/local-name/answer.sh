# shellcheck shell=bash
# Makes a Mac's local name, name.local, answer the other machines on its
# network, on the one link they reach it by. macOS answers for its own name
# only while some service names the Mac itself as its host, and then on
# every interface, its hundreds of loopback aliases too, each a record its
# name service probes and fights over (the module's header says when that
# spun a core). So this registers one address record of its own, for the
# name, with dns-sd's -P, which names a host of its own rather than the
# Mac's, on one link with -i: one record, on one link, whatever else the
# Mac has.
# The link is the first wired or Wi-Fi one (en*) macOS lists for IPv4, in
# its order of service, with its address: a tunnel, as a VPN's is, or a
# virtual machines' bridge isn't where the network finds the Mac. Every few
# seconds it reads them again, and when either changes, or the answer
# ends, it ends, and the answer with it, for launchd to start it again on
# the link as it is then. With no such link it waits for one. What it does
# goes to its log, with when.
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
# address, from scutil --nwi, or to nothing when it lists none with one or
# can't say, which waiting says once: what scutil says on a failed read is
# left out, since it would be said at every read.
pick() {
  local line section="" at="" listed
  link=""
  address=""
  listed=$(scutil --nwi 2>/dev/null) || listed=""
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

pick
waited=""
while [ -z "${link}" ]; do
  if [ -z "${waited}" ]; then
    say "no wired or Wi-Fi link has an IPv4 address, so ${name}.local waits for one"
    waited=yes
  fi
  # Reading again is how a link coming up is seen: macOS tells a script of
  # a change only through a framework a shell can't reach.
  sleep "${every}"
  pick
done
dns-sd -i "${link}" -P "${name}" _workstation._tcp local 9 "${name}.local" "${address}" >/dev/null &
answering=$!
say "${name}.local answers ${address} on ${link}"
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
