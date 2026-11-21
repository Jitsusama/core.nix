# shellcheck shell=bash
# Tells the recorder the machine is going to sleep, and that it woke.
# systemd-sleep runs it as root with "pre" or "post" and the sleep's name,
# last before the kernel freezes every task and first after it thaws them.
#
# The recorder writes one record saying the machine went to sleep, and
# nothing more until it hears the machine woke (src/bin/last-seconds-keep.rs),
# so a boot that never wakes, reset while asleep or on the way out of it,
# ends in that record rather than in one the reader would take for the
# second before a reset. It signals the recorder, and reads its ring only as
# the recorder's own account, so it needs nothing the recorder could misuse,
# and it never fails a sleep. Its help takes the shape every command of ours
# does (docs/commands.md), by hand.
set -u

# The time since boot in hundredths of a second, which no clock step moves
# and no locale writes with a comma; in base ten, since its first second
# reads with a leading zero.
hundredths() {
  local up
  read -r up _ < /proc/uptime
  echo "$((10#${up/./}))"
}

# This boot's newest record, as the recorder's account reads it, in JSON;
# nothing if it can't be read within half a second, killed a tenth after if
# it won't end, so a ring that blocks or never ends holds no look longer.
newest() {
  timeout -k 0.1 0.5 setpriv --reuid=last-seconds --regid=last-seconds --clear-groups \
    last-seconds "$boot" --seconds 1 --format json 2> /dev/null
}

case "${1:-}" in
-h | --help)
  cat << 'EOF'
last-seconds-sleeps  tell the recorder of a sleep

  systemd-sleep runs it with pre before a sleep, which
  has the recorder write a record saying so and hold
  off, waiting under three seconds for that record,
  and post after, which has it go on. It never fails
  a sleep.

EXAMPLES
  last-seconds-sleeps pre suspend
  last-seconds-sleeps post suspend

EXIT
  0   always
EOF
  ;;
pre)
  boot=$(tr -d - < /proc/sys/kernel/random/boot_id)
  # The kernel's freeze of every task must not come before the record. The
  # recorder writes it once the second it's in is written, a firmware read
  # under load included, so this waits until a newer record says it, and
  # gives up two seconds on, counting the look before the signal, a look
  # begun by then taking six tenths at most, so a recorder that can't write
  # never holds the sleep three seconds.
  now=$(hundredths)
  deadline=$((now + 200))
  # The record to wait for comes after the newest now, so a sleep soon
  # after a wake isn't taken for the last sleep's record.
  answer=$(newest) || answer=''
  before=$(jq -r '.seconds[-1].sequence // -1' <<< "$answer" 2> /dev/null) || before=''
  [ -n "$before" ] || before=-1
  systemctl kill --kill-whom=main --signal=SIGUSR1 last-seconds.service 2> /dev/null || exit 0
  now=$(hundredths)
  while [ "$now" -lt "$deadline" ]; do
    answer=$(newest) || answer=''
    said=$(jq -r --argjson before "$before" \
      '.seconds[-1] | select(.asleep and .sequence > $before) | .sequence' \
      <<< "$answer" 2> /dev/null) || said=''
    if [ -n "$said" ]; then
      # systemd-sleep puts what a hook says in the journal.
      echo "last-seconds-sleeps: second $said says the machine went to sleep"
      exit 0
    fi
    # Polls: it looks again until the record is there or the time is up.
    sleep 0.02
    now=$(hundredths)
  done
  echo "last-seconds-sleeps: no record said the machine went to sleep in two seconds"
  ;;
post)
  systemctl kill --kill-whom=main --signal=SIGUSR2 last-seconds.service 2> /dev/null || true
  ;;
# systemd-sleep says only pre or post, so there's nothing else to do.
*) ;;
esac
exit 0
