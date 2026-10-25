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
# The tests are shutdown-vm-test.py, which pytest runs inside the driver
# (tests/lib/vm_pytest.py).
{ self, pkgs }:
let
  plymouth = pkgs.writeShellScript "plymouth" (builtins.readFile ./shutdown-plymouth.sh);
  # What the stuck service and the leftover run, the tests' `values` fixture.
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

  testScript = ''
    import vm_pytest
    vm_pytest.run(globals(), "${./shutdown-vm-test.py}", values="${values}")
  '';
  extraPythonPackages = python: [ (pkgs.callPackage ./lib/vm-helpers.nix { } python) ];
  # nix fmt holds the tests and vm_pytest to mypy --strict against the
  # driver's own types, so the driver's check of these lines would add only
  # its time.
  skipTypeCheck = true;
}
