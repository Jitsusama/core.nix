"""libvirt's daemon stops at once while its udev thread waits for the CPU.

tests/libvirt.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and its
opening comment says how the old daemon hung. A busy loop takes the first CPU, and each round
pins the daemon there with its udev thread running only when nothing else will, so ten stops
would almost surely catch the old code hanging. The test needs no value from Nix.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

import pytest

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine

# How many times the daemon is stopped with its udev thread starved.
ROUNDS = 10


@pytest.fixture(scope="module")
def busy(machine: BaseMachine) -> BaseMachine:
    """Boot the machine until libvirt's daemon runs, and keep its first CPU busy."""
    machine.wait_for_unit("libvirtd.service")
    machine.succeed(
        "systemd-run --unit=busy -p Type=exec -p CPUAffinity=0 /bin/sh -c 'while :; do :; done'"
    )
    return machine


def test_the_daemon_stops_at_once_while_its_udev_thread_waits(busy: BaseMachine) -> None:
    """The daemon stops at once while its udev thread waits."""
    machine = busy
    for attempt in range(ROUNDS):
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
        assert result == "success", f"stop {attempt + 1} of {ROUNDS}"
    machine.succeed("systemctl is-active busy.service")
    journal = machine.succeed("journalctl -b -o cat -u libvirtd.service")
    assert "Make forcefull daemon shutdown" not in journal
