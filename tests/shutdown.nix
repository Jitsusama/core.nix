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
{ self, pkgs }:
let
  plymouth = pkgs.writeShellScript "plymouth" ''
    echo "<3>told plymouth $2" > /dev/kmsg
  '';
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
    import re
    import time
    from datetime import timedelta


    def reboot():
        machine.wait_for_unit("multi-user.target")
        said = len(machine.get_console_log())
        machine.succeed("systemctl --no-block reboot")
        return time.monotonic(), said


    # The kernel's own line, stamped with its time, and not the journal's copy.
    def told(said):
        return re.findall(r"\] told plymouth --status=(\w+)", machine.get_console_log()[said:])


    with subtest("a healthy reboot tells Plymouth nothing"):
        started, said = reboot()
        machine.wait_for_shutdown(timeout=timedelta(seconds=30))
        t.assertEqual(told(said), [])

    with subtest("a service that won't stop holds a reboot a minute"):
        machine.start()
        machine.wait_for_unit("multi-user.target")
        machine.succeed(
            "systemd-run --unit=stuck -p Type=exec -p TimeoutStopSec=infinity"
            " -p ExecStop='${pkgs.coreutils}/bin/sleep infinity'"
            " ${pkgs.coreutils}/bin/sleep infinity"
        )
        started, said = reboot()
        machine.wait_for_shutdown(timeout=timedelta(seconds=120))
        took = time.monotonic() - started
        t.assertGreater(took, 55, "the reboot didn't wait for the service")
        t.assertLess(took, 90, "the reboot waited past the minute")
        t.assertEqual(told(said), ["forced"])

    with subtest("a leftover that ignores SIGTERM can't hold the last stage"):
        machine.start()
        machine.wait_for_unit("multi-user.target")
        machine.succeed("test -e /dev/watchdog0")
        machine.succeed(
            "systemd-run --unit=leftover -p KillMode=none ${pkgs.bash}/bin/sh -c"
            " 'trap \"\" TERM; exec ${pkgs.coreutils}/bin/sleep infinity'"
        )
        started, said = reboot()
        machine.wait_for_qmp_event(
            lambda event: event["event"] == "WATCHDOG", timeout=timedelta(seconds=60)
        )
        took = time.monotonic() - started
        machine.wait_for_shutdown(timeout=timedelta(seconds=30))
        t.assertGreater(took, 25, "the watchdog fired before its 30 seconds")
        t.assertEqual(told(said), ["leftovers"])
  '';
}
