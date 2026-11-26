# shellcheck shell=bash
# Runs the sweep's room check, the program it's given, over readings of
# the machine written here, and passes when it lets a sweep start at once
# on a quiet machine or one it can't read, and on one stalled on memory or
# I/O or short of memory waits, then lets it start as soon as the stall
# clears or its wait runs out, each time ending 0 and saying why.
set -u
program=$1
here=$(mktemp -d)
failed=0

# Write the machine's readings: memory and I/O stalled for the last five
# minutes, and memory available and in all, in kB.
machine() {
  printf 'some avg10=0.00 avg60=0.00 avg300=0.00 total=0\n' > "$here/memory.new"
  printf 'full avg10=0.00 avg60=0.00 avg300=%s total=0\n' "$1" >> "$here/memory.new"
  printf 'some avg10=0.00 avg60=0.00 avg300=0.00 total=0\n' > "$here/io.new"
  printf 'full avg10=0.00 avg60=0.00 avg300=%s total=0\n' "$2" >> "$here/io.new"
  printf 'MemTotal: %s kB\nMemFree: 1 kB\nMemAvailable: %s kB\n' "$4" "$3" > "$here/meminfo.new"
  mv "$here/memory.new" "$here/memory"
  mv "$here/io.new" "$here/io"
  mv "$here/meminfo.new" "$here/meminfo"
}

# Judge what the check said, `$1`, and how it ended, `$2`, against `$3`,
# as `$4`.
judge() {
  echo "$4: $1 (ended $2)"
  if [ "$2" -ne 0 ]; then
    echo "on $4 the room check ended $2, not 0" >&2
    failed=1
  fi
  if [ "$1" != "$3" ]; then
    printf 'on %s the room check said\n  %s\nnot\n  %s\n' "$4" "$1" "$3" >&2
    failed=1
  fi
}

# Expect the check, waiting at most `$1` seconds, a second apart, to end 0
# and say `$2`, as `$3`, with a sweep that ended a minute ago. A wait
# its clock ticks past during its last reading says a second more.
expect() {
  local said status
  touch -d '1 minute ago' "$here/stamp"
  said=$("$program" "$here" "$here/meminfo" "$1" 1 "$here/stamp")
  status=$?
  said=${said//"waiting $(($1 + 1)) s"/"waiting $1 s"}
  judge "$said" "$status" "$2" "$3"
}

quiet="memory stalled 0.00%, I/O stalled 0.00% over five minutes, 62% of memory free"
marks="(it starts at 3.5%, 1.4% and 25%)"

machine 0.00 0.00 40000000 64000000
expect 0 "pi-sweep starts: $quiet" "a quiet machine"
machine 3.50 1.40 16000000 64000000
expect 0 "pi-sweep starts: memory stalled 3.50%, I/O stalled 1.40% over five minutes, \
25% of memory free" "a machine at each mark"

memory="memory stalled 3.51%, I/O stalled 0.00% over five minutes, 62% of memory free"
machine 3.51 0.00 40000000 64000000
expect 0 "pi-sweep starts without room after waiting 0 s: $memory" \
  "memory stalled past its mark"
io="memory stalled 0.00%, I/O stalled 1.41% over five minutes, 62% of memory free"
machine 0.00 1.41 40000000 64000000
expect 0 "pi-sweep starts without room after waiting 0 s: $io" "I/O stalled past its mark"
short="memory stalled 0.00%, I/O stalled 0.00% over five minutes, 24% of memory free"
machine 0.00 0.00 15990000 64000000
expect 0 "pi-sweep starts without room after waiting 0 s: $short" "memory a point short"
none="memory stalled 0.00%, I/O stalled 0.00% over five minutes, 0% of memory free"
machine 0.00 0.00 0 64000000
expect 0 "pi-sweep starts without room after waiting 0 s: $none" "no memory available"
machine 3.51 0.00 40000000 64000000
expect 2 "pi-sweep waits for room: $memory $marks
pi-sweep starts without room after waiting 2 s: $memory" "a stall that outlasts the wait"

# A stalled machine whose last sweep ended too long ago for a wait, or
# that has no stamp, starts the sweep at once.
machine 3.51 0.00 40000000 64000000
touch -d '106 minutes ago' "$here/stamp"
said=$("$program" "$here" "$here/meminfo" 900 1 "$here/stamp")
judge "$said" "$?" "pi-sweep starts: the last sweep ended 106 minutes ago, too long to wait" \
  "a stamp a wait would outlast"
said=$("$program" "$here" "$here/meminfo" 900 1 "$here/none")
judge "$said" "$?" "pi-sweep starts: no sweep has left a stamp, so the hosts sweep meanwhile" \
  "no stamp"

# A stall that clears two seconds into a wait of up to a minute starts the
# sweep then, not when the wait runs out.
machine 3.51 0.00 40000000 64000000
touch "$here/stamp"
"$program" "$here" "$here/meminfo" 60 1 "$here/stamp" > "$here/said" &
waiting=$!
sleep 2
machine 0.00 0.00 40000000 64000000
SECONDS=0
wait "$waiting"
status=$?
said=$(cat "$here/said")
waited="pi-sweep waits for room: $memory $marks"$'\n'"pi-sweep starts after waiting "
case $said in
"$waited"[23]" s for room: $quiet")
  judge "$said" "$status" "$said" "a stall that clears"
  ;;
*) judge "$said" "$status" "a wait of 2 or 3 s, then a start on the quiet reading" \
  "a stall that clears" ;;
esac
if [ "$SECONDS" -gt 10 ]; then
  echo "a stall that cleared still held the sweep $SECONDS more seconds" >&2
  failed=1
fi

for gone in memory io meminfo; do
  machine 0.00 0.00 40000000 64000000
  rm "$here/$gone"
  case $gone in
  memory) unread="memory pressure" ;;
  io) unread="I/O pressure" ;;
  *) unread="available memory, memory in all" ;;
  esac
  expect 0 "pi-sweep starts: the machine's $unread couldn't be read" \
    "a machine whose $gone can't be read"
done
machine 0.00 0.00 40000000 64000000
printf 'some avg10=0.00 avg60=0.00 avg300=0.00 total=0\n' > "$here/io"
expect 0 "pi-sweep starts: the machine's I/O pressure couldn't be read" \
  "an I/O reading with no full line"
machine 0.00 0.00 40000000 0
expect 0 "pi-sweep starts: the machine's memory in all couldn't be read" \
  "a machine with no memory in all"
machine 0.00 x 40000000 64000000
expect 0 "pi-sweep starts: the machine's I/O pressure couldn't be read" \
  "an I/O stall that isn't a number"
# Bash reads a value in arithmetic as an expression, so a reading that
# isn't a number must never get that far.
machine 0.00 0.00 'a[1]' 64000000
expect 0 "pi-sweep starts: the machine's available memory couldn't be read" \
  "memory that isn't a number"
rm -r "$here"
exit "$failed"
