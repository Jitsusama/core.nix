# shellcheck shell=bash
# Saves a backlight's level as systemd-backlight does when it stops at
# shutdown, except the screen's while a lid is shut. The panel is off then,
# and the level it reads is the dark one it was left at (169 of 65535 on
# 2026-10-26), which the next boot comes up at. So the level saved while the
# lid was open stays. Any other backlight, such as a keyboard's, is saved as
# it reads.
# usage: saves-while-open <instance>, as the unit gives it, with $save naming
# systemd-backlight and $LIDS the lids' folder (/proc/acpi/button/lid).

instance=$1
if [[ $instance == backlight:* ]]; then
  for state in "${LIDS:-/proc/acpi/button/lid}"/*/state; do
    [[ -r $state ]] || continue
    if grep -q closed "$state"; then
      echo "A lid is shut, so $instance keeps the level saved while it was open."
      exit 0
    fi
  done
fi
exec "$save" save "$instance"
