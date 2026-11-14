# shellcheck shell=bash disable=SC2154 # $save comes from saves-while-open.nix's header.
# Saves a backlight's level as systemd-backlight does when it stops at
# shutdown, except the screen's while a lid is shut. The panel is off then,
# and the level it reads is the dark one it was left at (169 of 65535 on
# 2026-10-26), which the next boot comes up at. So the level saved while the
# lid was open stays. Any other backlight, such as a keyboard's, is saved as
# it reads.
# It takes the instance as the unit gives it, with $save naming
# systemd-backlight and $LIDS the lids' folder (/proc/acpi/button/lid). Its
# help and failures take the shape every command of ours does
# (docs/commands.md), by hand.

case "${1:-}" in
-h | --help)
  cat << 'EOF'
saves-while-open  save a backlight's level as it stops

  As systemd-backlight does at shutdown, except the
  screen's while a lid is shut, when it reads dark: the
  level saved while the lid was open stays.

EXAMPLES
  saves-while-open backlight:intel_backlight

EXIT
  0   saved, or kept while a lid is shut
  64  asked wrongly
  else systemd-backlight's own
EOF
  exit 0
  ;;
*) ;;
esac

if [ "$#" -ne 1 ]; then
  printf '%s\n' "saves-while-open takes the backlight's instance" \
    "  try  saves-while-open --help" >&2
  exit 64
fi

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
