#!/usr/bin/env bash
# Runs a Mac's house-report-key job's own lines deciding whether a refusal
# says to make the key anew, over sample keys. The job's Mac tools are
# renamed in the lines given to the stand-ins below, which answer from
# each case's folder: the owner, mode and access list of the file the
# key's name comes to, and whether it comes to one. Shell tests on that
# file stay real, so the case's folder holds it.
#
# Usage: advice-test.sh LINES, LINES the job's lines as checks.nix renames
# them.
#
# The lines sourced set and read the job's own names and call the
# stand-ins, which shellcheck can't see from here.
# shellcheck disable=SC2034,SC2154,SC2329
set -eu

advice=$1
failed=0
here=

for name in stand_in_readlink stand_in_stat stand_in_find stand_in_ls; do
  if ! grep -q "${name} " "${advice}"; then
    echo "the job's advice doesn't run ${name}, so it runs a tool not stood in for" >&2
    exit 1
  fi
done

stand_in_readlink() {
  [[ $1 == -f ]] || return 2
  [[ -f "${here}/seen" ]] || return 1
  cat "${here}/seen"
}

stand_in_stat() {
  [[ "$1 $2" == "-f %u" ]] || return 2
  cat "${here}/owner"
}

# As the job calls it: FILE -maxdepth 0 -perm +MASK, printing FILE when its
# mode has any of MASK's bits.
stand_in_find() {
  [[ "$2 $3 $4" == "-maxdepth 0 -perm" ]] || return 2
  local mode mask
  mode=$(cat "${here}/mode")
  mask=${5#+}
  if ((8#${mode} & 8#${mask})); then
    echo "$1"
  fi
}

stand_in_ls() {
  [[ $1 == -le ]] || return 2
  echo "-rw-------  1 root  wheel  32 Nov 25 21:00 $2"
  cat "${here}/list"
}

# A key the job's name comes to: OWNER, MODE, LIST (ls -le's lines under
# it), and whether the name comes to it at all.
key() {
  here=$(mktemp -d)
  echo "$1" >"${here}/owner"
  echo "$2" >"${here}/mode"
  printf '%b' "$3" >"${here}/list"
  touch "${here}/key"
  if [[ $4 == comes ]]; then
    echo "${here}/key" >"${here}/seen"
  fi
}

# Whether the job, given WHY it refused and the FOLDER its walk ended in,
# advises as WANTED for the key last made: CASE names it.
advises() {
  local case=$1 wanted=$2 got=no
  houseReportWhy=$3
  houseReportFolder=$4
  houseReportRead=/etc/house.key
  houseReportDo=
  # shellcheck source=/dev/null
  . "${advice}"
  [[ -z ${houseReportDo} ]] || got=yes
  if [[ ${got} != "${wanted}" ]]; then
    echo "${case}: advised ${got}, wanted ${wanted}" >&2
    failed=1
  elif [[ ${got} == yes && ${houseReportDo} != "${houseReportKnown}" ]]; then
    echo "${case}: advised '${houseReportDo}', not the job's own words" >&2
    failed=1
  fi
}

walked="is under /etc/house-report, which another account may write"
key 0 600 "" comes
advises "a key root's alone, its folder refused" no "${walked}" /etc
key 0 644 "" comes
advises "a key others may read, its folder refused" yes "${walked}" /etc
key 0 620 "" comes
advises "a key others may write alone, refused for it" yes "may be read or written" /etc
key 0 711 "" comes
advises "a key others may only run" no "may be run by an account other than root's" /etc
key 501 600 "" comes
advises "a key another account owns, its folder refused" yes "${walked}" /etc
key 0 600 " 0: group:everyone allow read,readattr\n" comes
advises "a list letting others read, its link refused" yes "is a link" /etc
key 0 600 " 0: group:everyone allow readattr,readsecurity\n" comes
advises "a list of lesser rights" no "has an access list letting another account in" /etc
key 0 644 "" comes
advises "a key copied, as nothing refused it" no "" /etc
key 0 644 "" comes
advises "a folder that isn't there" no "is in a folder that isn't there" ""
key 0 644 "" nowhere
advises "a name that comes to no file" no "isn't there as a file" /etc

exit "${failed}"
