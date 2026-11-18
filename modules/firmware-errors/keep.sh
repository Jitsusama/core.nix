# shellcheck shell=bash # writeShellApplication gives it bash.
# Copies each table the firmware gave this boot about the last one's end
# into the boot's own folder under the service's state folder, as
# nixos.nix says why. A table the firmware didn't give is left out, since a
# VM or an Apple machine has none; a table it gave that can't be copied
# fails the unit, so a lost record is seen. Each copy is flushed before the
# next, so a second power-off soon after the boot still finds the first's.
#
#   firmware-errors-keep   with STATE_DIRECTORY set, as systemd sets it

tables=/sys/firmware/acpi/tables
boot=$(tr -d - </proc/sys/kernel/random/boot_id)
folder="$STATE_DIRECTORY/$boot"

keep() {
  local from="$tables/$1" name="$2"
  [[ -e $from ]] || return 0
  install -D -m 0640 "$from" "$folder/$name"
  sync "$folder/$name"
}

keep data/BERT bert-region
keep BERT bert-table
keep PHAT phat-table
