# shellcheck shell=bash
# Returns once an account's trace has begun: once bpftrace, having put every
# probe in place, has written its header to the trace (off-cpu.bt's BEGIN).
# Attaching takes seconds on a busy machine, and the asker times its window
# from when the unit has started, so the unit waits for this first
# (nixos.nix). It fails as soon as the trace's own process, systemd's
# MAINPID, has ended without beginning. It waits on writes in the trace's
# folder, looking again every five seconds for one made between a look and
# the wait.
#
# usage: niri-off-cpu-begun <account>

out=/run/niri-off-cpu/$1.txt
until [ -s "$out" ]; do
  if ! kill -0 "$MAINPID" 2> /dev/null; then
    echo "the trace ended before it began" >&2
    exit 1
  fi
  inotifywait -q -q -t 5 -e create -e modify -e close_write "${out%/*}" || true
done
