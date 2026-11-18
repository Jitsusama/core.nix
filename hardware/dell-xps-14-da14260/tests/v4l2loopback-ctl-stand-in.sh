# shellcheck shell=bash
# A stand-in for v4l2loopback-ctl over a folder of its own, as sysfs lists
# loopbacks: add makes the next free videoN with its name and prints its
# node, and delete refuses one with a file named held in it, as the module
# refuses a loopback a program has open.
set -euo pipefail

sysfs=$LOOPBACK_SYSFS
case $1 in
add)
  shift
  name=""
  options=()
  while [ $# -gt 0 ]; do
    if [ "$1" = -n ]; then
      name=$2
      shift 2
    else
      options+=("$1")
      shift
    fi
  done
  n=0
  while [ -e "$sysfs/video$n" ]; do n=$((n + 1)); done
  mkdir "$sysfs/video$n"
  echo "$name" >"$sysfs/video$n/name"
  echo "${options[*]}" >"$sysfs/video$n/options"
  echo "/dev/video$n"
  ;;
delete)
  node=$sysfs/$(basename "$2")
  if [ -e "$node/held" ]; then
    echo "$2: Device or resource busy" >&2
    exit 1
  fi
  rm -r "$node"
  ;;
*)
  echo "the stand-in knows add and delete" >&2
  exit 2
  ;;
esac
