"""A laptop's lid shut in a virtual machine, with and without a monitor plugged in.

tests/lid.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and its
opening comment says what the test proves. Each test asks for the state it starts from as a
fixture: the machine booted with its stand-in connectors, or its lid shut at a desk with the
monitor off. The stand-in lid, connectors and suspend are all named in the machine lid.nix
builds, so the tests need no value from Nix.
"""

from __future__ import annotations

import datetime as dt
from typing import TYPE_CHECKING

import pytest

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine

HELD = "systemd-inhibit --list --no-legend --no-pager | grep -q lid-docked"
HOLDER = "systemctl show -P MainPID lid-docked.service"
OPENED = (
    "busctl get-property org.freedesktop.login1 /org/freedesktop/login1"
    " org.freedesktop.login1.Manager LidClosed | grep -qx 'b false'"
)
# The suspend unit's state is read rather than asked with is-active, which calls a oneshot
# still running "activating" and so can't tell a suspend under way from one that's over.
SETTLED = (
    "busctl get-property org.freedesktop.login1 /org/freedesktop/login1"
    " org.freedesktop.login1.Manager PreparingForSleep | grep -qx 'b false'"
    " && ! systemctl list-jobs --no-legend | grep -q suspend"
    " && systemctl show -P ActiveState systemd-suspend.service | grep -qx inactive"
)
SUSPENDED = "test -e /run/suspended"
# How long logind may take to suspend once nothing holds the lid, or to settle afterwards.
SUSPENDING = dt.timedelta(minutes=1)
# How long logind may take to see the lid open.
OPENING = dt.timedelta(seconds=30)
# How long lid-docked may take to hold the lid once a monitor is plugged in.
HOLDING = dt.timedelta(seconds=10)
# How long logind is given to act on a shut lid it mustn't suspend for.
LOOKING = dt.timedelta(seconds=5)


class Laptop:
    """The machine's stand-in lid and monitor, and what lid-docked does about them."""

    def __init__(self, machine: BaseMachine) -> None:
        """Act on the stand-ins in that machine."""
        self.machine = machine

    def lid(self, *, shut: bool) -> None:
        """Shut the stand-in lid, or open it."""
        self.machine.succeed(f"echo {int(shut)} > /sys/module/stand_in_lid/parameters/shut")

    def monitor(self, status: str) -> None:
        """Write the monitor's connector as connected or disconnected, as the card would."""
        self.machine.succeed(
            f"echo {status} > /run/stand-in-drm/card0-DP-5/status",
            "udevadm trigger --settle --action=change /sys/class/drm/card0",
        )

    def holder(self) -> str:
        """Give the process lid-docked holds the lid with."""
        return self.machine.succeed(HOLDER).strip()


@pytest.fixture(scope="module")
def laptop(machine: BaseMachine) -> Laptop:
    """Boot the machine with the laptop's own screen connected and a monitor's connector."""
    machine.wait_for_unit("multi-user.target")
    machine.succeed(
        "test -e /sys/class/drm/card0",
        "mkdir -p /run/stand-in-drm/card0-eDP-1 /run/stand-in-drm/card0-DP-5",
        "echo connected > /run/stand-in-drm/card0-eDP-1/status",
    )
    return Laptop(machine)


@pytest.fixture(scope="module")
def shut_at_desk(laptop: Laptop) -> str:
    """Plug the monitor in and turn it off, shut the lid, and give the process holding it."""
    laptop.monitor("connected")
    laptop.machine.wait_until_succeeds(HELD, timeout=HOLDING)
    laptop.machine.succeed("echo disabled > /run/stand-in-drm/card0-DP-5/enabled")
    first = laptop.holder()
    laptop.lid(shut=True)
    return first


def test_a_laptop_shut_with_no_monitor_suspends(laptop: Laptop) -> None:
    """A laptop shut with no monitor suspends."""
    laptop.monitor("disconnected")
    laptop.machine.fail(HELD)
    laptop.lid(shut=True)
    laptop.machine.wait_until_succeeds(SUSPENDED, timeout=SUSPENDING)
    laptop.lid(shut=False)
    # The stand-in suspend returns at once, and logind, finding the lid
    # still shut, may queue another, which a loaded machine starts after
    # the lid opens. The file is cleared once logind has seen the lid
    # open and isn't preparing to sleep, and no suspend is queued or
    # running.
    laptop.machine.wait_until_succeeds(OPENED, timeout=OPENING)
    laptop.machine.wait_until_succeeds(SETTLED, timeout=SUSPENDING)
    laptop.machine.succeed("rm /run/suspended")


def test_a_laptop_shut_at_a_desk_keeps_running_once_the_monitor_is_off(
    laptop: Laptop, shut_at_desk: str
) -> None:
    """A laptop shut at a desk keeps running once the monitor is off."""
    laptop.machine.sleep(LOOKING)
    # A monitor waking or sleeping changes the card again.
    laptop.monitor("connected")
    laptop.machine.sleep(LOOKING)
    assert laptop.holder() == shut_at_desk
    laptop.machine.fail(SUSPENDED)


@pytest.mark.usefixtures("shut_at_desk")
def test_unplugging_the_monitor_with_the_lid_shut_suspends(laptop: Laptop) -> None:
    """Unplugging the monitor with the lid shut suspends."""
    laptop.monitor("disconnected")
    laptop.machine.wait_until_succeeds(SUSPENDED, timeout=SUSPENDING)
    laptop.machine.fail(HELD)
