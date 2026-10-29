# shellcheck shell=bash
# Stops systemd's runtime watchdog for the length of a sleep, and starts it
# afresh after. systemd-sleep runs it with "pre" or "post" and the sleep's
# name.
#
# The chipset's watchdog (sp5100_tco) is set up only when its driver loads,
# and the driver has no step for a resume: after a sleep the watchdog no
# longer reset a crashed kernel, which sat on its panic until the power
# button was held. Stopping it and starting it again sets the chip up anew,
# and a crash right after then restarted the laptop by itself. Stopped
# before the sleep, it can't reset a hibernation while the kernel, with
# systemd frozen, writes the image.
#
# The timeout systemd runs with is kept in a file, so only a watchdog this
# stopped is started again, at the timeout it had, and nothing here ever
# fails the sleep. BUSCTL and KEPT move both for the check.
set -u

busctl=${BUSCTL:-busctl}
kept=${KEPT:-/run/watchdog-wakes-again}
manager=(org.freedesktop.systemd1 /org/freedesktop/systemd1 org.freedesktop.systemd1.Manager)

case "$1" in
pre)
  timeout=$("$busctl" get-property "${manager[@]}" RuntimeWatchdogUSec) || exit 0
  timeout=${timeout#t }
  [ "$timeout" -gt 0 ] 2>/dev/null || exit 0
  echo "$timeout" >"$kept"
  "$busctl" set-property "${manager[@]}" RuntimeWatchdogUSec t 0 ||
    echo "couldn't stop the runtime watchdog before the sleep" >&2
  ;;
post)
  [ -f "$kept" ] || exit 0
  timeout=$(cat "$kept")
  rm -f "$kept"
  "$busctl" set-property "${manager[@]}" RuntimeWatchdogUSec t "$timeout" ||
    echo "couldn't start the runtime watchdog again after the sleep" >&2
  ;;
esac
exit 0
