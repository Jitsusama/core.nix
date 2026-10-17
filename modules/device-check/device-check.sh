# Names the devices that have stopped working: any whose runtime power
# management failed, which the kernel then holds failed and won't wake until
# the next boot, any the machine expects that isn't there, and any disk that
# reported a fault to smartd this boot, which device-check-disk keeps.
#
#   device-check    names what has gone wrong since it last ran, and what
#                   works again, in the journal and on the desktop
#
# Each is named once, so running it often costs nothing to read. It exits 0
# whatever it finds, and non-zero only when it couldn't look.

root=${DEVICE_CHECK_ROOT:-}
expected=${DEVICE_CHECK_EXPECTED:-/etc/device-check}
state=${DEVICE_CHECK_STATE:-${XDG_RUNTIME_DIR:?}/device-check}
notify=${DEVICE_CHECK_NOTIFY:-notify-send}
# sort and comm have to agree on the order.
export LC_ALL=C

mkdir -p "$state"
touch "$state/found"
now=$(mktemp "$state/now.XXXXXX")
trap 'rm -f "$now"' EXIT

# Each device whose power change failed, as "power", a tab, and the device
# with its driver.
failed_power() {
  local status value dev driver
  while IFS= read -r -d '' status; do
    read -r value < "$status" || continue
    [ "$value" = error ] || continue
    dev=${status%/power/runtime_status}
    driver=$(readlink "$dev/driver" || echo none)
    printf 'power\t%s (%s)\n' "${dev##*/}" "${driver##*/}"
  done < <(find "$root/sys/devices" -path '*/power/runtime_status' -print0 2> /dev/null)
}

# Each expected device that isn't there, as "missing", a tab, and the name
# the machine gave it. Its path's last part may be a glob.
missing() {
  local name path found
  [ -e "$expected" ] || return 0
  while IFS='=' read -r name path; do
    [ -n "$name" ] || continue
    found=$(find "$root${path%/*}" -mindepth 1 -maxdepth 1 -name "${path##*/}" -print -quit 2> /dev/null || true)
    [ -n "$found" ] || printf 'missing\t%s\n' "$name"
  done < "$expected"
}

# Each disk that reported a fault, as "disk", a tab, and the disk with what
# it reported.
disk_faults() {
  local file device message
  for file in "$root/run/device-check/disks"/*; do
    [ -f "$file" ] || continue
    IFS=$'\t' read -r device message < "$file" || continue
    printf 'disk\t%s reports %s\n' "$device" "$message"
  done
}

{
  failed_power
  missing
  disk_faults
} | sort -u > "$now"

# Says each finding, warnings for what broke and notices for what's fixed,
# whose priority journald reads from the prefix.
tell() {
  local change=$1 kind subject line priority=4 urgency=critical
  [ "$change" = broke ] || { priority=5 urgency=normal; }
  while IFS=$'\t' read -r kind subject; do
    case $kind/$change in
      power/broke) line="$subject failed a power change and won't wake until the next boot" ;;
      power/fixed) line="$subject wakes again" ;;
      missing/broke) line="$subject is missing" ;;
      missing/fixed) line="$subject is back" ;;
      disk/broke) line="The disk $subject" ;;
      disk/fixed) line="The disk ${subject/ reports / no longer reports }" ;;
    esac
    echo "<$priority>$line" >&2
    "$notify" -u "$urgency" -a Devices "$line" || true
  done
}
comm -13 "$state/found" "$now" | tell broke
comm -23 "$state/found" "$now" | tell fixed
mv "$now" "$state/found"
trap - EXIT
