# shellcheck shell=bash
# A foreground program for slices-vm-test.py, run with bash in app.slice: it
# waits a tenth of a second, the given number of times, and writes to the
# given file how late its worst wake was, in microseconds. Every wait is a
# builtin read on a pipe nothing writes to, so no process it starts adds to
# the lateness it measures; a machine short of memory or CPU shows in it.
# Its arguments are sleep's path, the count and the file.
set -u
# The sleep only holds the pipe open; it never ends, so its status is moot.
exec 3< <("$1" infinity || true)
worst=0
for ((i = 0; i < $2; i++)); do
  before=${EPOCHREALTIME/./}
  read -r -t 0.1 -u 3 || true
  late=$((${EPOCHREALTIME/./} - before - 100000))
  if ((late > worst)); then
    worst=$late
  fi
done
echo "$worst" > "$3.tmp"
mv "$3.tmp" "$3"
