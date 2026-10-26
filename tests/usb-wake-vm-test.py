"""A virtual machine's USB 2 and 1.1 root hubs may wake it, as usb-wake sets them, and not USB 3's.

tests/usb-wake.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), on a
machine given a USB controller, which has root hubs at each speed. The kernel leaves each root
hub's wakeup off, so finding the 12 and 480 Mbit/s ones enabled shows the module's setting took,
and the SuperSpeed ones left off shows it took no more. The booted machine's hubs are a fixture,
so any test runs alone with `-k`. The tests need no value from Nix.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

import pytest

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine

# The root hubs' speeds that may wake the machine, in Mbit/s.
WAKING = ("12", "480")


@pytest.fixture(scope="module")
def hubs(machine: BaseMachine) -> dict[str, tuple[str, str]]:
    """Boot, and give each root hub's speed and wakeup, by its folder in sysfs."""
    machine.wait_for_unit("multi-user.target")
    found = {}
    for hub in machine.succeed("ls -d /sys/bus/usb/devices/usb*").split():
        speed = machine.succeed(f"cat {hub}/speed").strip()
        wakeup = machine.succeed(f"cat {hub}/power/wakeup").strip()
        machine.log(f"{hub} at {speed} Mbit/s, wakeup {wakeup}")
        found[hub] = (speed, wakeup)
    return found


def test_the_machine_has_root_hubs_that_may_wake_it_and_ones_that_may_not(
    hubs: dict[str, tuple[str, str]],
) -> None:
    """The machine has a USB 2 root hub and a SuperSpeed one, so both rules are tried."""
    speeds = [speed for speed, _ in hubs.values()]
    assert "480" in speeds
    assert any(speed not in WAKING for speed in speeds)


def test_usb_2_and_1_1_root_hubs_may_wake_it(hubs: dict[str, tuple[str, str]]) -> None:
    """Its USB 2 and 1.1 root hubs may wake it, so a cable plugged in does."""
    slow = {hub: wakeup for hub, (speed, wakeup) in hubs.items() if speed in WAKING}
    assert slow
    assert set(slow.values()) == {"enabled"}


def test_usb_3_root_hubs_may_not_wake_it(hubs: dict[str, tuple[str, str]]) -> None:
    """Its USB 3 root hubs may not wake it, so a device pulled out while it sleeps doesn't."""
    fast = {hub: wakeup for hub, (speed, wakeup) in hubs.items() if speed not in WAKING}
    assert fast
    assert set(fast.values()) == {"disabled"}
