# shellcheck shell=bash # The check runs it with bash.
# Runs the sleep hook, the path named first, as writeShellApplication runs
# it, against stand-ins of systemctl, setpriv and the reader, and fails
# saying what it did wrong: the check last-seconds-sleeps-wait-for-the-record,
# in checks.nix. The path named second is the PATH of what else it uses. It
# works in the folder it's run in.
set -euo pipefail

hook=$1
tools=$2
here=$PWD

# How long a sleep let go at once may take, in hundredths: well under the
# two seconds the hook would wait, with room for a loaded builder's slow
# process starts.
at_once=150

# Fails saying what the sleep's hook did wrong.
hook_wrongly() {
  echo "The hook $1." >&2
  exit 1
}

mkdir -p bin
# The reader's stand-in answers with ./answer, as the boot's newest record.
reader() {
  cat > bin/last-seconds << EOF
#!$BASH
cat "$here/answer"
EOF
  chmod +x bin/last-seconds
}
reader
# setpriv's drops its options and runs the rest as it was.
cat > bin/setpriv << EOF
#!$BASH
while [ "\${1#--}" != "\$1" ]; do shift; done
exec "\$@"
EOF
# systemctl's fails where ./absent says no recorder runs, logs the signal it
# was asked to send, and has ./next become the newest record, as the
# recorder would write it.
cat > bin/systemctl << EOF
#!$BASH
[ ! -e "$here/absent" ] || exit 1
echo "\$3" >> "$here/signals"
[ ! -e "$here/next" ] || cp "$here/next" "$here/answer"
EOF
chmod +x bin/*

# A record as the reader gives it: its sequence and whether it's asleep.
record() { echo "{\"seconds\":[{\"sequence\":$1,\"asleep\":$2}]}"; }

# The time since boot in hundredths, to say how long the hook held a sleep.
hundredths() {
  local up
  read -r up _ < /proc/uptime
  echo "$((10#${up/./}))"
}

# Runs the hook with the stand-ins first, giving what it said and taking
# how long it took into ./took.
run() {
  local began ended
  began=$(hundredths)
  PATH=$here/bin:$tools "$BASH" -euo pipefail "$hook" "$@"
  ended=$(hundredths)
  echo $((ended - began)) > took
}

# The recorder writes its record when signalled: the hook says it saw it
# and lets the sleep go at once.
record 41 false > answer
record 42 true > next
said=$(run pre suspend)
[[ $said == *"second 42 says the machine went to sleep"* ]] ||
  hook_wrongly "didn't see the recorder's record: $said"
(($(< took) < at_once)) || hook_wrongly "held a sleep $(< took) hundredths with its record there"
[ "$(< signals)" = --signal=SIGUSR1 ] || hook_wrongly "sent $(< signals) before a sleep"

# A newer record comes that doesn't say asleep, an ordinary second written
# before the recorder heard: the hook keeps waiting, and gives up at two
# seconds.
rm signals
record 41 false > answer
record 42 false > next
said=$(run pre suspend)
[[ $said == *"no record said the machine went to sleep"* ]] ||
  hook_wrongly "let a sleep go on a record that didn't say asleep: $said"

# The newest record is the last sleep's, and nothing newer comes: the hook
# keeps waiting rather than taking it, and gives up at two seconds.
rm next signals
record 42 true > answer
said=$(run pre suspend)
[[ $said == *"no record said the machine went to sleep"* ]] ||
  hook_wrongly "took the last sleep's record for this one's: $said"
(($(< took) >= 200 && $(< took) < 300)) ||
  hook_wrongly "gave up after $(< took) hundredths, not two seconds"

# A reader that never ends, ignoring the signal to stop, holds no look past
# six tenths, the look before the signal included, so the hook still gives
# up within three seconds.
cat > bin/last-seconds << EOF
#!$BASH
trap '' TERM
# A reader that hangs, for the hook to give up on.
exec sleep 30
EOF
said=$(run pre suspend)
[[ $said == *"no record said"* ]] || hook_wrongly "didn't give up on a reader that hung: $said"
(($(< took) < 300)) || hook_wrongly "held a sleep $(< took) hundredths on a reader that hung"

# After the sleep, the recorder is told it woke.
rm signals
run post suspend > /dev/null
[ "$(< signals)" = --signal=SIGUSR2 ] || hook_wrongly "sent $(< signals) after a sleep"

# No recorder running: the hook lets the sleep go at once, saying nothing.
reader
touch absent
said=$(run pre suspend)
[ -z "$said" ] || hook_wrongly "said something with no recorder running: $said"
(($(< took) < at_once)) ||
  hook_wrongly "held a sleep $(< took) hundredths with no recorder running"
