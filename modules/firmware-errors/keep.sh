# shellcheck shell=bash # writeShellApplication gives it bash.
# Copies each table the firmware gave this boot about the last one's end
# into the boot's own folder under the service's state folder, as
# nixos.nix says why. A table the firmware didn't give is left out, and
# where it gives none, as in a VM, nothing is kept, so no boot is listed
# that says nothing; a table it gave that can't be copied
# fails the unit, so a lost record is seen, but only once the others are
# copied, since the health table can name the reset's reason on its own.
# Each copy is flushed before the next, so a second power-off soon after
# the boot still finds the first's, and each is written under another name
# and moved into place, so a power-off midway, at boot or at a switch,
# never leaves a short copy that reads as the firmware's. Its help takes
# the shape every command of ours does (docs/commands.md), by hand.

case "${1:-}" in
-h | --help)
  cat <<'EOF'
firmware-errors-keep  keep this boot's firmware errors

  Copies the boot error record, its region and the
  platform health table the firmware gave this boot,
  the machine's serial number and when the boot
  began, into the boot's own folder under
  STATE_DIRECTORY, readable by its group. A table not
  given is skipped; with none given, nothing is kept.

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

# Where it reads, each of which the tests give it as a stand-in; the unit
# pins each to the default here.
tables=${FIRMWARE_ERRORS_TABLES:-/sys/firmware/acpi/tables}
serial=${FIRMWARE_ERRORS_SERIAL:-/sys/class/dmi/id/product_serial}
proc=${FIRMWARE_ERRORS_PROC:-/proc}
boot=$(tr -d - <"$proc/sys/kernel/random/boot_id")
folder="${STATE_DIRECTORY:?systemd sets it}/$boot"

# Moves a file written whole as NAME.new into place as NAME, once it's on
# the drive.
settle() {
  local name="$1"
  sync "$folder/$name.new" &&
    mv -f "$folder/$name.new" "$folder/$name" &&
    sync "$folder"
}

# A table doesn't change within a boot, so one already kept whole is left
# as it is, its times with it: only a copy missing or short is written.
keep() {
  local from="$1" name="$2"
  [[ -e $from ]] || return 0
  cmp -s "$from" "$folder/$name" && return 0
  mkdir -p "$folder" &&
    install -m 0640 "$from" "$folder/$name.new" &&
    settle "$name"
}

# Writes when the boot began, in seconds since 1970: the folder's own
# times can't say it, since the first run may come at a switch, not at
# boot. The kernel's btime is the clock less the time since boot, so it
# moves when the clock is set after the boot began; each run writes it
# again where it moved, so a boot that began on a clock still wrong is
# placed where it really began once a later run sees the clock right.
# Only a later run does that, such as a switch that changes the unit;
# nothing runs it again once the clock is set.
began() {
  local name value
  while read -r name value _; do
    if [[ $name == btime ]]; then
      [[ -s $folder/began && $(<"$folder/began") == "$value" ]] && return 0
      mkdir -p "$folder" &&
        echo "$value" >"$folder/began.new" &&
        settle began
      return
    fi
  done <"$proc/stat"
  return 1
}

[[ -e $tables/data/BERT || -e $tables/BERT || -e $tables/PHAT ]] || exit 0
lost=0
began || lost=1
keep "$tables/data/BERT" bert-region || lost=1
keep "$tables/BERT" bert-table || lost=1
keep "$tables/PHAT" phat-table || lost=1
# The serial number a maker's support asks for, its service tag on a Dell,
# which the kernel lets only root read.
keep "$serial" product-serial || lost=1
exit "$lost"
