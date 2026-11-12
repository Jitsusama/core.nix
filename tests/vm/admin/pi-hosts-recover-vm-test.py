"""A pi host's unit restarts a wedged host and leaves one that keeps crashing down.

pi-hosts-recover.nix, beside this, runs this through pytest inside the driver
(tests/lib/vm_pytest.py), on a machine whose account lingers with a realm's hosts as their module
renders them, their node a stand-in that says ready and pings the watchdog each second until a file
tells it to hang or crash. Each test starts the background host afresh, its start limit cleared,
so any test runs alone with `-k`. The tests need no value from Nix.
"""

from __future__ import annotations

import datetime as dt
import shlex
from typing import TYPE_CHECKING

import pytest

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine

UNIT = "pi-host-test-background.service"
# The stand-in's folder, where each start writes its pid and the test leaves what it's told.
FOLDER = "/home/alice/stand-in"
STARTS = f"{FOLDER}/background.starts"
# The host program the unit's condition looks for, which the harness's install puts there.
ENTRY = (
    "/home/alice/.local/share/pi-packages/github.com/Jitsusama/agentic-harness.pi"
    "/current/scripts/pi-host.ts"
)
# How many starts the unit is given in ten minutes before it's left down.
START_LIMIT = 3
# How long a start, a watchdog's kill and the five seconds before the next start may take.
COMING_BACK = dt.timedelta(seconds=60)
# How long a host left down is watched for a start it shouldn't get: past two restart delays.
STAYING_DOWN = dt.timedelta(seconds=15)


def as_alice(command: str) -> str:
    """Give a command that runs as alice, reaching her user manager."""
    return "su - alice -c " + shlex.quote(f"export XDG_RUNTIME_DIR=/run/user/1000; {command}")


class Host:
    """The background host of alice's realm, as her user manager runs it."""

    def __init__(self, machine: BaseMachine) -> None:
        """Act on the background host on that machine."""
        self.machine = machine

    def run(self, command: str) -> str:
        """Run a command as alice and give what it printed."""
        return self.machine.succeed(as_alice(command))

    def show(self, field: str) -> str:
        """Give one of the unit's properties, as systemd says it."""
        return self.run(f"systemctl --user show -p {field} --value {UNIT}").strip()

    def starts(self) -> list[str]:
        """Give the pid of each start the stand-in made, oldest first."""
        return self.run(f"cat {STARTS} 2>/dev/null || true").split()

    def until(self, command: str) -> None:
        """Wait until a command run as alice succeeds."""
        self.machine.wait_until_succeeds(as_alice(command), timeout=COMING_BACK)

    def said(self) -> str:
        """Give what alice's manager told the journal about the unit."""
        # --user-unit matches the caller's own uid, which is root's here.
        return self.machine.succeed(f"journalctl --no-pager _UID=1000 USER_UNIT={UNIT}")

    def afresh(self) -> None:
        """Start the host with nothing told, its starts and its start limit cleared."""
        self.run(f"systemctl --user stop {UNIT}; systemctl --user reset-failed {UNIT} || true")
        self.run(f"rm -rf {FOLDER}; mkdir -p {FOLDER}")
        self.run(f"systemctl --user start {UNIT}")
        self.until(f"systemctl --user is-active {UNIT}")


@pytest.fixture(scope="module")
def booted(machine: BaseMachine) -> Host:
    """Boot the machine with alice's manager running and the harness's host program there."""
    machine.wait_for_unit("multi-user.target")
    machine.wait_for_unit("user@1000.service")
    host = Host(machine)
    host.run(f"mkdir -p {shlex.quote(ENTRY.rsplit('/', 1)[0])} && touch {shlex.quote(ENTRY)}")
    return host


@pytest.fixture
def host(booted: Host) -> Host:
    """Start the background host afresh for this test."""
    booted.afresh()
    return booted


def test_a_host_whose_loop_stops_pinging_is_killed_and_started_again(host: Host) -> None:
    """A host that stays alive but stops pinging is killed by the watchdog and started again."""
    [first] = host.starts()
    host.run(f"touch {FOLDER}/hang")
    host.until(f"test -e {FOLDER}/hung && test $(wc -l < {STARTS}) -gt 1")
    host.until(f"systemctl --user is-active {UNIT}")
    said = host.said()
    assert "Failed with result 'watchdog'" in said, said
    # The watchdog's signal, which the harness's killed-host test sends its host too.
    assert "status=6/ABRT" in said, said
    assert host.starts()[0] == first
    assert first not in host.starts()[1:]


def test_a_host_that_keeps_crashing_stays_down_after_three_starts(host: Host) -> None:
    """A host crashing at each start is started three times in ten minutes, then left down.

    The first start is the test's own, so a host up for ten minutes stays down at its fourth crash.
    """
    host.run(f"touch {FOLDER}/crash")
    host.until(f"systemctl --user show -p Result --value {UNIT} | grep -qx start-limit-hit")
    assert len(host.starts()) == START_LIMIT, host.starts()
    assert host.show("ActiveState") == "failed"
    # Nothing is to happen, so the test waits out the time a restart would come in.
    host.machine.sleep(int(STAYING_DOWN.total_seconds()))
    assert len(host.starts()) == START_LIMIT, host.starts()
    assert host.show("ActiveState") == "failed"
