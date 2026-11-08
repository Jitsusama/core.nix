#!/usr/bin/env bash
# A 40 GiB disk laid out as Asahi's installer leaves an Apple silicon Mac
# given "UEFI environment only", for guide-beside.nix: Apple's partitions
# in a Mac's numbering (its boot container, macOS's and the recovery
# container at the end), then Asahi's stub and EFI system partition after
# macOS, and free space between. Each of the four not the machine's starts
# with bytes of its own, so the test can prove it untouched by its hash,
# and the boot partition holds files where m1n1 and the firmware's copy
# would. It writes the disk as a qcow2, the table as sfdisk printed it, and
# what the test checks the disk against, into the folder it's given.
set -euo pipefail

out=$1

raw=disk.raw
truncate -s 40G "$raw"

apfs=7C3457EF-0000-11AA-AA11-00306543ECAC
sfdisk --quiet "$raw" <<EOF
label: gpt
unit: sectors

${raw}1 : start=2048, size=131072, type=69646961-6700-11AA-AA11-00306543ECAC, name="iBootSystemContainer"
${raw}2 : start=133120, size=2097152, type=$apfs, name="Container"
${raw}3 : start=83359744, size=524288, type=52637672-7900-11AA-AA11-00306543ECAC, name="RecoveryOSContainer"
${raw}4 : start=2230272, size=524288, type=$apfs, name="Asahi Linux"
${raw}5 : start=2754560, size=524288, type=C12A7328-F81F-11D2-BA4B-00A0C93EC93B, name="EFI - UEFI"
EOF

mkdir -p "$out"
sfdisk --json "$raw" >"$out/table.json"

# So many bytes of a line said over and over. yes stops when head has
# enough, which pipefail would take for a failure.
bytes() { (yes "$1" || true) | head -c "$2"; }

# Where a partition starts and how long it is, in sectors.
part() {
  sfdisk --dump "$raw" | grep "^$raw$1 " | sed -E 's/.*start= *([0-9]+), size= *([0-9]+).*/\1 \2/'
}

# Four MiB of bytes of its own at the start of each partition not the
# machine's, the rest left as zeros, which a hash covers all the same.
for n in 1 2 3 4; do
  read -r first sectors < <(part "$n")
  bytes "partition $n, not the machine's" $((4 << 20)) |
    dd of="$raw" bs=512 seek="$first" conv=notrunc status=none
  dd if="$raw" bs=512 skip="$first" count="$sectors" status=none | sha256sum |
    cut -d' ' -f1 >"$out/partition-$n.sha256"
done

# The boot partition, FAT as Asahi's installer makes it, with m1n1's next
# stage and the firmware's copy where they live.
read -r first sectors < <(part 5)
mkfs.fat --offset="$first" "$raw" $((sectors / 2)) >/dev/null
esp="$raw@@$((first * 512))"
bytes "m1n1 and U-Boot" $((1 << 20)) >boot.bin
bytes "the firmware copied from macOS" $((2 << 20)) >firmware.cpio
mmd -i "$esp" ::m1n1 ::vendorfw
mcopy -i "$esp" boot.bin ::m1n1/boot.bin
mcopy -i "$esp" firmware.cpio ::vendorfw/firmware.cpio
sha256sum boot.bin | cut -d' ' -f1 >"$out/boot.bin.sha256"
sha256sum firmware.cpio | cut -d' ' -f1 >"$out/firmware.cpio.sha256"

qemu-img convert -f raw -O qcow2 "$raw" "$out/disk.qcow2"
