# shellcheck shell=bash
# Runs the sweep's room check, the program it's given, over readings of
# the machine written here, and passes when it lets a sweep start at once
# on a quiet machine or one it can't read, and on one stalled on memory or
# I/O or short of memory waits, then lets it start, each time ending 0 and
# saying why.
set -u
program=$1
here=$(mktemp -d)
failed=0

# Write the machine's readings: memory and I/O stalled for the last five
# minutes, and memory available and in all, in kB.
machine() {
  printf 'some avg10=0.00 avg60=0.00 avg300=0.00 total=0\n' > "$here/memory"
  printf 'full avg10=0.00 avg60=0.00 avg300=%s total=0\n' "$1" >> "$here/memory"
  printf 'some avg10=0.00 avg60=0.00 avg300=0.00 total=0\n' > "$here/io"
  printf 'full avg10=0.00 avg60=0.00 avg300=%s total=0\n' "$2" >> "$here/io"
  printf 'MemTotal: %s kB\nMemFree: 1 kB\nMemAvailable: %s kB\n' "$4" "$3" > "$here/meminfo"
}

# Expect the check, trying `$1` times before it gives up waiting, to end 0
# and say `$2`, as `$3`.
expect() {
  local said status
  said=$("$program" "$here" "$here/meminfo" "$1" 0)
  status=$?
  echo "$3: $said (ended $status)"
  if [ "$status" -ne 0 ]; then
    echo "on $3 the room check ended $status, not 0" >&2
    failed=1
  fi
  if [ "$said" != "$2" ]; then
    printf 'on %s the room check said\n  %s\nnot\n  %s\n' "$3" "$said" "$2" >&2
    failed=1
  fi
}

quiet="memory stalled 0.00%, I/O stalled 0.00% over five minutes, 62% of memory free"
unread="pi-sweep starts: the machine's pressure or memory couldn't be read"
marks="(it starts at 3.5%, 1.4% and 25%)"

machine 0.00 0.00 40000000 64000000
expect 0 "pi-sweep starts: $quiet" "a quiet machine"
machine 3.50 1.40 16000000 64000000
expect 0 "pi-sweep starts: memory stalled 3.50%, I/O stalled 1.40% over five minutes, \
25% of memory free" "a machine at each mark"

memory="memory stalled 3.51%, I/O stalled 0.00% over five minutes, 62% of memory free"
machine 3.51 0.00 40000000 64000000
expect 0 "pi-sweep starts without room after waiting 0 minutes: $memory" \
  "memory stalled past its mark"
io="memory stalled 0.00%, I/O stalled 1.41% over five minutes, 62% of memory free"
machine 0.00 1.41 40000000 64000000
expect 0 "pi-sweep starts without room after waiting 0 minutes: $io" "I/O stalled past its mark"
short="memory stalled 0.00%, I/O stalled 0.00% over five minutes, 23% of memory free"
machine 0.00 0.00 15000000 64000000
expect 0 "pi-sweep starts without room after waiting 0 minutes: $short" "memory short"
machine 3.51 0.00 40000000 64000000
expect 2 "pi-sweep waits for room: $memory $marks
pi-sweep starts without room after waiting 0 minutes: $memory" "a stall that outlasts the wait"

for gone in memory io meminfo; do
  machine 0.00 0.00 40000000 64000000
  rm "$here/$gone"
  expect 0 "$unread" "a machine whose $gone can't be read"
done
machine 0.00 0.00 40000000 64000000
printf 'some avg10=0.00 avg60=0.00 avg300=0.00 total=0\n' > "$here/io"
expect 0 "$unread" "an I/O reading with no full line"
machine 0.00 0.00 40000000 0
expect 0 "$unread" "a machine with no memory in all"
machine 0.00 0.00 '$(touch hacked)' 64000000
expect 0 "$unread" "memory that isn't a number"
machine 0.00 x 40000000 64000000
expect 0 "$unread" "an I/O stall that isn't a number"
if [ -e hacked ] || [ -e "$here/hacked" ]; then
  echo "a reading ran as a command" >&2
  failed=1
fi
rm -r "$here"
exit "$failed"
