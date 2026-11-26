# shellcheck shell=bash
# Runs the sweep's room check, the program it's given, over readings of
# the machine written here, and passes when it lets a sweep start on a
# quiet machine or one it can't read, and skips the hour on one stalled
# on memory or I/O or short of memory, each time saying why.
set -u
program=$1
here=$(mktemp -d)
export SWEEP_ROOM_PRESSURE=$here SWEEP_ROOM_MEMINFO=$here/meminfo
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

# Expect the check to end `$1` and say something holding `$2`, as `$3`.
expect() {
  local said status
  said=$("$program")
  status=$?
  echo "$3: $said (ended $status)"
  if [ "$status" -ne "$1" ]; then
    echo "on $3 the room check ended $status, not $1" >&2
    failed=1
  fi
  case $said in
  *"$2"*) ;;
  *)
    echo "on $3 the room check didn't say \"$2\"" >&2
    failed=1
    ;;
  esac
}

machine 0.00 0.00 40000000 64000000
expect 0 "pi-sweep starts: memory stalled 0.00%" "a quiet machine"
machine 3.50 1.40 16000000 64000000
expect 0 "pi-sweep starts" "a machine at each mark"
machine 3.51 0.00 40000000 64000000
expect 1 "waits for the next hour: memory stalled 3.51%" "memory stalled past its mark"
machine 0.00 1.41 40000000 64000000
expect 1 "I/O stalled 1.41%" "I/O stalled past its mark"
machine 0.00 0.00 15000000 64000000
expect 1 "23% of memory free" "memory short"
rm "$here/io"
expect 0 "couldn't be read" "a machine whose I/O can't be read"
rm -r "$here"
exit "$failed"
