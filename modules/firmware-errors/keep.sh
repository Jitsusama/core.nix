# shellcheck shell=bash # writeShellApplication gives it bash.
# Copies each table the firmware gave this boot about the last one's end
# into the boot's own folder under the service's state folder, as
# nixos.nix says why. A table the firmware didn't give is left out, since a
# VM or an Apple machine has none; a table it gave that can't be copied
# fails the unit, so a lost record is seen, but only once the others are
# copied, since the health table can name the reset's reason on its own.
# Each copy is flushed before the next, so a second power-off soon after
# the boot still finds the first's. Its help takes the shape every command
# of ours does (docs/commands.md), by hand.

case "${1:-}" in
-h | --help)
  cat <<'EOF'
firmware-errors-keep  keep this boot's firmware errors

  Copies the boot error record, its region and the
  platform health table the firmware gave this boot
  into the boot's own folder under STATE_DIRECTORY,
  readable by its group. A table not given is skipped.

EXAMPLES
  STATE_DIRECTORY=/var/lib/firmware-errors \
    firmware-errors-keep

EXIT
  0   each table given was kept
  1   a table given couldn't be copied
EOF
  exit 0
  ;;
*) ;;
esac

tables=/sys/firmware/acpi/tables
boot=$(tr -d - </proc/sys/kernel/random/boot_id)
folder="${STATE_DIRECTORY:?systemd sets it}/$boot"

keep() {
  local from="$tables/$1" name="$2"
  [[ -e $from ]] || return 0
  install -D -m 0640 "$from" "$folder/$name" && sync "$folder/$name"
}

lost=0
keep data/BERT bert-region || lost=1
keep BERT bert-table || lost=1
keep PHAT phat-table || lost=1
exit "$lost"
