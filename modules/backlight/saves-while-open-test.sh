# shellcheck shell=bash
# Checks saves-while-open, given a stand-in for systemd-backlight that
# writes what it's asked to save, keeps the screen's level from being saved
# while a lid is shut, saves it with the lid open, and saves a keyboard's
# either way.
# usage: saves-while-open-test.sh <saves-while-open> <what the stand-in writes>
set -euo pipefail

program=$1
said=$2
LIDS=$(mktemp -d)
export LIDS
trap 'rm -rf "$LIDS"' EXIT
mkdir "$LIDS/LID0"

fails() {
  echo "saves-while-open $1" >&2
  exit 1
}
saved() { [[ -f $said ]] && grep -qxF "save $1" "$said"; }

echo "state:      closed" > "$LIDS/LID0/state"
"$program" backlight:intel_backlight
! saved backlight:intel_backlight || fails "saved the screen's level with the lid shut"
"$program" leds:dell::kbd_backlight
saved leds:dell::kbd_backlight || fails "didn't save the keyboard's level with the lid shut"

echo "state:      open" > "$LIDS/LID0/state"
"$program" backlight:intel_backlight
saved backlight:intel_backlight || fails "didn't save the screen's level with the lid open"
