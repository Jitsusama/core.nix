# The checks of device-check, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  holds,
  lib,
  onLinux,
  passesClippy,
  pkgs,
  workstation,
  ...
}:
{
  # device-check names a device whose power change failed, one the machine
  # expects that's gone and a disk smartd heard a fault from, each once, as a
  # warning and a notification, and again when it's fixed, and exits 0
  # whatever it finds. Files stand in for sysfs, smartd's variables are set
  # by hand, and a script stands in for notify-send.
  devices-that-stop-working-are-named =
    pkgs.runCommandLocal "devices-that-stop-working-are-named"
      { nativeBuildInputs = [ (pkgs.callPackage ./package.nix { }) ]; }
      ''
        root=$PWD/root
        fail() { echo "$1"; cat out; exit 1; }
        device() {
          mkdir -p "$root/sys/devices/$1/power"
          echo "$2" > "$root/sys/devices/$1/power/runtime_status"
        }
        device pci/0000:00:05.0 error
        mkdir -p "$root/sys/bus/pci/drivers/intel-ipu7"
        ln -s ../../../bus/pci/drivers/intel-ipu7 "$root/sys/devices/pci/0000:00:05.0/driver"
        device "pci/0000:00:1f.3/sof_sdw/Jack Out" suspended
        device pci/0000:00:14.0 active
        mkdir -p "$root/sys/class/ieee80211/phy1"
        printf 'Bluetooth=/sys/class/bluetooth/hci0\nWi-Fi=/sys/class/ieee80211/phy*\n' > expected
        printf '#!/bin/sh\necho "$*" >> %s/notified\n' "$PWD" > notify
        chmod +x notify
        export DEVICE_CHECK_ROOT=$root DEVICE_CHECK_EXPECTED=$PWD/expected
        export DEVICE_CHECK_STATE=$PWD/state DEVICE_CHECK_NOTIFY=$PWD/notify

        SMARTD_DEVICE=/dev/nvme0 SMARTD_FAILTYPE=Health \
          SMARTD_MESSAGE="Device: /dev/nvme0, Critical Warning (0x04): Reliability" \
          device-check-disk > out 2>&1 || fail "smartd's hook failed"
        [ ! -s out ] || fail "smartd's hook wrote to its output, which smartd reads as trouble"

        device-check 2> out || fail "it failed with devices to name"
        grep -qxF "<4>0000:00:05.0 (intel-ipu7) failed a power change and won't wake until the next boot" out ||
          fail "it didn't warn of the failed device"
        grep -qxF "<4>Bluetooth is missing" out || fail "it didn't warn of the missing device"
        grep -qxF "<4>The disk /dev/nvme0 reports Critical Warning (0x04): Reliability" out ||
          fail "it didn't warn of the disk's fault"
        [ "$(wc -l < out)" = 3 ] || fail "it named more than what's wrong"
        [ "$(grep -c '^-u critical -a Devices ' notified)" = 3 ] || fail "it didn't notify of all three"

        : > notified
        device-check 2> out || fail "a second look failed"
        [ ! -s out ] && [ ! -s notified ] || fail "it named the same findings again"

        SMARTD_DEVICE=/dev/nvme0 SMARTD_FAILTYPE=Health \
          SMARTD_MESSAGE="Device: /dev/nvme0, Critical Warning (0x04): Reliability" \
          device-check-disk > out 2>&1 || fail "smartd's hook failed a second time"
        device-check 2> out || fail "a look after smartd's repeat failed"
        [ ! -s out ] && [ ! -s notified ] || fail "it named the disk's fault again when smartd repeated it"

        echo active > "$root/sys/devices/pci/0000:00:05.0/power/runtime_status"
        mkdir -p "$root/sys/class/bluetooth/hci0"
        rm "$root"/run/device-check/disks/*
        device-check 2> out || fail "a look after the fix failed"
        grep -qxF "<5>0000:00:05.0 (intel-ipu7) wakes again" out || fail "it didn't say the device wakes"
        grep -qxF "<5>Bluetooth is back" out || fail "it didn't say the device is back"
        grep -qxF "<5>The disk /dev/nvme0 no longer reports Critical Warning (0x04): Reliability" out ||
          fail "it didn't say the disk's fault cleared"
        [ "$(grep -c '^-u normal -a Devices ' notified)" = 3 ] || fail "it didn't notify of the fixes"
        touch $out
      '';

  # device-check names a kernel that oopsed, locked up a processor, heard of a
  # hardware error or found a bad page, and a task stuck in it, each once,
  # since a kernel in that state can misbehave until the next boot. Its other
  # taints, a warning among them, say nothing, and neither does a taint that
  # somehow clears, since the kernel never clears one. Files stand in for
  # /proc.
  kernel-trouble-is-named =
    pkgs.runCommandLocal "kernel-trouble-is-named"
      { nativeBuildInputs = [ (pkgs.callPackage ./package.nix { }) ]; }
      ''
        root=$PWD/root
        fail() { echo "$1"; cat out; exit 1; }
        kernel=$root/proc/sys/kernel
        mkdir -p "$kernel"
        printf '#!/bin/sh\necho "$*" >> %s/notified\n' "$PWD" > notify
        chmod +x notify
        export DEVICE_CHECK_ROOT=$root DEVICE_CHECK_EXPECTED=$PWD/none
        export DEVICE_CHECK_STATE=$PWD/state DEVICE_CHECK_NOTIFY=$PWD/notify
        echo 120 > "$kernel/hung_task_timeout_secs"
        echo 0 > "$kernel/hung_task_detect_count"

        # A warning, an out-of-tree module and a staging driver.
        echo $((512 + 4096 + 1024)) > "$kernel/tainted"
        device-check 2> out || fail "it failed on a kernel with harmless taints"
        [ ! -s out ] || fail "it named a harmless taint"

        echo $((128 + 512 + 4096 + 1024)) > "$kernel/tainted"
        device-check 2> out || fail "it failed on a kernel that oopsed"
        grep -qxF "<4>The kernel hit a fault and may misbehave until the next boot" out ||
          fail "it didn't warn of the oops"
        [ "$(wc -l < out)" = 1 ] || fail "it named more than the oops"
        [ "$(grep -c '^-u critical -a Devices ' notified)" = 1 ] || fail "it didn't notify of the oops"

        : > notified
        device-check 2> out || fail "a second look failed"
        [ ! -s out ] && [ ! -s notified ] || fail "it named the oops again"

        echo 1 > "$kernel/hung_task_detect_count"
        echo $((128 + 16384 + 16 + 32)) > "$kernel/tainted"
        device-check 2> out || fail "it failed on a kernel in worse trouble"
        grep -qxF "<4>The kernel locked up a processor and may misbehave until the next boot" out ||
          fail "it didn't warn of the lockup"
        grep -qxF "<4>The processor reported a hardware error" out ||
          fail "it didn't warn of the machine check"
        grep -qxF "<4>The kernel found a damaged memory page" out || fail "it didn't warn of the bad page"
        grep -qxF "<4>A task was stuck in the kernel for over 120 seconds" out ||
          fail "it didn't warn of the stuck task"
        [ "$(wc -l < out)" = 4 ] || fail "it named the oops again, or more than what's new"

        echo 2 > "$kernel/hung_task_detect_count"
        device-check 2> out || fail "a look after a second stuck task failed"
        [ ! -s out ] || fail "it named a second stuck task"

        : > notified
        echo 0 > "$kernel/tainted"
        echo 0 > "$kernel/hung_task_detect_count"
        device-check 2> out || fail "a look at a clean kernel failed"
        [ ! -s out ] && [ ! -s notified ] || fail "it said the kernel's trouble had cleared"
        touch $out
      '';

  # A workstation hears from its disks before they fail: smartd reads each
  # disk's own warnings and hands them to device-check, which names them to
  # whoever is signed in, rather than writing to every terminal.
  disks-say-when-they-fail =
    let
      inherit (workstation.config.services) smartd;
    in
    holds "disks-say-when-they-fail"
      "A workstation doesn't watch its disks, or doesn't hand what they report to device-check."
      (
        smartd.enable
        && smartd.autodetect
        && !smartd.notifications.wall.enable
        && lib.hasInfix "-H -f -l error " smartd.defaults.autodetected
        && lib.hasSuffix "/bin/device-check-disk" smartd.defaults.autodetected
      );

  # A machine with no disk that speaks SMART, as a virtual machine or a USB
  # stick can be, runs smartd without it failing: it finds nothing to watch
  # and stops cleanly. The build's sandbox has no disks, so the workstation's
  # own smartd command runs there as it would on such a machine.
  smartd-stops-cleanly-without-disks =
    let
      smartd = workstation.config.systemd.services.smartd.serviceConfig.ExecStart;
    in
    pkgs.runCommandLocal "smartd-stops-cleanly-without-disks"
      { nativeBuildInputs = [ pkgs.coreutils ]; }
      ''
        fail() { echo "$1"; cat out; exit 1; }
        status=0
        timeout 60 ${smartd} > out 2>&1 || status=$?
        [ "$status" != 124 ] || fail "smartd kept running with no disks to watch"
        [ "$status" = 0 ] || fail "smartd failed with no disks to watch, exiting $status"
        touch $out
      '';
}
// onLinux {
  # clippy finds nothing in device-check or device-check-disk, their tests
  # included.
  device-check-passes-clippy = passesClippy "device-check-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
}
