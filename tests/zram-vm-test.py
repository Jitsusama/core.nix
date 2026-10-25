"""The compressed swap comes down while something holds its device open, as udev's probe does.

tests/zram.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and its
opening comment says why a reset that tries once fails. hold-zram, on the machine's path,
holds /dev/zram0 open until the reset says it's busy, and lets go at once. The test needs no
value from Nix.
"""

from __future__ import annotations

from datetime import timedelta
from typing import TYPE_CHECKING

import pytest

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine

SETUP = "systemd-zram-setup@zram0.service"
# How long the setup's stop may take: hold-zram lets go at the first busy reset.
STOPPING = timedelta(seconds=30)


@pytest.fixture
def held(machine: BaseMachine) -> BaseMachine:
    """Give the machine with its compressed swap up and hold-zram holding the device open."""
    machine.wait_for_unit("dev-zram0.swap")
    machine.succeed("systemd-run --unit=hold -p Type=exec hold-zram")
    machine.wait_for_file("/run/holding-zram")
    return machine


def test_the_swap_comes_down_once_the_device_is_let_go(held: BaseMachine) -> None:
    """The swap comes down once the device is let go, its size reset to nothing."""
    held.succeed(f"systemctl stop --no-block {SETUP}")
    held.wait_until_succeeds(
        f"systemctl show -P ActiveState {SETUP} | grep -q -x -E 'inactive|failed'",
        timeout=STOPPING,
    )
    assert held.succeed(f"systemctl show -P Result {SETUP}").strip() == "success"
    assert held.succeed("cat /sys/block/zram0/disksize").strip() == "0"
