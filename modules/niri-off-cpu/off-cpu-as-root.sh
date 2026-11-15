# shellcheck shell=bash
# Traces an account's niri, its session's compositor, off the CPU
# (off-cpu.bt) until the unit running it is stopped, into a file in
# /run/niri-off-cpu named for the account, which only the account and root
# can read. Beside it, the graphics buffers reclaim backed up and read back
# meanwhile (restores.bt), into a second file.
# Its help and failures take the shape every command of ours does
# (docs/commands.md), by hand.

case "${1:-}" in
-h | --help)
  cat << 'EOF'
niri-off-cpu-as-root  trace an account's niri off the CPU

  niri-off-cpu@<account>.service runs it as root until
  it's stopped, writing to /run/niri-off-cpu/<account>.txt.
  People run niri-off-cpu, which starts and stops it.

EXAMPLES
  niri-off-cpu-as-root jitsusama

EXIT
  0   traced until stopped
  1   no niri session runs as the account, or the trace
      couldn't start
  64  asked wrongly
EOF
  exit 0
  ;;
*) ;;
esac

if [ "$#" -ne 1 ] || [ -z "$1" ]; then
  printf '%s\n' "niri-off-cpu-as-root takes the account whose niri to trace" \
    "  try  niri-off-cpu-as-root --help" >&2
  exit 64
fi

account=$1
id -u "$account" > /dev/null

# The oldest, since every `niri msg` the account runs is a niri too.
if ! pid=$(pgrep --oldest --uid "$account" --full '/bin/niri --session$'); then
  echo "no niri session runs as $account" >&2
  exit 1
fi

# Its buffers' types come from the graphics memory manager's own BTF, so a
# machine that hasn't loaded one, as the VM test's hasn't, has none to count.
if [ -e /sys/kernel/btf/ttm ]; then
  counts=/run/niri-off-cpu/$account-restores.txt
  install -m 0600 -o "$account" /dev/null "$counts"
  # Stopping the unit interrupts it too, and it prints its counts then.
  bpftrace -q -B line -o "$counts" "$RESTORES" &
  # So the unit waits for it to begin only while it runs (off-cpu-begun.sh).
  echo "$!" > "/run/niri-off-cpu/$account-restores.pid"
fi

out=/run/niri-off-cpu/$account.txt
install -m 0600 -o "$account" /dev/null "$out"
# A line at a time, so the header BEGIN prints once every probe is in place
# reaches the file then, and the unit can wait for it (nixos.nix).
exec bpftrace -q -B line -o "$out" "$OFF_CPU" "$pid"
