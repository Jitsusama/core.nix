# Reboots a machine whose shutdown would hang, once in each half of it. A
# service whose stop never ends holds the first half, where systemd stops the
# services, so the reboot must force its way on once the minute is up. A
# process that ignores SIGTERM, left behind by its service, holds the last
# half, where systemd-shutdown waits for what's left, so the watchdog must
# reset the machine. QEMU's i6300ESB stands in for the laptop's iTCO, and
# QEMU names a reset the watchdog caused. The test's VM exits on any reset,
# so a reboot ends in wait_for_shutdown.
#
# Each reboot also says what it told Plymouth: a stand-in for its client
# writes the keyword to the console, where a healthy reboot must write none.
# The script is shutdown-vm-test.py.
{ self, pkgs }:
let
  plymouth = pkgs.writeShellScript "plymouth" ''
    echo "<3>told plymouth $2" > /dev/kmsg
  '';
  # What the stuck service and the leftover run, which the script reads from
  # this file, swapped in for its @values@ marker.
  values = pkgs.writeText "shutdown-values.json" (
    builtins.toJSON {
      sleep = "${pkgs.coreutils}/bin/sleep";
      sh = "${pkgs.bash}/bin/sh";
    }
  );
in
pkgs.testers.runNixOSTest {
  name = "shutdown";

  nodes.machine =
    { lib, ... }:
    {
      imports = [ self.nixosModules.systemd ];
      virtualisation.qemu.options = [ "-device i6300esb" ];
      boot.kernelModules = [ "i6300esb" ];
      boot.plymouth.enable = true;
      systemd.services.shutdown-status.environment.PLYMOUTH = lib.mkForce "${plymouth}";
    };

  testScript = builtins.replaceStrings [ "@values@" ] [ "${values}" ] (
    builtins.readFile ./shutdown-vm-test.py
  );
  # nix fmt holds the script to mypy --strict against the driver's own types,
  # through driver_names.pyi, which the driver's lighter check can't find.
  skipTypeCheck = true;
}
