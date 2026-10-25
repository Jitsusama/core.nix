"""A reboot whose shutdown would hang goes on anyway, and says what it told Plymouth.

tests/shutdown.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and
its opening comment says what the test proves. The machine exits on any reset, so each test's
reboot ends with the machine down, and the next test starts it again. The programs the stuck
service and the leftover run are named by their paths in the store, given as the `values`
fixture.
"""

from __future__ import annotations

import re
import time
from dataclasses import dataclass
from datetime import timedelta
from typing import TYPE_CHECKING, Any

import pytest
from test_driver.machine import QemuMachine

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine

# How long a reboot nothing holds may take to end.
HEALTHY = timedelta(seconds=30)
# How long a reboot held by a service that won't stop may take: systemd's minute, and more.
HELD = timedelta(minutes=2)
# How long the watchdog may take to fire once the last stage is held: its 30 seconds, and more.
WATCHED = timedelta(minutes=1)
# What a reboot held by a service that won't stop takes, in seconds: systemd's minute, with
# room either side for the boot's own pace.
HELD_AT_LEAST = 55
HELD_AT_MOST = 90
# How soon the watchdog may fire, in seconds: its 30, less a margin.
WATCHDOG_AT_LEAST = 25


@dataclass(frozen=True)
class Reboot:
    """A reboot the machine was asked for: when, and how much its console had said before."""

    machine: QemuMachine
    started: float
    said: int

    def took(self) -> float:
        """Give the seconds since the reboot was asked for."""
        return time.monotonic() - self.started

    def told(self) -> list[str]:
        """Give what the machine told Plymouth since the reboot was asked for.

        It reads the kernel's own line, stamped with its time, and not the journal's copy.
        """
        console = self.machine.get_console_log()[self.said :]
        return re.findall(r"\] told plymouth --status=(\w+)", console)


def reboot(machine: QemuMachine) -> Reboot:
    """Ask the machine to reboot, and give when and how much its console had said."""
    said = len(machine.get_console_log())
    machine.succeed("systemctl --no-block reboot")
    return Reboot(machine, time.monotonic(), said)


@pytest.fixture
def up(machine: BaseMachine) -> QemuMachine:
    """Give the machine up, started again when the last test's reboot ended it."""
    assert isinstance(machine, QemuMachine), "the watchdog and the console are QEMU's"
    if not machine.is_up():
        machine.start()
    machine.wait_for_unit("multi-user.target")
    return machine


def test_a_healthy_reboot_tells_plymouth_nothing(up: QemuMachine) -> None:
    """A healthy reboot tells Plymouth nothing."""
    rebooted = reboot(up)
    up.wait_for_shutdown(timeout=HEALTHY)
    assert rebooted.told() == []


def test_a_service_that_wont_stop_holds_a_reboot_a_minute(
    up: QemuMachine, values: dict[str, Any]
) -> None:
    """A service that won't stop holds a reboot a minute, and no more, and says it forced it."""
    sleep = values["sleep"]
    up.succeed(
        "systemd-run --unit=stuck -p Type=exec -p TimeoutStopSec=infinity"
        f" -p ExecStop='{sleep} infinity' {sleep} infinity"
    )
    rebooted = reboot(up)
    up.wait_for_shutdown(timeout=HELD)
    took = rebooted.took()
    assert took > HELD_AT_LEAST, "the reboot didn't wait for the service"
    assert took < HELD_AT_MOST, "the reboot waited past the minute"
    assert rebooted.told() == ["forced"]


def test_a_leftover_that_ignores_sigterm_cant_hold_the_last_stage(
    up: QemuMachine, values: dict[str, Any]
) -> None:
    """A leftover that ignores SIGTERM can't hold the last stage: the watchdog resets it."""
    sleep, sh = values["sleep"], values["sh"]
    up.succeed("test -e /dev/watchdog0")
    up.succeed(
        f"systemd-run --unit=leftover -p KillMode=none {sh} -c"
        f" 'trap \"\" TERM; exec {sleep} infinity'"
    )
    rebooted = reboot(up)
    up.wait_for_qmp_event(lambda event: event["event"] == "WATCHDOG", timeout=WATCHED)
    took = rebooted.took()
    up.wait_for_shutdown(timeout=HEALTHY)
    assert took > WATCHDOG_AT_LEAST, "the watchdog fired before its 30 seconds"
    assert rebooted.told() == ["leftovers"]
