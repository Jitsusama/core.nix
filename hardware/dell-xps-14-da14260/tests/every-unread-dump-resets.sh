# shellcheck shell=bash
# Checks that btintel_pcie's read of an exception dump has no way out but
# the label that raises the Hardware Error when nothing was passed on, so
# a dump it can't read still resets the chip. A return left in the read,
# as a newer kernel's early one for a bad address or length would be once
# the patch is carried onto it, skips the error as surely as no call.
# usage: every-unread-dump-resets.sh <btintel_pcie.c>
set -euo pipefail

read=$(awk '/^static void btintel_pcie_read_hwexp\(/,/^}$/' "$1")
if [ -z "$read" ]; then
  echo "btintel_pcie has no btintel_pcie_read_hwexp to hold."
  exit 1
fi
if grep -qw 'return' <<<"$read"; then
  echo "A return in btintel_pcie_read_hwexp skips the Hardware Error, so its chip stays halted."
  exit 1
fi
if ! sed -n '/^exit_on_error:$/,$p' <<<"$read" |
  grep -q '^[[:space:]]*btintel_pcie_hwexp_unread(data);$'; then
  echo "Bluetooth's chip is left halted when its exception dump can't be read."
  exit 1
fi
