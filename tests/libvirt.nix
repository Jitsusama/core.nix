# Stops libvirt's daemon while its udev thread waits for the CPU, as it does
# on a machine busy shutting down. At shutdown libvirt tells that thread to
# end and then joins it, holding the lock the thread needs to see it was told,
# so a thread that hasn't run by then never can: both wait until the daemon
# gives up 30 seconds later and exits as failed. The daemon shares one CPU
# with a busy loop here, and its udev thread runs only when nothing else will,
# so a stop often finds the thread still waiting, and ten leave the old code
# next to no way through.
{ self, pkgs }:
pkgs.testers.runNixOSTest {
  name = "libvirt";

  nodes.machine = {
    imports = [ self.nixosModules.libvirt ];
    virtualisation.memorySize = 2048;
    # The busy loop takes one, and the test's own shell the other.
    virtualisation.cores = 2;
  };

  testScript = ''
    machine.wait_for_unit("libvirtd.service")
    machine.succeed(
        "systemd-run --unit=busy -p Type=exec -p CPUAffinity=0 /bin/sh -c 'while :; do :; done'"
    )

    with subtest("the daemon stops at once while its udev thread waits"):
        for attempt in range(10):
            # systemd refuses a unit's sixth start in ten seconds, manual ones
            # included, and a fast runner gets through a round in under two.
            # These starts are the test's, so its count is cleared for each.
            machine.succeed("systemctl reset-failed libvirtd.service libvirtd-config.service")
            machine.succeed("systemctl start libvirtd.service")
            machine.succeed("virsh --connect qemu:///system version")
            pid = machine.succeed("systemctl show -P MainPID libvirtd.service").strip()
            machine.succeed(f"taskset -a -p -c 0 {pid}")
            thread = machine.succeed(f"grep -l -x udev-event /proc/{pid}/task/*/comm").split("/")[4]
            machine.succeed(f"chrt --idle -p 0 {thread}")
            machine.succeed("systemctl stop libvirtd.service")
            result = machine.succeed("systemctl show -P Result libvirtd.service").strip()
            t.assertEqual(result, "success", f"stop {attempt + 1} of 10")
        machine.succeed("systemctl is-active busy.service")
        journal = machine.succeed("journalctl -b -o cat -u libvirtd.service")
        t.assertNotIn("Make forcefull daemon shutdown", journal)
  '';
}
