# shellcheck shell=bash
# A stand-in udevadm for give-back-never-fails.sh, over the made-up machine
# at MX_BRIO_ROOT, which that test runs through the bash it was given, since
# a build's sandbox has no /usr/bin/env for a shebang to find.
#
# It answers as udev does: info fails for a node no longer in dev/, and
# reads each node's record from facts/ otherwise. A trigger writes each
# replay to triggers before answering it. A removal leaves a node only the
# kernel's own keys, as udev deletes its record, or unplugs the node when
# after/ holds a .unplugged for it; an add takes the node's record from
# after/. A file named fails makes every trigger fail, adds-fail only each
# add, and hangs every settle; killed has the first settle end the
# give-back with the TERM its unit's timeout sends.
#
# It refuses what give-back.sh must never ask: a flag it wasn't written for,
# a trigger naming no device, which udev replays on every device the
# machine has, or a settle with no bound, under a second, or one that twice
# over would outlast the unit's TimeoutStartSec, given as MX_BRIO_TIMEOUT,
# as would every settle of a run together.
set -euo pipefail
root=${MX_BRIO_ROOT:?}
limit=${MX_BRIO_TIMEOUT:?}

refuse() {
  echo "the stand-in udevadm refuses: $*" >&2
  echo "$*" >>"${root}/refused"
  exit 99
}

case "$1" in
info)
  node=${3##*/}
  [[ -e "${root}/dev/${node}" ]] || exit 1
  case "$2" in
  --query=property) cat "${root}/facts/${node}" ;;
  --query=path) echo "/devices/${node}" ;;
  *) refuse "info $2" ;;
  esac
  ;;
trigger)
  [[ "$2" == --action=add || "$2" == --action=remove ]] || refuse "trigger $2"
  action=${2#--action=}
  shift 2
  (($# > 0)) || refuse "a trigger of every device"
  for path in "$@"; do
    [[ "${path}" == /* ]] || refuse "trigger ${path}"
    node=${path##*/}
    echo "${action} ${node}" >>"${root}/triggers"
  done
  [[ ! -e "${root}/fails" ]] || exit 1
  [[ "${action}" != add || ! -e "${root}/adds-fail" ]] || exit 1
  for path in "$@"; do
    node=${path##*/}
    if [[ "${action}" == remove && -e "${root}/after/${node}.unplugged" ]]; then
      rm -f "${root}/dev/${node}" "${root}/sys/devices/${node}" "${root}/facts/${node}"
    elif [[ "${action}" == remove ]]; then
      printf 'DEVNAME=/dev/%s\nSUBSYSTEM=video4linux\n' "${node}" >"${root}/facts/${node}"
    elif [[ -e "${root}/after/${node}" ]]; then
      cp "${root}/after/${node}" "${root}/facts/${node}"
    fi
  done
  ;;
settle)
  bound=${2:-}
  [[ "${bound}" == --timeout=* ]] || refuse "a settle with no bound"
  seconds=${bound#--timeout=}
  ((seconds >= 1)) || refuse "a settle of ${seconds} s, which waits for nothing"
  ((seconds * 2 < limit)) || refuse "a settle of ${seconds} s, twice over past ${limit} s"
  echo "${seconds}" >>"${root}/settles"
  total=0
  while read -r each; do
    total=$((total + each))
  done <"${root}/settles"
  ((total < limit)) || refuse "settles of ${total} s in all, past ${limit} s"
  if [[ -e "${root}/killed" ]]; then
    rm "${root}/killed"
    kill -TERM "${PPID}"
  fi
  [[ ! -e "${root}/hangs" ]] || exit 1
  ;;
*) refuse "$1" ;;
esac
