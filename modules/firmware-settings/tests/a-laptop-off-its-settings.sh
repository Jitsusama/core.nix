# shellcheck shell=bash
# Builds what a laptop whose firmware has drifted from its declared
# settings shows firmware-settings, for commands-agree to set its bare run
# at each width from: two settings off what's declared, with long names
# and values as a vendor gives them, and one as declared.
# usage: a-laptop-off-its-settings.sh <folder to build it in>
set -euo pipefail

out=$1
fw=$out/fw/vendor

mkdir -p "$fw/authentication/Admin" "$fw/attributes"
echo 1 > "$fw/authentication/Admin/is_enabled"
: > "$fw/authentication/Admin/current_password"
echo 0 > "$fw/attributes/pending_reboot"
for held in BatteryChargeMode=Adaptive BatteryChargeStartThreshold=50 \
  ThunderboltSecurityLevel=UserAuthorization; do
  mkdir -p "$fw/attributes/${held%%=*}"
  echo "${held#*=}" > "$fw/attributes/${held%%=*}/current_value"
done
printf '%s\n' BatteryChargeMode=Custom BatteryChargeStartThreshold=75 \
  ThunderboltSecurityLevel=UserAuthorization > "$out/declared"
: > "$out/waiting"
