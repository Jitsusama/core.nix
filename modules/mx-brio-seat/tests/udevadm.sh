# shellcheck shell=bash
# A stand-in udevadm for give-back-never-fails.sh, over the made-up machine
# at MX_BRIO_ROOT, which that test runs through the bash it was given, since
# a build's sandbox has no /usr/bin/env for a shebang to find.
#
# It answers as udev does: info fails for a node no longer in dev/, saying
# so, and reads each node's record from facts/ otherwise. An add queued
# before the give-back, as at boot, is a record in queued/, which a settle
# that doesn't run out writes to facts/. A trigger writes each
# replay to triggers before answering it. A removal leaves a node only the
# kernel's own keys, as udev deletes its record, or unplugs the node when
# after/ holds a .unplugged for it; an add takes the node's record from
# after/. A file named fails makes every trigger fail, adds-fail only each
# add, and hangs every settle; adds-partly refuses an add once its first
# node's add is queued all the same; a refused trigger says why, as
# udevadm does; lags has an add queued and not yet handled, its settle
# running out; killed has the first settle end the give-back with the TERM
# its unit's timeout sends; a node named in vanishes/ is unplugged as info
# reads it, one named in paths-fail/ can't have its path read, and one named
# in paths-vanish/ is unplugged as its path is read.
#
# It refuses what give-back.sh must never ask: a flag it wasn't written for,
# a trigger naming no device, which udev replays on every device the
# machine has, or a settle with no bound, under a second, or one that three
# times over, as many settles as give-back.sh makes, would outlast the
# unit's TimeoutStartSec, given as MX_BRIO_TIMEOUT, as would every settle of
# a run together.
set -euo pipefail
root=${MX_BRIO_ROOT:?}
limit=${MX_BRIO_TIMEOUT:?}

refuse() {
  echo "the stand-in udevadm refuses: $*" >&2
  echo "$*" >>"${root}/refused"
  exit 99
}

# Refuses the trigger of the device at $1 as udevadm does, saying why.
refused() {
  echo "Failed to write '${action}' to '$1/uevent': Permission denied" >&2
  exit 1
}

case "$1" in
info)
  node=${3##*/}
  if [[ -e "${root}/vanishes/${node}" ]]; then
    rm -f "${root}/dev/${node}" "${root}/facts/${node}" "${root}/vanishes/${node}"
  fi
  if [[ ! -e "${root}/dev/${node}" ]]; then
    echo "Unknown device \"$3\": No such device" >&2
    exit 1
  fi
  case "$2" in
  --query=property) cat "${root}/facts/${node}" ;;
  --query=path)
    if [[ -e "${root}/paths-vanish/${node}" ]]; then
      rm -f "${root}/dev/${node}" "${root}/facts/${node}"
      echo "Unknown device \"$3\": No such device" >&2
      exit 1
    fi
    if [[ -e "${root}/paths-fail/${node}" ]]; then
      echo "Failed to get device path for \"$3\": Permission denied" >&2
      exit 1
    fi
    echo "/devices/${node}"
    ;;
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
  [[ ! -e "${root}/fails" ]] || refused "$1"
  [[ "${action}" != add || ! -e "${root}/adds-fail" ]] || refused "$1"
  if [[ "${action}" == add && -e "${root}/adds-partly" ]]; then
    mkdir -p "${root}/queued"
    node=${1##*/}
    mv "${root}/after/${node}" "${root}/queued/${node}"
    refused "$2"
  fi
  [[ "${action}" != add || ! -e "${root}/lags" ]] || exit 0
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
  (($# <= 2)) || refuse "settle ${*:2}"
  bound=${2:-}
  [[ "${bound}" == --timeout=* ]] || refuse "a settle with no bound"
  seconds=${bound#--timeout=}
  ((seconds >= 1)) || refuse "a settle of ${seconds} s, which waits for nothing"
  ((seconds * 3 < limit)) || refuse "a settle of ${seconds} s, three times over past ${limit} s"
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
  # The adds queued before the give-back, handled once udev settles.
  if [[ -d "${root}/queued" ]]; then
    for queued in "${root}/queued"/*; do
      [[ -e "${queued}" ]] || continue
      mv "${queued}" "${root}/facts/${queued##*/}"
    done
  fi
  # The add a lagging udev hasn't handled yet is the settle that runs out.
  if [[ -e "${root}/lags" ]] && grep -q '^add ' "${root}/triggers"; then
    exit 1
  fi
  ;;
*) refuse "$1" ;;
esac
