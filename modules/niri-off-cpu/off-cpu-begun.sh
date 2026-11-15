# shellcheck shell=bash
# Returns once an account's trace has begun: once bpftrace, having put every
# probe in place, has written its header to the trace (off-cpu.bt's BEGIN),
# and to the count of restores where there is one (restores.bt's). A count
# that ended without beginning is said in the journal, and the trace goes
# on without it.
# Attaching takes seconds on a busy machine, and the asker times its window
# from when the unit has started, so the unit waits for this first
# (nixos.nix). It fails as soon as the trace's own process, systemd's
# MAINPID, has ended without beginning. It waits on writes in the trace's
# folder, looking again every five seconds for one made between a look and
# the wait.
# Its help and failures take the shape every command of ours does
# (docs/commands.md), by hand.

case "${1:-}" in
-h | --help)
  cat << 'EOF'
niri-off-cpu-begun  wait until an account's trace has begun

  niri-off-cpu@<account>.service runs it once the trace
  has started, so the unit counts as started only once
  every probe is in place.

EXAMPLES
  niri-off-cpu-begun jitsusama

EXIT
  0   the trace has begun
  1   the trace ended before it began
  64  asked wrongly
EOF
  exit 0
  ;;
*) ;;
esac

if [ "$#" -ne 1 ] || [ -z "$1" ]; then
  printf '%s\n' "niri-off-cpu-begun takes the account whose trace to wait for" \
    "  try  niri-off-cpu-begun --help" >&2
  exit 64
fi

out=/run/niri-off-cpu/$1.txt
restores=/run/niri-off-cpu/$1-restores.txt
counting=/run/niri-off-cpu/$1-restores.pid
# As off-cpu-as-root.sh decides whether to count them, which it says by
# writing the count's process.
begun() {
  [ -s "$out" ] || return 1
  [ -e /sys/kernel/btf/ttm ] || return 0
  [ -s "$restores" ] && return 0
  [ -s "$counting" ] && kill -0 "$(cat "$counting")" 2> /dev/null && return 1
  # Not written yet, or it ended without beginning.
  [ -s "$counting" ]
}
until begun; do
  if ! kill -0 "$MAINPID" 2> /dev/null; then
    echo "the trace ended before it began" >&2
    exit 1
  fi
  inotifywait -q -q -t 5 -e create -e modify -e close_write "${out%/*}" || true
done
