# shellcheck shell=bash
# Checks the job that makes a Mac's local name answer on its network, with a
# stand-in scutil that lists the links macOS would, read by read, and a
# stand-in dns-sd that records what it's asked to answer and answers until
# it's ended, or ends by itself when told to: the name answers the first
# wired or Wi-Fi link's address, past a tunnel or a bridge listed before
# it, on that link alone and as a host of its own; a moved address, another
# link or none ends it, and its answer, for launchd to start it again; with
# no link, or a scutil that fails, it waits and says so once; and an answer
# that ends by itself ends it, saying so.
# usage: local-name-answers-on-its-link.sh <answer.sh> <bash>
set -euo pipefail

answer=$1
bash=$2
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir "$work/bin"
cat > "$work/bin/scutil" <<EOF
#!${bash}
[ "\$*" = --nwi ] || exit 64
read=\$(( \$(cat "$work/reads") + 1 ))
echo "\${read}" > "$work/reads"
while [ "\${read}" -gt 1 ] && [ ! -e "$work/nwi.\${read}" ]; do read=\$((read - 1)); done
if [ "\$(cat "$work/nwi.\${read}")" = fails ]; then echo "scutil: no network information" >&2; exit 1; fi
cat "$work/nwi.\${read}"
EOF
cat > "$work/bin/dns-sd" <<EOF
#!${bash}
echo "\$*" >> "$work/answers"
echo "\$\$" > "$work/answering"
if [ -e "$work/ends" ]; then exit 3; fi
exec sleep infinity
EOF
chmod +x "$work/bin/scutil" "$work/bin/dns-sd"
export PATH="$work/bin:$PATH"

fail() {
  echo "local-name-answers-on-its-link: $1" >&2
  exit 1
}
# Writes the $1th read of scutil --nwi: macOS's layout, its IPv4 links in
# order from link=address pairs, and an IPv6 section that must be passed by.
nwi() {
  local read=$1 pair
  shift
  {
    printf 'Network information\n\nIPv4 network interface information\n'
    if [ "$#" -eq 0 ]; then
      printf '   No IPv4 states found\n'
    fi
    for pair in "$@"; do
      printf '%8s : flags      : 0x5 (IPv4,DNS)\n' "${pair%%=*}"
      printf '           address    : %s\n' "${pair#*=}"
      printf '           reach      : 0x00000002 (Reachable)\n'
    done
    printf '\n   REACH : flags 0x00000002 (Reachable)\n\n'
    printf 'IPv6 network interface information\n'
    printf '     en7 : flags      : 0x6 (IPv6,DNS)\n'
    printf '           address    : fe80::1\n'
    printf '\nNetwork interfaces: en0\n'
  } > "$work/nwi.${read}"
}
# Starts a case afresh.
fresh() {
  rm -f "$work"/nwi.* "$work/answers" "$work/answering" "$work/ends"
  echo 0 > "$work/reads"
}
# Runs the job, reading every 0 seconds, its exit in ended, what it said,
# without when, in said, and what dns-sd was asked in answers.
answers() {
  ended=0
  timeout 60 "$bash" "$answer" its-own-name 0 2> "$work/said" || ended=$?
  [ "${ended}" -ne 124 ] || fail "$1: it ran a minute without ending"
  said=$(cut -d' ' -f2- "$work/said")
  asked=$(cat "$work/answers" 2> /dev/null || echo "(dns-sd wasn't run)")
}
# Fails saying $1 unless the job ended $2, said $3 and asked dns-sd $4.
holds() {
  [ "${ended}" = "$2" ] || fail "$1: it ended ${ended}, not $2; it said: ${said}"
  [ "${said}" = "$3" ] || fail "$1: it said \"${said}\", not \"$3\""
  [ "${asked}" = "$4" ] || fail "$1: dns-sd was asked \"${asked}\", not \"$4\""
}
# Fails saying $1 if the dns-sd it started still answers.
ended_its_answer() {
  local answering=""
  if [ -e "$work/answering" ]; then
    answering=$(cat "$work/answering")
  fi
  if [ -n "${answering}" ] && kill -0 "${answering}" 2> /dev/null; then
    fail "$1: it ended, leaving its answer running"
  fi
}
# What dns-sd is asked for the name on en0 at 192.168.4.49, as a host of its
# own on that link alone.
on_en0="-i en0 -P its-own-name _workstation._tcp local 9 its-own-name.local 192.168.4.49"

fresh
nwi 1 utun4=100.104.89.255 bridge100=192.168.64.1 en0=192.168.4.49 en1=10.0.0.2
nwi 3 utun4=100.104.89.255 bridge100=192.168.64.1 en0=192.168.4.50 en1=10.0.0.2
answers "a moved address"
holds "a moved address" 0 "its-own-name.local answers 192.168.4.49 on en0
its-own-name.local moves from 192.168.4.49 on en0 to 192.168.4.50 on en0" \
  "${on_en0}"
ended_its_answer "a moved address"

fresh
nwi 1 en0=192.168.4.49 en1=10.0.0.2
nwi 2 utun4=100.104.89.255 en1=10.0.0.2
answers "another link"
holds "another link" 0 "its-own-name.local answers 192.168.4.49 on en0
its-own-name.local moves from 192.168.4.49 on en0 to 10.0.0.2 on en1" \
  "${on_en0}"
ended_its_answer "another link"

fresh
nwi 1 en0=192.168.4.49
nwi 2 utun4=100.104.89.255
answers "no link left"
holds "no link left" 0 "its-own-name.local answers 192.168.4.49 on en0
its-own-name.local moves from 192.168.4.49 on en0 to nothing on no link" \
  "${on_en0}"
ended_its_answer "no link left"

fresh
echo fails > "$work/nwi.1"
nwi 2 utun4=100.104.89.255 bridge100=192.168.64.1
nwi 3
nwi 4 en0=192.168.4.49
nwi 5 en0=192.168.4.51
answers "waiting for a link"
holds "waiting for a link" 0 "no wired or Wi-Fi link has an IPv4 address, so its-own-name.local waits for one
its-own-name.local answers 192.168.4.49 on en0
its-own-name.local moves from 192.168.4.49 on en0 to 192.168.4.51 on en0" \
  "${on_en0}"
ended_its_answer "waiting for a link"

fresh
nwi 1 en0=192.168.4.49
touch "$work/ends"
answers "an answer that ends"
holds "an answer that ends" 1 "its-own-name.local answers 192.168.4.49 on en0
the answer for its-own-name.local on en0 ended (exit 3), so it starts again" \
  "${on_en0}"
