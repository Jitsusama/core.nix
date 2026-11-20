# shellcheck shell=bash
# Stops systemd's runtime watchdog for the length of a sleep that
# hibernates, and starts it again after. systemd-sleep runs it with "pre"
# or "post" and the sleep's name.
#
# While the kernel writes a hibernation's image, systemd is frozen and
# can't feed the watchdog, and Intel's iTCO_wdt stops its count by itself
# only for suspend-to-idle. An image of tens of gigabytes takes most of a
# minute to write, so the watchdog would reset the machine part way, losing
# the image and everything in it. Stopped before the hibernation, it can't.
# A suspend is left to the driver, which stops the count in the last step
# before the machine sleeps, with interrupts off, and starts it in the
# first after it wakes, so a driver that hangs on the way in or out before
# or after those is still reset. One that hangs between them, or a machine
# that never wakes from suspend-to-idle, isn't, and the NMI watchdog
# doesn't see it either, with every core idle. So a hang that leaves nothing in
# pstore, with a suspend the journal's last words, was most likely there.
#
# systemd-sleep names the whole sleep in its arguments, suspend-then-
# hibernate say, and the step it's taking now in SYSTEMD_SLEEP_ACTION, so
# the step decides. Only a watchdog this stopped is started again, a file
# keeping that it did, and nothing here ever fails the sleep. It's started
# by handing it back to systemd's own setting, not by naming its timeout,
# which systemd would keep over the setting until a reboot, so a machine
# that stops importing this would still be watched through a hibernation.
# systemd opens the watchdog at most once in 5 s, so after a sleep that
# short it's back at systemd's next feeding, half the timeout on, rather
# than at once. BUSCTL and KEPT move both for the check. Its help takes the
# shape every command of ours does (docs/commands.md), by hand.
set -u

busctl=${BUSCTL:-busctl}
kept=${KEPT:-/run/watchdog-sleeps}
manager=(org.freedesktop.systemd1 /org/freedesktop/systemd1 org.freedesktop.systemd1.Manager)

case "${1:-}" in
-h | --help)
  cat << 'EOF'
watchdog-sleeps  stop the watchdog for a hibernation

  systemd-sleep runs it with pre before a sleep that
  hibernates, which stops systemd's runtime watchdog,
  and post after, which starts it again as systemd is
  set to. A suspend is left alone. It never fails a
  sleep.

EXAMPLES
  watchdog-sleeps pre hibernate
  watchdog-sleeps post hibernate

EXIT
  0   always
EOF
  ;;
pre)
  case "${SYSTEMD_SLEEP_ACTION:-${2:-}}" in
  hibernate | hybrid-sleep) ;;
  *) exit 0 ;;
  esac
  timeout=$("$busctl" get-property "${manager[@]}" RuntimeWatchdogUSec) || exit 0
  timeout=${timeout#t }
  [ "$timeout" -gt 0 ] 2>/dev/null || exit 0
  touch "$kept"
  "$busctl" set-property "${manager[@]}" RuntimeWatchdogUSec t 0 ||
    echo "couldn't stop the runtime watchdog before the sleep" >&2
  ;;
post)
  [ -f "$kept" ] || exit 0
  rm -f "$kept"
  # The largest number is systemd's word for its own setting.
  "$busctl" set-property "${manager[@]}" RuntimeWatchdogUSec t 18446744073709551615 ||
    echo "couldn't start the runtime watchdog again after the sleep" >&2
  ;;
# systemd-sleep says only pre or post, so there's nothing else to do.
*) ;;
esac
exit 0
