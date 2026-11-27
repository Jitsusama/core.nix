# shellcheck shell=bash
# Runs the sweep's room check, the program it's given, over readings of
# the machine written here, and passes when it lets a sweep start at once
# on a quiet machine or one it can't read, and on one stalled on memory or
# I/O or short of memory waits, then lets it start as soon as the stall
# clears, each time ending 0 and saying why. A wait that runs out skips
# the hour, ending 1, unless the run at the next top of the hour might
# find the last sweep too long ago to wait on, when it starts as before.
set -u
program=$1
here=$(mktemp -d)
failed=0
# Only a unit's run marks itself so, and the cases below run none.
unset INVOCATION_ID

# Set the local clock, by its offset from UTC, `$1` seconds short of the
# top of an hour, now. A POSIX offset needs no zone files, and the
# stamp's age, by seconds since the epoch, stays as it is.
clock() {
  local offset
  offset=$((($(date +%s) + $1) % 3600))
  TZ=$(printf 'TST+00:%02d:%02d' $((offset / 60)) $((offset % 60)))
  export TZ
}

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
# as `$4`, and against ending `$5`, 0 if none is given.
judge() {
  echo "$4: $1 (ended $2)"
  if [ "$2" -ne "${5:-0}" ]; then
    echo "on $4 the room check ended $2, not ${5:-0}" >&2
    failed=1
  fi
  if [ "$1" != "$3" ]; then
    printf 'on %s the room check said\n  %s\nnot\n  %s\n' "$4" "$1" "$3" >&2
    failed=1
  fi
}

# Expect the check, waiting at most `$1` seconds, a second apart, to end
# `$4`, or 0, and say `$2`, as `$3`, with a sweep that ended a minute ago.
# A wait its clock ticks past during its last reading says a second more.
expect() {
  local said status
  touch -d '1 minute ago' "$here/stamp"
  said=$("$program" "$here" "$here/meminfo" "$1" 1 "$here/stamp")
  status=$?
  said=${said//"waiting $(($1 + 1)) s"/"waiting $1 s"}
  judge "$said" "$status" "$2" "$3" "${4:-0}"
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
expect 0 "pi-sweep skips this hour without room after waiting 0 s: $memory" \
  "memory stalled past its mark" 1
io="memory stalled 0.00%, I/O stalled 1.41% over five minutes, 62% of memory free"
machine 0.00 1.41 40000000 64000000
expect 0 "pi-sweep skips this hour without room after waiting 0 s: $io" \
  "I/O stalled past its mark" 1
short="memory stalled 0.00%, I/O stalled 0.00% over five minutes, 24% of memory free"
machine 0.00 0.00 15990000 64000000
expect 0 "pi-sweep skips this hour without room after waiting 0 s: $short" \
  "memory a point short" 1
none="memory stalled 0.00%, I/O stalled 0.00% over five minutes, 0% of memory free"
machine 0.00 0.00 0 64000000
expect 0 "pi-sweep skips this hour without room after waiting 0 s: $none" \
  "no memory available" 1
machine 3.51 0.00 40000000 64000000
expect 2 "pi-sweep waits for room: $memory $marks
pi-sweep skips this hour without room after waiting 2 s: $memory" \
  "a stall that outlasts the wait" 1

# A stalled machine whose last sweep ended too long ago for a wait, or
# that has no stamp, starts the sweep at once. A wait of 15 minutes a
# second apart ends two minutes and a second short of two hours, so a
# stamp 104 minutes old is already too old.
machine 3.51 0.00 40000000 64000000
touch -d '104 minutes ago' "$here/stamp"
said=$("$program" "$here" "$here/meminfo" 900 1 "$here/stamp")
judge "$said" "$?" "pi-sweep starts: the last sweep ended 104 minutes ago, too long to wait" \
  "a stamp a wait would outlast"
said=$("$program" "$here" "$here/meminfo" 900 1 "$here/none")
judge "$said" "$?" \
  "pi-sweep starts: no sweep has left a stamp it can read, so the hosts sweep meanwhile" \
  "no stamp"
# A stamp ahead of the clock can't say how long the hosts will wait.
touch -d '10 minutes' "$here/stamp"
said=$("$program" "$here" "$here/meminfo" 0 1 "$here/stamp")
judge "$said" "$?" \
  "pi-sweep starts: no sweep has left a stamp it can read, so the hosts sweep meanwhile" \
  "a stamp ahead of the clock"
# Its last pass counts too: readings 30 s apart over a minute's wait end
# 3 minutes 30 s short of two hours, so a stamp 6995 s old, which a
# check a second apart would still wait on, is too old.
touch -d '6995 seconds ago' "$here/stamp"
said=$("$program" "$here" "$here/meminfo" 60 30 "$here/stamp")
judge "$said" "$?" "pi-sweep starts: the last sweep ended 116 minutes ago, too long to wait" \
  "a stamp its last pass would outlast"
# A wait that runs out skips the hour when the run at the next top of the
# hour finds the stamp young enough to start: a second apart, under
# 7079 s, two minutes and a second short of two hours. A stamp's age and
# the time to the hour add to the same sum as a wait goes on, so each
# case below sets the clock with a few seconds either side of its mark.
# A stamp an hour old, on a run started by hand 200 s before the top of
# the hour, skips: that run finds it 3800 s old.
clock 200
touch -d '60 minutes ago' "$here/stamp"
said=$("$program" "$here" "$here/meminfo" 2 1 "$here/stamp")
status=$?
said=${said//"waiting 3 s"/"waiting 2 s"}
judge "$said" "$status" "pi-sweep waits for room: $memory $marks
pi-sweep skips this hour without room after waiting 2 s: $memory" \
  "a stamp the next top of the hour starts on" 1
# The same stamp at a run 3500 s before the hour starts: that run would
# find it 7100 s old, too late to start before the hosts.
clock 3500
touch -d '60 minutes ago' "$here/stamp"
said=$("$program" "$here" "$here/meminfo" 2 1 "$here/stamp")
status=$?
said=${said//"waiting 3 s"/"waiting 2 s"}
judge "$said" "$status" "pi-sweep waits for room: $memory $marks
pi-sweep starts without room after waiting 2 s: $memory" \
  "a stamp the next top of the hour finds too old"
# A wait that spans the top of the hour lets that hour's timer pass while
# it runs, so the next run is the hour after: a ten-second wait begun 5 s
# before the hour, on a stamp 3476 s old, starts, as that run would find
# it 7081 s old, though an hour after it began the stamp would be 7076.
clock 5
touch -d '3476 seconds ago' "$here/stamp"
said=$("$program" "$here" "$here/meminfo" 10 1 "$here/stamp")
status=$?
said=${said//"waiting 11 s"/"waiting 10 s"}
judge "$said" "$status" "pi-sweep waits for room: $memory $marks
pi-sweep starts without room after waiting 10 s: $memory" \
  "a wait that spans the top of the hour"
unset TZ
# A wait that can't say what it did lets the sweep start rather than skip.
touch -d '1 minute ago' "$here/stamp"
"$program" "$here" "$here/meminfo" 0 1 "$here/stamp" >&- 2> /dev/null
judge "" "$?" "" "a wait that can't write"
# And so does one that fails in bash itself, on a count that isn't one.
"$program" "$here" "$here/meminfo" 0 x "$here/stamp" > /dev/null 2>&1
judge "" "$?" "" "a wait that fails in bash"
# Being asked wrongly still says so, and under a unit lets the sweep
# start rather than skip every hour unheard.
"$program" "$here" > /dev/null 2>&1
judge "" "$?" "" "a wait asked wrongly" 64
INVOCATION_ID=0 "$program" "$here" > /dev/null 2> "$here/said"
judge "" "$?" "" "a unit's wait asked wrongly"
if ! grep -q 'pi-sweep-room takes' "$here/said"; then
  echo "a unit's wait asked wrongly didn't say so" >&2
  failed=1
fi

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
# Bash counts its seconds by the wall clock's, so a wait read a moment
# past its third reading may count a second more.
case $said in
"$waited"[234]" s for room: $quiet")
  judge "$said" "$status" "$said" "a stall that clears"
  ;;
*) judge "$said" "$status" "a wait of 2 to 4 s, then a start on the quiet reading" \
  "a stall that clears" ;;
esac
if [ "$SECONDS" -gt 10 ]; then
  echo "a stall that cleared still held the sweep $SECONDS more seconds" >&2
  failed=1
fi

# A stamp that ages past the wait's end during it, as when the machine
# sleeps, starts the sweep at the next reading, not when the wait runs out.
machine 3.51 0.00 40000000 64000000
touch -d '1 minute ago' "$here/stamp"
"$program" "$here" "$here/meminfo" 60 1 "$here/stamp" > "$here/said" &
waiting=$!
sleep 2
touch -d '118 minutes ago' "$here/stamp"
SECONDS=0
wait "$waiting"
status=$?
said=$(cat "$here/said")
judge "$said" "$status" "pi-sweep waits for room: $memory $marks
pi-sweep starts without room: the last sweep ended 118 minutes ago, too long to wait more: \
$memory" "a stamp that ages during a wait"
if [ "$SECONDS" -gt 10 ]; then
  echo "a stamp too old to wait for still held the sweep $SECONDS more seconds" >&2
  failed=1
fi

# A stamp that ages past the wait's end during its last sleep starts the
# sweep, read as it is then, rather than skipping by the age it had. A
# one-second wait sleeps once, between its two readings, so the stamp
# ages once it says it waits; one whose clock ticked a second before its
# first reading ends at once, unslept, so it is run again.
machine 3.51 0.00 40000000 64000000
for _ in 1 2; do
  touch -d '1 minute ago' "$here/stamp"
  : > "$here/said"
  "$program" "$here" "$here/meminfo" 1 1 "$here/stamp" > "$here/said" &
  waiting=$!
  for _ in $(seq 100); do
    ! grep -q 'waits for room' "$here/said" || break
    kill -0 "$waiting" 2> /dev/null || break
    sleep 0.01
  done
  touch -d '118 minutes ago' "$here/stamp"
  wait "$waiting"
  status=$?
  said=$(cat "$here/said")
  case $said in "pi-sweep waits for room: "*) break ;; *) ;; esac
done
judge "$said" "$status" "pi-sweep waits for room: $memory $marks
pi-sweep starts without room: the last sweep ended 118 minutes ago, too long to wait more: \
$memory" "a stamp that ages in the last sleep"

# A stamp that goes during a wait starts the sweep at the next reading.
touch -d '1 minute ago' "$here/stamp"
"$program" "$here" "$here/meminfo" 60 1 "$here/stamp" > "$here/said" &
waiting=$!
sleep 2
rm "$here/stamp"
SECONDS=0
wait "$waiting"
status=$?
said=$(cat "$here/said")
judge "$said" "$status" "pi-sweep waits for room: $memory $marks
pi-sweep starts without room: the last sweep's stamp can't be read: $memory" \
  "a stamp that goes during a wait"
if [ "$SECONDS" -gt 10 ]; then
  echo "a stamp that went still held the sweep $SECONDS more seconds" >&2
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
