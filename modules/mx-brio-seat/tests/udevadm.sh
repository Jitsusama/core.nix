# shellcheck shell=bash
# A stand-in udevadm for give-back-never-fails.sh, over the made-up machine
# at MX_BRIO_ROOT, which that test runs through the bash it was given, since
# a build's sandbox has no /usr/bin/env. It reads each node's record from
# facts/, writes each replay to triggers before answering it, takes a node's
# record from after/ once it's added, or loses it when after/ holds a .gone
# for it, and fails every trigger, or every settle, when a file named fails,
# or hangs, says to. It refuses what give-back.sh must never ask: a flag it
# wasn't written for, or a settle with no bound, or one that twice over
# would outlast the unit's TimeoutStartSec, given as MX_BRIO_TIMEOUT.
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
  for path in "$@"; do
    [[ "${path}" == /* ]] || refuse "trigger ${path}"
    node=${path##*/}
    echo "${action} ${node}" >>"${root}/triggers"
  done
  [[ ! -e "${root}/fails" ]] || exit 1
  for path in "$@"; do
    node=${path##*/}
    if [[ "${action}" != add ]]; then
      continue
    fi
    if [[ -e "${root}/after/${node}.gone" ]]; then
      rm -f "${root}/facts/${node}"
    elif [[ -e "${root}/after/${node}" ]]; then
      cp "${root}/after/${node}" "${root}/facts/${node}"
    fi
  done
  ;;
settle)
  bound=${2:-}
  [[ "${bound}" == --timeout=* ]] || refuse "a settle with no bound"
  seconds=${bound#--timeout=}
  ((seconds * 2 < limit)) || refuse "a settle of ${seconds} s, twice over past ${limit} s"
  [[ ! -e "${root}/hangs" ]] || exit 1
  ;;
*) refuse "$1" ;;
esac
