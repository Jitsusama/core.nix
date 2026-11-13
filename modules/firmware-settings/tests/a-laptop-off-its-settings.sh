# shellcheck shell=bash
# Builds what a laptop whose firmware has drifted from its declared
# settings shows firmware-settings, for commands-agree to set its bare run
# at each width from: two settings off what's declared, with long names
# and values as a vendor gives them, and one as declared.
# usage: a-laptop-off-its-settings.sh <folder to build it in>
set -euo pipefail

out=$1
fw=$out/fw/vendor

# A setting the firmware holds, at the value given.
setting() {
  mkdir -p "$fw/attributes/$1"
  echo "$2" > "$fw/attributes/$1/current_value"
}

mkdir -p "$fw/authentication/Admin" "$fw/attributes"
echo 1 > "$fw/authentication/Admin/is_enabled"
: > "$fw/authentication/Admin/current_password"
echo 0 > "$fw/attributes/pending_reboot"
setting BatteryChargeMode Adaptive
setting BatteryChargeStartThreshold 50
setting ThunderboltSecurityLevel UserAuthorization
printf '%s\n' BatteryChargeMode=Custom BatteryChargeStartThreshold=75 \
  ThunderboltSecurityLevel=UserAuthorization > "$out/declared"
: > "$out/waiting"
