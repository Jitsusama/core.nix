# shellcheck shell=bash
# Holds the firmware's settings to the ones the machine declares, through
# the kernel's firmware-attributes class, which Dell's, Lenovo's and HP's
# drivers provide.
#
#   firmware-settings check           names every setting that differs
#   firmware-settings apply           writes them, asking for the admin password
#   firmware-settings set NAME VALUE  writes one for now, which check then names
#
# It exits 1 when a setting differs, 2 when the firmware has no such
# setting, and 77 when it can't read one, as without root.

root=${FIRMWARE_ATTRIBUTES:-/sys/class/firmware-attributes}
declared=${FIRMWARE_SETTINGS:-/etc/firmware-settings}
# How many seconds to wait for a setting to appear: a driver announces its
# interface before it has made the settings in it.
wait=${FIRMWARE_SETTINGS_WAIT:-0}

command=${1:-}

usage() {
  echo "usage: firmware-settings check | apply | set NAME VALUE" >&2
  exit 64
}

# Where a setting lives: the first interface that has it.
where() {
  local dir waited=0
  while true; do
    for dir in "$root"/*/attributes/"$1"; do
      if [ -d "$dir" ]; then
        echo "$dir"
        return 0
      fi
    done
    [ "$waited" -lt "$wait" ] || break
    sleep 1
    waited=$((waited + 1))
  done
  echo "firmware-settings: no firmware setting $1" >&2
  return 2
}

# A setting's value, or a refusal naming the command to run as root: the
# kernel lets only root read most of them, and an empty value would read
# as a setting that differs.
current() {
  if [ ! -r "$1/current_value" ]; then
    echo "firmware-settings: can't read $2; run it as root: run0 firmware-settings $command" >&2
    return 77
  fi
  cat "$1/current_value"
}

# Each declared setting the firmware holds otherwise, as NAME=VALUE.
differences() {
  local name value dir now
  while IFS='=' read -r name value; do
    [ -n "$name" ] || continue
    dir=$(where "$name") || return 2
    now=$(current "$dir" "$name") || return
    [ "$now" = "$value" ] || echo "$name=$value"
  done < "$declared"
}

check() {
  local drift name value
  drift=$(differences) || return
  if [ -z "$drift" ]; then
    echo "The firmware's settings are as declared."
    return 0
  fi
  while IFS='=' read -r name value; do
    echo "$name is $(cat "$(where "$name")/current_value"), declared $value"
  done <<< "$drift"
  return 1
}

pending() {
  local flag
  for flag in "$root"/*/attributes/pending_reboot; do
    if [ "$(cat "$flag" 2> /dev/null)" = 1 ]; then
      echo "The firmware takes some of its settings at the next boot."
    fi
  done
}

# An interface with an admin password changes nothing without it, so it's
# asked for once and given to each such interface.
unlocked=()
unlock() {
  local admin password="" asked=""
  for admin in "$root"/*/authentication/Admin; do
    [ "$(cat "$admin/is_enabled" 2> /dev/null)" = 1 ] || continue
    if [ -z "$asked" ]; then
      printf 'Firmware admin password: ' >&2
      if [ -t 0 ]; then stty -echo; fi
      IFS= read -r password || true
      if [ -t 0 ]; then
        stty echo
        echo >&2
      fi
      asked=1
    fi
    unlocked+=("$admin/current_password")
    printf '%s' "$password" > "$admin/current_password"
  done
}

# The kernel keeps the password until something overwrites it, and any root
# process could change the firmware's settings with it until then.
lock() {
  local file
  if [ -t 0 ]; then stty echo; fi
  for file in "${unlocked[@]}"; do
    echo > "$file" || true
  done
}
trap lock EXIT
trap 'exit 130' INT TERM HUP

apply() {
  local drift before="" name value passes=0
  unlock
  # Some settings unlock others, as a battery's custom limits wait for its
  # custom mode, so it writes in passes until a pass changes nothing.
  while drift=$(differences) && [ -n "$drift" ] && [ "$drift" != "$before" ] \
    && [ "$passes" -lt 10 ]; do
    before=$drift
    passes=$((passes + 1))
    while IFS='=' read -r name value; do
      printf '%s' "$value" 2> /dev/null > "$(where "$name")/current_value" || true
    done <<< "$drift"
  done
  pending
  check
}

set_one() {
  local dir
  dir=$(where "$1") || return 2
  unlock
  printf '%s' "$2" > "$dir/current_value"
  echo "$1 is $(cat "$dir/current_value")"
  pending
}

case "${1:-}" in
  check) check ;;
  apply) apply ;;
  set)
    [ $# = 3 ] || usage
    set_one "$2" "$3"
    ;;
  *) usage ;;
esac
