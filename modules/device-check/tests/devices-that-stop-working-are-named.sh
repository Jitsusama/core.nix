# shellcheck shell=bash
# Runs device-check over files standing in for sysfs, with smartd's
# variables set by hand and a stand-in for notify-send, and checks it names
# a device whose power change failed, an expected one that's gone and a
# disk smartd heard a fault from, each once, as a warning and a
# notification, and again when each is fixed, exiting 0 whatever it finds.
# device-check and device-check-disk come from PATH.
# usage: devices-that-stop-working-are-named.sh
set -euo pipefail

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
cd "$work"
root=$work/root
fault="Critical Warning (0x04): Reliability"

fail() {
  echo "$1"
  cat out
  exit 1
}
# A device in the stand-in sysfs, in the power state given.
device() {
  mkdir -p "$root/sys/devices/$1/power"
  echo "$2" > "$root/sys/devices/$1/power/runtime_status"
}
# smartd's hook, told of the disk's fault as smartd tells it.
smartd_reports() {
  SMARTD_DEVICE=/dev/nvme0 SMARTD_FAILTYPE=Health \
    SMARTD_MESSAGE="Device: /dev/nvme0, $fault" device-check-disk > out 2>&1
}

device pci/0000:00:05.0 error
mkdir -p "$root/sys/bus/pci/drivers/intel-ipu7"
ln -s ../../../bus/pci/drivers/intel-ipu7 "$root/sys/devices/pci/0000:00:05.0/driver"
device "pci/0000:00:1f.3/sof_sdw/Jack Out" suspended
device pci/0000:00:14.0 active
mkdir -p "$root/sys/class/ieee80211/phy1"
printf 'Bluetooth=/sys/class/bluetooth/hci0\nWi-Fi=/sys/class/ieee80211/phy*\n' > expected
printf '#!%s\necho "$*" >> %s/notified\n' "$BASH" "$work" > notify
chmod +x notify
export DEVICE_CHECK_ROOT=$root DEVICE_CHECK_EXPECTED=$work/expected
export DEVICE_CHECK_STATE=$work/state DEVICE_CHECK_NOTIFY=$work/notify

smartd_reports || fail "smartd's hook failed"
[ ! -s out ] || fail "smartd's hook wrote to its output, which smartd reads as trouble"

device-check 2> out || fail "it failed with devices to name"
grep -qxF "<4>0000:00:05.0 (intel-ipu7) failed a power change and won't wake until the next boot" out \
  || fail "it didn't warn of the failed device"
grep -qxF "<4>Bluetooth is missing" out || fail "it didn't warn of the missing device"
grep -qxF "<4>The disk /dev/nvme0 reports $fault" out || fail "it didn't warn of the disk's fault"
[ "$(wc -l < out)" = 3 ] || fail "it named more than what's wrong"
[ "$(grep -c '^-u critical -a Devices ' notified)" = 3 ] || fail "it didn't notify of all three"

: > notified
device-check 2> out || fail "a second look failed"
[ ! -s out ] && [ ! -s notified ] || fail "it named the same findings again"

smartd_reports || fail "smartd's hook failed a second time"
device-check 2> out || fail "a look after smartd's repeat failed"
[ ! -s out ] && [ ! -s notified ] || fail "it named the disk's fault again when smartd repeated it"

echo active > "$root/sys/devices/pci/0000:00:05.0/power/runtime_status"
mkdir -p "$root/sys/class/bluetooth/hci0"
rm "$root"/run/device-check/disks/*
device-check 2> out || fail "a look after the fix failed"
grep -qxF "<5>0000:00:05.0 (intel-ipu7) wakes again" out || fail "it didn't say the device wakes"
grep -qxF "<5>Bluetooth is back" out || fail "it didn't say the device is back"
grep -qxF "<5>The disk /dev/nvme0 no longer reports $fault" out \
  || fail "it didn't say the disk's fault cleared"
[ "$(grep -c '^-u normal -a Devices ' notified)" = 3 ] || fail "it didn't notify of the fixes"
