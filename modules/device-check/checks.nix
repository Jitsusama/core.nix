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
  # by hand, and a script stands in for notify-send
  # (tests/devices-that-stop-working-are-named.sh).
  devices-that-stop-working-are-named =
    pkgs.runCommandLocal "devices-that-stop-working-are-named"
      { nativeBuildInputs = [ (pkgs.callPackage ./package.nix { }) ]; }
      ''
        ${lib.getExe pkgs.bash} ${./tests/devices-that-stop-working-are-named.sh}
        touch $out
      '';

  # device-check names a kernel that oopsed, locked up a processor, heard of a
  # hardware error or found a bad page, and a task stuck in it, each once,
  # since a kernel in that state can misbehave until the next boot. Its other
  # taints, a warning among them, say nothing, and neither does a taint that
  # somehow clears, since the kernel never clears one. Files stand in for
  # /proc (tests/kernel-trouble-is-named.sh).
  kernel-trouble-is-named =
    pkgs.runCommandLocal "kernel-trouble-is-named"
      { nativeBuildInputs = [ (pkgs.callPackage ./package.nix { }) ]; }
      ''
        ${lib.getExe pkgs.bash} ${./tests/kernel-trouble-is-named.sh}
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
