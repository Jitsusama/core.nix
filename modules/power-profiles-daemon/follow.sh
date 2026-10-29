# shellcheck shell=bash
# Sets the power profile for the charger and the battery: performance on mains
# power where the daemon offers it, balanced on battery, and power saver once
# the battery falls below UPower's low mark. On battery it stays in power saver
# even if the reading climbs back over the mark, so a battery hovering there
# can't flick the profile back and forth, until the charger is back. It acts
# only when that choice changes, so a profile picked by hand holds until the
# next change. UPower says when the charger or the battery changes.
#
# The profile is set over D-Bus and read back, since the daemon can refuse
# one, as it did power saver for an hour on battery when the kernel wouldn't
# take it. A refused profile is said once, as a warning, and tried again
# quietly at each change until it takes, which is said too.
#
# Each choice that takes is written to wanted in its runtime folder,
# /run/power-profile, where the shell reads it: a profile that differs from
# it was picked by hand, which the shell's Now lists as a mode left on
# (modules/quickshell/pieces/Profile.qml). A refused one isn't, so the
# profile left active still matches it and isn't taken for Joel's pick.
low=$1

daemon=(
  org.freedesktop.UPower.PowerProfiles /org/freedesktop/UPower/PowerProfiles
  org.freedesktop.UPower.PowerProfiles
)

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

active() {
  local reply
  reply=$(busctl --system get-property "${daemon[@]}" ActiveProfile)
  reply=${reply#s \"}
  echo "${reply%\"}"
}

wanted() {
  if [ "$1" = mains ]; then
    # Without the firmware's or the CPU's support, the daemon offers no
    # performance profile, and asking for one fails.
    if busctl --system get-property "${daemon[@]}" Profiles |
      grep -qF '"Profile" s "performance"'; then
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

# Says why the profile wanted didn't take, once for each profile refused.
refused() {
  [ "$now" = "$refusing" ] && return
  echo "<4>power profile: $now wasn't taken: $1"
  refusing=$now
}

follow() {
  local supply=mains now error
  on_battery && supply=battery
  now=$(wanted "$supply")
  [ "$now" = "$last" ] && return
  if ! error=$(busctl --system set-property "${daemon[@]}" ActiveProfile s "$now" 2>&1); then
    refused "${error:-busctl failed}"
    return
  fi
  if [ "$(active)" != "$now" ]; then
    refused "the daemon kept $(active)"
    return
  fi
  # Written in place, so a reader watching the file keeps its watch.
  [ -n "${RUNTIME_DIRECTORY:-}" ] && echo "$now" > "$RUNTIME_DIRECTORY/wanted"
  if [ "$supply" = battery ]; then
    echo "power profile: $now, the battery at $(percentage)%"
  else
    echo "power profile: $now, on mains power"
  fi
  last=$now
  refusing=
}

last=
refusing=
follow
gdbus monitor --system --dest org.freedesktop.UPower | while read -r line; do
  case $line in
    *PropertiesChanged*) follow ;;
  esac
done
# The monitor ends only when the system bus does.
exit 1
