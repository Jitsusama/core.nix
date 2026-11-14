# shellcheck shell=bash
# Traces your own niri's main thread for so many seconds: every time it's
# off the CPU for 8 ms or more, when, for how long, what held it and the
# kernel stack it waited in (off-cpu.bt), then says where that was written,
# after where the graphics buffers read back meanwhile were counted, on a
# machine that counts them (restores.bt).
# polkit lets an account in wheel start and stop its own trace and nobody
# else's (nixos.nix), so it asks for no password. Its help and failures
# take the shape every command of ours does (docs/commands.md), by hand.

case "${1:-}" in
-h | --help)
  cat << 'EOF'
niri-off-cpu  trace when your niri waits off the CPU

  Each wait of 8 ms or more is written with what held it
  and the kernel stack it waited in; the file's path is
  the last line said.

EXAMPLES
  niri-off-cpu 30   trace for 30 seconds
  niri-off-cpu 600  the longest it traces, ten minutes

EXIT
  0   traced
  1   the trace couldn't start, and the journal says why
  64  asked wrongly
EOF
  exit 0
  ;;
esac

if [ "$#" -ne 1 ] || ! [[ $1 =~ ^[1-9][0-9]*$ ]] || [ "$1" -gt 600 ]; then
  printf '%s\n' "niri-off-cpu takes the seconds to trace, 1 to 600" \
    "  try  niri-off-cpu 30" >&2
  exit 64
fi

account=$(id -un)
unit="niri-off-cpu@$account.service"

if ! systemctl start "$unit"; then
  journalctl --unit "$unit" --invocation=0 --no-pager --output=cat >&2 || true
  exit 1
fi
# The trace is the window being measured, so it's waited out whole.
sleep "$1"
systemctl stop "$unit"
restores=/run/niri-off-cpu/$account-restores.txt
if [ -e /sys/kernel/btf/ttm ] && [ -s "$restores" ]; then
  echo "$restores"
fi
echo "/run/niri-off-cpu/$account.txt"
