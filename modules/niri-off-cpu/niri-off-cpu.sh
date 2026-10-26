# shellcheck shell=bash
# Traces your own niri's main thread for so many seconds: every time it's
# off the CPU for 8 ms or more, when, for how long, what held it and the
# kernel stack it waited in (off-cpu.bt), then says where that was written,
# after where the graphics buffers read back meanwhile were counted, on a
# machine that counts them (restores.bt).
# polkit lets an account in wheel start and stop its own trace and nobody
# else's (nixos.nix), so it asks for no password.
#
# usage: niri-off-cpu <seconds>

if [ "$#" -ne 1 ] || ! [[ $1 =~ ^[1-9][0-9]*$ ]] || [ "$1" -gt 600 ]; then
  echo "usage: niri-off-cpu <seconds, 1 to 600>" >&2
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
