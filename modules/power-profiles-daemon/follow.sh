# shellcheck shell=bash
# Sets the power profile for the charger and the battery: performance on mains
# power where the daemon offers it, balanced on battery, and power saver once
# the battery falls below UPower's low mark. On battery it stays in power saver
# even if the reading climbs back over the mark, so a battery hovering there
# can't flick the profile back and forth, until the charger is back. It acts
# only when that choice changes, so a profile picked by hand holds until the
# next change. UPower says when the charger or the battery changes.
#
# Each choice it makes is written to wanted in its runtime folder,
# /run/power-profile, where the shell reads it: a profile that differs from
# it was picked by hand, which the shell's Now lists as a mode left on
# (modules/quickshell/pieces/Profile.qml).
low=$1

on_battery() {
  busctl --system get-property org.freedesktop.UPower /org/freedesktop/UPower \
    org.freedesktop.UPower OnBattery | grep -q true
}

percentage() {
  local reply
  reply=$(busctl --system get-property org.freedesktop.UPower \
    /org/freedesktop/UPower/devices/DisplayDevice org.freedesktop.UPower.Device Percentage)
  reply=${reply#d }
  echo "${reply%%.*}"
}

wanted() {
  if [ "$1" = mains ]; then
    # Without the firmware's or the CPU's support, the daemon offers no
    # performance profile, and asking for one fails.
    if powerprofilesctl list | grep -q 'performance:'; then
      echo performance
    else
      echo balanced
    fi
  elif [ "$last" = power-saver ] || [ "$(percentage)" -lt "$low" ]; then
    echo power-saver
  else
    echo balanced
  fi
}

follow() {
  local supply=mains now
  on_battery && supply=battery
  now=$(wanted "$supply")
  [ "$now" = "$last" ] && return
  powerprofilesctl set "$now" || return
  # Written in place, so a reader watching the file keeps its watch.
  [ -n "${RUNTIME_DIRECTORY:-}" ] && echo "$now" > "$RUNTIME_DIRECTORY/wanted"
  if [ "$supply" = battery ]; then
    echo "power profile: $now, the battery at $(percentage)%"
  else
    echo "power profile: $now, on mains power"
  fi
  last=$now
}

last=
follow
gdbus monitor --system --dest org.freedesktop.UPower | while read -r line; do
  case $line in
    *PropertiesChanged*) follow ;;
  esac
done
# The monitor ends only when the system bus does.
exit 1
