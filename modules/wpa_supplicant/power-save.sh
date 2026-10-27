# shellcheck shell=bash
# Keeps every Wi-Fi card awake while the machine runs on the charger, and
# lets it doze between the access point's beacons on battery. udev runs this
# whenever a card appears or a charger comes or goes, so each card is set as
# it comes and changed only when the power does.
#
# A dozing card leaves the access point to hold what comes in for it until it
# next wakes. Some access points hold what's marked as voice or video for
# seconds while anything else downloads, which is a call breaking up. A
# machine with no battery has nothing to save, so its cards stay awake too.
battery=no
charger=no
for supply in /sys/class/power_supply/*; do
  [ -r "$supply/type" ] || continue
  case $(cat "$supply/type") in
    Battery)
      if [ "$(cat "$supply/present" 2>/dev/null || echo 1)" = 1 ]; then
        battery=yes
      fi
      ;;
    Mains | USB)
      if [ "$(cat "$supply/online" 2>/dev/null || echo 0)" = 1 ]; then
        charger=yes
      fi
      ;;
  esac
done

if [ "$battery" = yes ] && [ "$charger" = no ]; then
  wanted=on
else
  wanted=off
fi

for radio in /sys/class/net/*/phy80211; do
  [ -e "$radio" ] || continue
  card=$(basename "$(dirname "$radio")")
  now=$(iw dev "$card" get power_save 2>/dev/null) || continue
  [ "$now" = "Power save: $wanted" ] && continue
  if iw dev "$card" set power_save "$wanted"; then
    logger -t wifi-power-save "$card: power save $wanted"
  else
    logger -t wifi-power-save "$card: power save couldn't be turned $wanted"
  fi
done
