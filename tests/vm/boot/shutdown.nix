# Reboots a machine whose shutdown would hang, once in each half of it. A
# service whose stop never ends holds the first half, where systemd stops the
# services, so the reboot must force its way on once the minute is up. A
# process that ignores SIGTERM, left behind by its service, holds the last
# half, where systemd-shutdown waits for what's left, so the watchdog must
# reset the machine. QEMU's i6300ESB stands in for the laptop's iTCO, and
# QEMU names a reset the watchdog caused. The test's VM exits on any reset,
# so a reboot ends in wait_for_shutdown.
#
# An account's manager holds a service and a scope that ignore SIGTERM, each
# with a 90 second stop limit, and must still stop in seconds, both alone and
# as part of a reboot, which then tells Plymouth nothing.
#
# Each reboot also says what it told Plymouth: a stand-in for its client
# writes the keyword to the console, where a healthy reboot must write none.
# The tests are shutdown-vm-test.py, which pytest runs inside the driver
# (tests/lib/vm_pytest.py).
{ self, pkgs }:
let
  plymouth = pkgs.writeShellScript "plymouth" (builtins.readFile ./shutdown-plymouth.sh);
  stubborn = pkgs.writeShellScript "stubborn" (builtins.readFile ./shutdown-stubborn.sh);
  # What the stuck service and the leftover run, the tests' `values` fixture.
  values = pkgs.writeText "shutdown-values.json" (
    builtins.toJSON {
      sleep = "${pkgs.coreutils}/bin/sleep";
      sh = "${pkgs.bash}/bin/sh";
      stubborn = "${stubborn}";
    }
  );
in
import ../../lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "shutdown";

  nodes.machine =
    { lib, ... }:
    {
      imports = [ self.nixosModules.systemd ];
      virtualisation.qemu.options = [ "-device i6300esb" ];
      boot.kernelModules = [ "i6300esb" ];
      boot.plymouth.enable = true;
      systemd.services.shutdown-status.environment.PLYMOUTH = lib.mkForce "${plymouth}";
      # An account whose manager runs from the boot, as Joel's does.
      users.users.alice = {
        isNormalUser = true;
        linger = true;
      };
    };

  tests = ./shutdown-vm-test.py;
  inherit values;
}
