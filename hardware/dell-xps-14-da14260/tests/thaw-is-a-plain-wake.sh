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

# The condition of the first if, at the wake's own level, whose block resets
# the chip, joined onto one line.
waking=$(body btintel_pcie_resume_event)
condition=$(awk '
  /^\tif \(/ { reading = 1; condition = "" }
  reading { condition = condition " " $0 }
  reading && /\{$/ { reading = 0; inside = 1; next }
  reading && /;$/ { reading = 0; next }
  inside && /btintel_pcie_request_reset\(/ { print condition; exit }
  inside && /^\t}$/ { inside = 0 }
' <<<"$waking")
if [ -z "$condition" ]; then
  echo "Bluetooth's wake no longer resets its chip where this check reads."
  exit 1
fi
condition=$(tr -s '[:space:]' ' ' <<<"$condition")

# A reset at the wake's own level, under no test, is taken on every wake.
if grep -q $'^\tbtintel_pcie_request_reset(' <<<"$waking"; then
  echo "Bluetooth's wake resets its chip on every wake, so it resets it for a thaw," \
    "and a hibernation's power-off can be refused."
  exit 1
fi

# A thaw always follows a freeze, so a reset that reads anything but the
# restore event, or a resume from sleep held in memory, can be taken on a
# thaw: the freeze before it, a stored record of that sleep, or a test that
# no event but one is in hand. Each comparison it may make is struck out, and
# anything left over is something else deciding the reset.
left=$(sed -E \
  -e 's/mesg\.event == PM_EVENT_(RESTORE|RESUME)//g' \
  -e 's/pm_suspend_target_state == PM_SUSPEND_MEM//g' \
  -e 's/\bif\b|[][(){}|&[:space:]]//g' <<<"$condition")
if ! grep -q 'mesg\.event == PM_EVENT_RESTORE' <<<"$condition" || [ -n "$left" ]; then
  echo "Bluetooth's wake resets its chip on more than a restore or a resume from" \
    "sleep, so it can reset it for a thaw, and a hibernation's power-off can be" \
    "refused. It resets when:${condition%\{ }"
  exit 1
fi
