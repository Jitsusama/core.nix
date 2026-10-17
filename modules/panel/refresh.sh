# Drops the built-in panel to 60 Hz once the battery falls below 30%, and
# brings it back to its fastest rate when the charger is plugged in. On
# battery it stays at 60 Hz even if the reading climbs back over 30%, so a
# battery hovering there can't flick the screen back and forth. It acts only
# when that choice changes, so a rate picked by hand holds until the next
# change. UPower says when the battery changes, and niri sets the mode.
below=30

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
  if ! on_battery; then
    echo fast
  elif [ "$last" = slow ] || [ "$(percentage)" -lt "$below" ]; then
    echo slow
  else
    echo fast
  fi
}

# The panel's name and the mode to ask niri for, at the size it shows now:
# the slowest rate from 59 Hz up, or the fastest. Nothing when the panel is
# off, such as with the lid closed.
mode() {
  niri msg --json outputs | jq -r --arg rate "$1" '
    to_entries[]
    | select(.key | startswith("eDP"))
    | .key as $name
    | .value
    | select(.current_mode != null)
    | .modes[.current_mode] as $now
    | [.modes[] | select(.width == $now.width and .height == $now.height)]
    | if $rate == "slow"
      then map(select(.refresh_rate >= 59000)) | min_by(.refresh_rate)
      else max_by(.refresh_rate)
      end
    | select(. != null)
    | "\($name) \(.width)x\(.height)@\(.refresh_rate / 1000 | floor)"
      + ".\(.refresh_rate % 1000 | tostring | ("00" + .)[-3:])"'
}

follow() {
  local now name size
  now=$(wanted)
  [ "$now" = "$last" ] && return
  mode "$now" | while read -r name size; do
    niri msg output "$name" mode "$size"
    echo "$name at $size, the battery at $(percentage)%"
  done
  last=$now
}

# niri starts the panel at its preferred mode, its fastest.
last=fast
follow
gdbus monitor --system --dest org.freedesktop.UPower | while read -r line; do
  case $line in
    *PropertiesChanged*) follow ;;
  esac
done
# The monitor ends only when the system bus does.
exit 1
