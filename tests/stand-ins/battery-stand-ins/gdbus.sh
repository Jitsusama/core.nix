# shellcheck shell=bash
# Stands in for gdbus monitoring UPower: it says UPower has its name, then
# that the battery's properties changed once for each step left in
# $STATE/steps, the service having asked once as it started, then ends, as
# a monitor does when UPower goes.
# usage: gdbus.sh monitor <arguments>
set -euo pipefail
: "${STATE:?run.sh gives the folder the states are in}"

changed="org.freedesktop.DBus.Properties.PropertiesChanged"
echo "The name org.freedesktop.UPower is owned by :1.12"
steps=$(wc -l < "$STATE/steps")
for _ in $(seq "$steps"); do
  echo "/org/freedesktop/UPower/devices/DisplayDevice: $changed ('org.freedesktop.UPower.Device', ...)"
done
