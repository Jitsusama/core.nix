# shellcheck shell=bash
# Checks that Bluetooth's driver thaws its chip, after a hibernation's image
# is made, with a plain wake and never a reset. A thaw that resets the chip
# leaves it in its bootloader, and the power-off after refuses it. A name in
# the driver's power table can't show that alone: the thaw must pass the
# thaw's event to the shared wake, and that wake must reset for a restore or
# a resume from sleep, never for a thaw.
# usage: thaw-is-a-plain-wake.sh <drivers/bluetooth/btintel_pcie.c>
set -euo pipefail

source=$1

# The lines of one of the driver's functions, from its first line to its
# closing brace.
body() {
  awk -v name="$1" '
    index($0, "static int " name "(") == 1 { inside = 1 }
    inside { print }
    inside && /^}$/ { exit }
  ' "$source"
}

thaw=$(sed -n 's/^[[:space:]]*\.thaw = \([a-z0-9_]*\),$/\1/p' "$source")
if [ -z "$thaw" ]; then
  echo "Bluetooth's driver no longer names its thaw where this check reads."
  exit 1
fi
thawing=$(body "$thaw")
if [ -z "$thawing" ]; then
  echo "Bluetooth's driver names $thaw as its thaw, and this check can't find it."
  exit 1
fi
if ! grep -q 'btintel_pcie_resume_event(dev, PMSG_THAW)' <<<"$thawing"; then
  echo "Bluetooth's thaw doesn't wake its chip as a thaw, so it can reset it," \
    "and a hibernation's power-off can be refused."
  exit 1
fi

waking=$(body btintel_pcie_resume_event)
if ! grep -q 'mesg\.event == PM_EVENT_RESTORE' <<<"$waking"; then
  echo "Bluetooth's wake no longer tests the event it wakes for where this check reads."
  exit 1
fi
if grep -q 'PM_EVENT_THAW' <<<"$waking"; then
  echo "Bluetooth's wake resets its chip for a thaw," \
    "so a hibernation's power-off can be refused."
  exit 1
fi
