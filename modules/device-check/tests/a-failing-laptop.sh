# shellcheck shell=bash
# Builds what a laptop with three things wrong shows device-check, for
# commands-agree to set its bare run at each width from: a camera whose
# power change failed, Bluetooth expected and missing, and a disk smartd
# heard a fault from. It's built rather than kept as files, since a
# stand-in sysfs is folders a dozen deep and a driver is a link.
# usage: a-failing-laptop.sh <folder to build it in>
set -euo pipefail

out=$1
root=$out/root

# A device in the stand-in sysfs, in the power state given.
device() {
  mkdir -p "$root/sys/devices/$1/power"
  echo "$2" > "$root/sys/devices/$1/power/runtime_status"
}

device pci/0000:00:05.0 error
mkdir -p "$root/sys/bus/pci/drivers/intel-ipu7"
ln -s ../../../bus/pci/drivers/intel-ipu7 "$root/sys/devices/pci/0000:00:05.0/driver"
device pci/0000:00:14.0 active
mkdir -p "$root/sys/class/ieee80211/phy0" "$root/run/device-check/disks"
printf '/dev/nvme0\tDevice: /dev/nvme0, Critical Warning (0x04): Reliability\n' \
  > "$root/run/device-check/disks/nvme0"
printf 'Bluetooth=/sys/class/bluetooth/hci0\nWi-Fi=/sys/class/ieee80211/phy*\n' \
  > "$out/expected"
mkdir -p "$out/found"
