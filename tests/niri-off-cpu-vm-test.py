"""An account in wheel traces its own niri off the CPU with no password, and nobody else does.

tests/niri-off-cpu.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py).
A stand-in named as niri's session is runs as an account in wheel, waiting in epoll and then
sleeping outside it, over and over. Its trace says each sleep, with the kernel stack it slept
in and a time on the machine's monotonic clock, and leaves the waits in epoll out. An account
outside wheel can't trace, an account can't start another's trace, and a trace is read only
by the account it's of.
"""

from __future__ import annotations

import re
from typing import TYPE_CHECKING, Any

import pytest

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine

TRACE = "/run/niri-off-cpu/someone.txt"
# The least time off the CPU the trace says, in microseconds.
LEAST_US = 8000
# Three seconds of 50 ms sleeps, each between two 50 ms waits, give about thirty; ten leaves
# room for a slow machine.
LEAST_SLEEPS = 10
SLEEP = re.compile(r"^off at \d+ for (\d+) us, asleep, lock held by nobody$", re.MULTILINE)


@pytest.fixture(scope="module")
def traced(machine: BaseMachine, values: dict[str, Any]) -> str:
    """Give what three seconds' trace of the stand-in said, traced by its own account."""
    machine.wait_for_unit("multi-user.target")
    machine.succeed(
        f"systemd-run --uid=someone --unit=stand-in-niri {values['stand_in']} --session"
    )
    machine.wait_until_succeeds("pgrep --uid someone --full '/bin/niri --session$'")
    status, said = machine.execute("runuser -u someone -- niri-off-cpu 3 2>&1")
    assert status == 0, said
    assert said.strip().endswith(TRACE), said
    return machine.succeed(f"runuser -u someone -- cat {TRACE}")


def test_each_sleep_is_said_with_its_kernel_stack(traced: str) -> None:
    """Each sleep is said with its kernel stack."""
    sleeps = SLEEP.findall(traced)
    assert len(sleeps) >= LEAST_SLEEPS, traced
    assert all(int(us) >= LEAST_US for us in sleeps), traced
    assert "nanosleep" in traced, traced


def test_waiting_in_epoll_is_left_out(traced: str) -> None:
    """Waiting in epoll is left out."""
    assert "epoll" not in traced, traced


def test_times_are_on_the_machines_monotonic_clock(machine: BaseMachine, traced: str) -> None:
    """Times are on the machine's monotonic clock."""
    times = [int(at) for at in re.findall(r"^off at (\d+) ", traced, re.MULTILINE)]
    assert times, traced
    now = int(float(machine.succeed("cut -d ' ' -f 1 /proc/uptime")) * 1e9)
    assert all(0 < at < now for at in times), (times, now)
    assert times == sorted(times), times


@pytest.mark.usefixtures("traced")
def test_nobody_else_traces_or_reads_it(machine: BaseMachine) -> None:
    """Nobody else traces it or reads it."""
    # polkit refuses both, rather than either failing for want of a niri.
    status, said = machine.execute("runuser -u other -- niri-off-cpu 1 2>&1")
    assert status != 0, said
    assert "authentication required" in said, said
    status, said = machine.execute(
        "runuser -u someone -- systemctl start niri-off-cpu@other.service 2>&1"
    )
    assert status != 0, said
    assert "authentication required" in said, said
    machine.fail(f"runuser -u other -- cat {TRACE}")
