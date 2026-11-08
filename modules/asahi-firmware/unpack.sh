# shellcheck shell=bash
# Unpacks the firmware Asahi's installer copied out of macOS (Wi-Fi,
# Bluetooth, the camera) from the EFI system partition into the initrd, as
# asahi-scripts' dracut module (99asahi-firmware) does, before udev asks
# for any of it.

# /lib is in the store inside the initrd, so the link to where the
# firmware lands can't be made when the initrd is built.
mkdir -p /lib/firmware
ln -s /vendorfw /lib/firmware/vendor

if [ -e /vendorfw ]; then
  echo "asahi-firmware: the boot loader already loaded the firmware"
  exit 0
fi

# m1n1 names the partition Asahi's installer made for this system.
partition=/proc/device-tree/chosen/asahi,efi-system-partition
if [ ! -e "$partition" ]; then
  echo "asahi-firmware: m1n1 named no EFI system partition, so no firmware is loaded"
  exit 0
fi

disk_is_up() {
  for disk in /sys/bus/platform/drivers/nvme-apple/*.nvme/nvme/nvme*/nvme*n1/; do
    [ -e "$disk" ] && return 0
  done
  return 1
}

# This runs before udev, so nothing announces the disk: it's looked for
# every tenth of a second, for five seconds, as Asahi's own does.
modprobe nvme-apple
for _ in $(seq 0 50); do
  disk_is_up && break
  sleep 0.1
done
if ! disk_is_up; then
  echo "asahi-firmware: the disk didn't appear within five seconds" >&2
  exit 1
fi

esp=/run/.system-efi
cpio_file="$esp/vendorfw/firmware.cpio"
mkdir -p "$esp"
mount "PARTUUID=$(tr -d '\0' <"$partition")" "$esp"
if [ ! -e "$cpio_file" ]; then
  echo "asahi-firmware: no vendorfw/firmware.cpio on the EFI system partition" >&2
  umount "$esp"
  exit 1
fi
(cd / && cpio --quiet -i <"$cpio_file")
echo "asahi-firmware: unpacked"
umount "$esp"
