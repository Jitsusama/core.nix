# shellcheck shell=bash
# Holds every core able to come out of rest within BOUND_US microseconds
# for as long as it runs, as deep-rest.nix says why. The kernel keeps the
# request only while the file stays open, and reads a write of exactly four
# bytes as a binary number and any other as hexadecimal text, so the bound
# goes as eight hex digits and a newline, eleven bytes whatever it is. The
# kernel's bound is then read back, and only when it holds does systemd
# hear it's ready, so a start waits for the hold and fails with it. A sleep
# then keeps the file open. It needs od, sleep and systemd-notify on PATH.
set -euo pipefail

bound=${BOUND_US:?the unit sets it}
exec 3>/dev/cpu_dma_latency
printf '0x%08x\n' "$bound" >&3
held=$(od -A n -t d4 -N 4 /dev/cpu_dma_latency)
if ((held > bound)); then
  echo "The kernel lets the cores take $held us out of rest, not $bound, so nothing is held." >&2
  exit 1
fi
systemd-notify --ready
exec sleep infinity
