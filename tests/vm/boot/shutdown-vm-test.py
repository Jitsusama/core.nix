"""A reboot whose shutdown would hang goes on anyway, and says what it told Plymouth.

shutdown.nix, beside this, runs this through pytest inside the driver (tests/lib/vm_pytest.py), and
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
# How long an account's manager may take to stop, in seconds, whatever its units' own limits.
ACCOUNT_STOPS_WITHIN = 10
# How long the account's stubborn units may take to start.
STARTING = timedelta(seconds=30)
# The account, its manager, and how root reaches that manager.
ACCOUNT = "alice"
MANAGER = "user@1000.service"
AS_ACCOUNT = f"-M {ACCOUNT}@ --user"
# What a unit is given to stop when it names no limit of its own: the account's units were
# started with this, so the manager's own cap is what's proven.
DEFAULT_LIMIT = "1min 30s"


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
    # The reboot can close the test's shell before the shell says how
    # systemctl exited, so the ask doesn't wait for an answer; every
    # caller waits for the shutdown instead.
    machine.execute("systemctl --no-block reboot", check_output=False)
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


@pytest.fixture
def stubborn_account(up: QemuMachine, values: dict[str, Any]) -> QemuMachine:
    """Give the machine with the account's manager holding units that ignore SIGTERM.

    A service and a scope, each with the 90 second stop limit a unit gets when it names none.
    """
    sleep, stubborn = values["sleep"], values["stubborn"]
    up.succeed(f"systemctl start {MANAGER}")
    up.succeed(f"systemd-run {AS_ACCOUNT} --unit=stubborn {stubborn} {sleep}")
    # A scope takes the process that asks for it, so a service of the account's asks.
    up.succeed(
        f"systemd-run {AS_ACCOUNT} --unit=scoper"
        " /run/current-system/sw/bin/systemd-run --user --scope --unit=stubborn-scope"
        f" {stubborn} {sleep}"
    )
    units = "stubborn.service stubborn-scope.scope"
    up.wait_until_succeeds(f"systemctl {AS_ACCOUNT} is-active {units}", timeout=STARTING)
    for unit in units.split():
        limit = up.succeed(f"systemctl {AS_ACCOUNT} show -P TimeoutStopUSec {unit}").strip()
        assert limit == DEFAULT_LIMIT, f"{unit} was started with {limit} to stop"
    return up


def test_an_accounts_units_that_wont_stop_hold_its_manager_seconds(
    stubborn_account: QemuMachine,
) -> None:
    """An account's units that ignore SIGTERM hold its manager seconds, then are killed."""
    started = time.monotonic()
    stubborn_account.succeed(f"systemctl stop {MANAGER}")
    took = time.monotonic() - started
    assert took < ACCOUNT_STOPS_WITHIN, f"the account's manager took {took:.1f} s to stop"
    stubborn_account.fail(f"pgrep -u {ACCOUNT}")


def test_an_accounts_units_that_wont_stop_cant_hold_a_reboot(
    stubborn_account: QemuMachine,
) -> None:
    """An account's units that ignore SIGTERM can't hold a reboot, which tells Plymouth nothing."""
    rebooted = reboot(stubborn_account)
    stubborn_account.wait_for_shutdown(timeout=HEALTHY)
    assert rebooted.told() == []
