# shellcheck shell=bash
# Runs the ergodox command over a copy of sysfs holding three raw HID
# nodes: another vendor's raw channel, the board's keyboard and the board's
# own channel, told apart by ZSA's vendor number and Oryx's usage page
# together. It checks each command is written to the board's channel as one
# report and nothing reaches the others, that an unknown layer is refused
# as a usage error, and that with no channel the command says so.
# usage: ergodox-command-reaches-the-board.sh <ergodox>
set -euo pipefail

ergodox=$1
root=$(mktemp -d)
trap 'rm -rf "$root"' EXIT
export ERGODOX_ROOT=$root
# What follows a command's two bytes to fill a report of 32, which goes
# after a report number of 0.
padding=$(printf '%060d' 0)
# The exit statuses sysexits.h gives a usage error and an absent service.
usage=64
unavailable=69

# A raw HID node, by its HID id and its report descriptor's opening bytes.
node() {
  mkdir -p "$root/sys/class/hidraw/$1/device" "$root/dev"
  echo "HID_ID=$2" > "$root/sys/class/hidraw/$1/device/uevent"
  printf '%b' "$3" > "$root/sys/class/hidraw/$1/device/report_descriptor"
  : > "$root/dev/$1"
}
# What's been written to a node, in hex.
sent() {
  od -An -tx1 -v "$root/dev/$1" | tr -d ' \n'
}
fail() {
  echo "$1"
  exit 1
}
# Runs a command, and checks the board's channel got exactly its report and
# no other node got anything.
expect() {
  local want=00$3$padding
  # A command with no argument is given none, so its word stays unquoted.
  # shellcheck disable=SC2086
  "$ergodox" "$1" $2
  [ "$(sent hidraw2)" = "$want" ] || fail "ergodox $1 $2 wrote $(sent hidraw2), not $want."
  [ -z "$(sent hidraw0)$(sent hidraw1)" ] \
    || fail "ergodox $1 $2 wrote to a node that isn't the board's channel."
}
# Runs a command that should fail, and says what it exited with.
exited() {
  local status=0
  "$ergodox" "$@" 2> /dev/null || status=$?
  echo "$status"
}

# Usage page 0xff60, usage 0x61: Oryx's raw channel.
raw='\006\140\377\011\141'
# Generic desktop, keyboard.
keyboard='\005\001\011\006'
node hidraw0 0003:0000046D:0000C52B "$raw"
node hidraw1 0003:00003297:00004976 "$keyboard"
node hidraw2 0003:00003297:00004976 "$raw"

expect bootloader "" 8000
expect layer numb 8202
expect layer Punk 8201
expect layer 0 8200
expect lights off 8300
expect brighter "" 8401
expect dimmer "" 8400

status=$(exited layer sideways)
[ "$status" = "$usage" ] || fail "An unknown layer exited $status, not $usage."

rm -r "$root/sys/class/hidraw/hidraw2" "$root/dev/hidraw2"
status=$(exited bootloader)
[ "$status" = "$unavailable" ] || fail "With no board, bootloader exited $status, not $unavailable."
[ -z "$(sent hidraw0)$(sent hidraw1)" ] || fail "With no board, ergodox wrote to another node."
