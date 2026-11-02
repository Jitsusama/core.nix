"""A laptop's lid shut in a virtual machine with its hinge coming down, left open and slammed.

tests/lid-hinge.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and
its opening comment says what the test proves. The stand-in lid, hinge, light sensor and sleep
are all named in the machine lid-hinge.nix builds, so the tests need no value from Nix.
"""

from __future__ import annotations

import datetime as dt
import struct
from typing import TYPE_CHECKING

import pytest

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine

HINGE = "/run/stand-in-iio-nodes/iio:device0"
LIGHT = "/run/stand-in-iio/iio:device1/in_illuminance_raw"
OPENED = (
    "busctl get-property org.freedesktop.login1 /org/freedesktop/login1"
    " org.freedesktop.login1.Manager LidClosed | grep -qx 'b false'"
)
# As lid-vm-test.py reads it: the sleep unit's state, since is-active calls a oneshot still
# running "activating".
SETTLED = (
    "busctl get-property org.freedesktop.login1 /org/freedesktop/login1"
    " org.freedesktop.login1.Manager PreparingForSleep | grep -qx 'b false'"
    " && ! systemctl list-jobs --no-legend | grep -q suspend"
    " && systemctl show -P ActiveState systemd-suspend.service | grep -qx inactive"
)
ASLEEP = "test -e /run/suspended"
LOGIND_SAID = "journalctl -b -u systemd-logind --no-pager -o cat"
SAID = "journalctl -b -u lid-hinge --no-pager -o cat"
# How long logind may take to sleep once the lid shuts, or to settle afterwards.
FALLING_ASLEEP = dt.timedelta(minutes=1)
# How long logind may take to see the lid open, or lid-hinge to hear what it's sent.
OPENING = dt.timedelta(seconds=30)
# How long logind is given to act on a shut lid it mustn't sleep for.
LOOKING = dt.timedelta(seconds=5)
# How long a sample may take to go into the hinge's pipe, which waits while nothing reads it.
WRITING = 10
# Near shut and wide open, in the hinge sensor's steps of a degree.
WIDE_OPEN = 100
COMING_DOWN = (60, 30, 10)


class Laptop:
    """The machine's stand-in lid, hinge and light, and what logind makes of them."""

    def __init__(self, machine: BaseMachine) -> None:
        """Act on the stand-ins in that machine."""
        self.machine = machine

    def lid(self, *, shut: bool) -> None:
        """Shut the stand-in lid, or open it."""
        self.machine.succeed(f"echo {int(shut)} > /sys/module/stand_in_lid/parameters/shut")

    def hinge(self, *angles: int) -> None:
        """Put each angle in the hinge's buffer, as the sensor hub would."""
        sample = "".join(f"\\x{byte:02x}" for angle in angles for byte in struct.pack("<i", angle))
        self.machine.succeed(f"timeout {WRITING} sh -c \"printf '{sample}' > {HINGE}\"")

    def light(self, raw: int) -> None:
        """Set what the light sensor reads."""
        self.machine.succeed(f"echo {raw} > {LIGHT}")

    def said(self, words: str) -> int:
        """Count lid-hinge's lines in the journal with those words."""
        lines = self.machine.succeed(SAID)
        return sum(words in line for line in lines.splitlines())

    def wait_to_say(self, words: str, before: int) -> None:
        """Wait for lid-hinge to say those words once more than it had."""
        try:
            self.machine.wait_until_succeeds(
                f"test $({SAID} | grep -cF '{words}') -gt {before}",
                timeout=OPENING,
            )
        except Exception as error:
            said = self.machine.execute(SAID)[1]
            message = f"lid-hinge never said {words!r}; it said:\n{said}"
            raise AssertionError(message) from error

    def open_and_settle(self) -> None:
        """Open the lid and wait for logind to settle, clearing the stand-in sleep's file."""
        self.lid(shut=False)
        self.machine.wait_until_succeeds(OPENED, timeout=OPENING)
        self.machine.wait_until_succeeds(SETTLED, timeout=FALLING_ASLEEP)
        self.machine.succeed("rm -f /run/suspended")


@pytest.fixture(scope="module")
def laptop(machine: BaseMachine) -> Laptop:
    """Boot the machine on battery with lid-hinge following the stand-in lid and its hinge."""
    machine.wait_for_unit("multi-user.target")
    machine.wait_for_unit("lid-hinge.service")
    machine.wait_for_unit("stand-in-hinge.service")
    machine.succeed("grep -qx Discharging /sys/class/power_supply/test_battery/status")
    machine.wait_until_succeeds(f"{LOGIND_SAID} | grep -qF '(Lid Switch (Hinge))'", timeout=OPENING)
    return Laptop(machine)


def test_a_hinge_that_comes_late_is_found(laptop: Laptop) -> None:
    """A hinge that appears after lid-hinge has looked for one is found, as at a real boot."""
    laptop.machine.succeed(f"{SAID} | grep -qF 'with no hinge'")
    laptop.wait_to_say("found the hinge", 0)


def test_logind_hears_lid_hinges_switch_alone(laptop: Laptop) -> None:
    """Logind watches lid-hinge's switch, and udev hides the lid's own from it."""
    laptop.machine.fail(f"{LOGIND_SAID} | grep -qF 'Stand-in Lid Switch'")


def test_a_lid_shut_with_the_hinge_coming_down_sleeps(laptop: Laptop) -> None:
    """A lid shut with the hinge coming down sleeps at once."""
    before = laptop.said("nearly shut")
    laptop.hinge(WIDE_OPEN, *COMING_DOWN, 0)
    laptop.wait_to_say("nearly shut", before)
    laptop.lid(shut=True)
    laptop.machine.wait_until_succeeds(ASLEEP, timeout=FALLING_ASLEEP)
    laptop.open_and_settle()


def test_a_lid_shut_wide_open_in_the_light_keeps_running(laptop: Laptop) -> None:
    """A lid's switch saying shut with the hinge wide open and the light on keeps running."""
    laptop.light(10000)
    laptop.hinge(WIDE_OPEN)
    before = laptop.said("until the light goes dark")
    laptop.lid(shut=True)
    laptop.wait_to_say("until the light goes dark", before)
    laptop.machine.sleep(LOOKING)
    laptop.machine.fail(ASLEEP)
    laptop.machine.succeed(OPENED)
    laptop.open_and_settle()


def test_a_lid_slammed_shut_sleeps_once_the_light_goes_dark(laptop: Laptop) -> None:
    """A lid slammed shut from wide open sleeps once the light goes dark."""
    laptop.light(10000)
    laptop.hinge(WIDE_OPEN)
    before = laptop.said("until the light goes dark")
    laptop.lid(shut=True)
    laptop.wait_to_say("until the light goes dark", before)
    laptop.machine.fail(ASLEEP)
    laptop.light(0)
    laptop.machine.wait_until_succeeds(ASLEEP, timeout=FALLING_ASLEEP)
    laptop.open_and_settle()
