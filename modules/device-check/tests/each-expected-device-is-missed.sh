# shellcheck shell=bash
# Proves a machine's expected devices against what its sysfs was seen to
# hold: builds a stand-in sysfs from the seen paths and checks device-check
# finds nothing missing in it, so every entry names something the machine
# really has; then takes away, one entry at a time, whatever that entry's
# path matches and checks device-check names that entry missing, and only
# that one. Matching here is bash's own, apart from device-check's.
# device-check comes from PATH.
# usage: each-expected-device-is-missed.sh <expected> <seen>
set -euo pipefail

expected=$1
seen=$2
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
export DEVICE_CHECK_EXPECTED=$expected DEVICE_CHECK_NOTIFY=true DEVICE_CHECK_SUSPENDED=0

# The seen paths, comments aside.
paths=()
while IFS= read -r path; do
  [ "${path:0:1}" = "#" ] || paths+=("$path")
done < "$seen"

# What device-check finds missing in a sysfs of the seen paths but those
# the given folder's pattern matches, a name a line.
missing_without() {
  local dir=$1 pattern=$2 root path
  root=$(mktemp -d -p "$work")
  for path in "${paths[@]}"; do
    # shellcheck disable=SC2053 # The pattern is a glob on purpose.
    if [ "${path%/*}" = "$dir" ] && [[ ${path##*/} == $pattern ]]; then
      continue
    fi
    mkdir -p "$root$path"
  done
  DEVICE_CHECK_ROOT=$root DEVICE_CHECK_STATE=$root/state device-check --json \
    | jq -r '.wrong[] | select(.kind == "missing") | .subject'
}

none=$(missing_without "" "")
[ -z "$none" ] || {
  echo "Expected devices the machine was seen to have are named missing:"
  echo "$none"
  exit 1
}

count=0
while IFS= read -r line; do
  [ -n "$line" ] || continue
  name=${line%%=*}
  path=${line#*=}
  found=$(missing_without "${path%/*}" "${path##*/}")
  [ "$found" = "$name" ] || {
    echo "Taking away what $path matches should leave $name alone missing; it left:"
    echo "${found:-nothing}"
    exit 1
  }
  count=$((count + 1))
done < "$expected"
[ "$count" -gt 0 ] || {
  echo "No expected devices were given."
  exit 1
}
echo "Each of $count expected devices is named missing once it's gone."
