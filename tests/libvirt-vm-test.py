"""libvirt's daemon stops at once while its udev thread waits for the CPU.

tests/libvirt.nix runs this, and its opening comment says how the old daemon hung. A busy loop
takes the first CPU, and each round pins the daemon there with its udev thread running only
when nothing else will, so ten stops would almost surely catch the old code hanging. The
script needs no value from Nix.
"""

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from driver_names import machine, subtest, t

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
