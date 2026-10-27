"""An account in wheel traces its own niri off the CPU with no password, and nobody else does.

tests/niri-off-cpu.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py).
A stand-in named as niri's session is runs as an account in wheel, waiting in epoll for an
eventfd its waker thread writes and then sleeping outside it, over and over. Its trace says
each, with the kernel stack it slept in, a time on the machine's monotonic clock, and who woke
it: the waker through the eventfd, or the timer. An account outside wheel can't trace, an
account can't start another's trace, and a trace is read only by the account it's of.
"""

from __future__ import annotations

import re
from typing import TYPE_CHECKING, Any

import pytest

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine

TRACE = "/run/niri-off-cpu/someone.txt"
RESTORES = "/run/niri-off-cpu/someone-restores.txt"
# The least time off the CPU the trace says, in microseconds.
LEAST_US = 8000
# Three seconds of 50 ms waits in epoll, each followed by a 50 ms sleep, give about thirty of
# each; ten leaves room for a slow machine.
LEAST_SLEEPS = 10
SLEEP = re.compile(r"^off at \d+ for (\d+) us, asleep, lock held by nobody$", re.MULTILINE)
WOKEN_THROUGH = re.compile(r"^woken by waker \d+ through fd \d+ \[eventfd\]$", re.MULTILINE)


@pytest.fixture(scope="module")
def asked(machine: BaseMachine, values: dict[str, Any]) -> str:
    """Give what tracing the stand-in for three seconds, as its own account, said."""
    machine.wait_for_unit("multi-user.target")
    machine.succeed(
        f"systemd-run --uid=someone --unit=stand-in-niri {values['stand_in']} --session"
    )
    machine.wait_until_succeeds("pgrep --uid someone --full '/bin/niri --session$'")
    status, said = machine.execute("runuser -u someone -- niri-off-cpu 3 2>&1")
    assert status == 0, said
    assert said.strip().endswith(TRACE), said
    return said


@pytest.fixture(scope="module")
def traced(machine: BaseMachine, asked: str) -> str:
    """Give what three seconds' trace of the stand-in said."""
    assert asked
    return machine.succeed(f"runuser -u someone -- cat {TRACE}")


def test_graphics_buffers_read_back_are_counted_beside_it(machine: BaseMachine, asked: str) -> None:
    """The graphics buffers reclaim backed up, read back and kept are counted beside the trace."""
    assert asked.split()[-2:] == [RESTORES, TRACE], asked
    counted = machine.succeed(f"runuser -u someone -- cat {RESTORES}")
    assert "backed up, by shared and caching" in counted, counted
    assert "read back, by thread, shared and caching" in counted, counted
    assert "kept resident by the shrinker as busy, by caching" in counted, counted
    assert "held at once by the shrinker as busy, by caching" in counted, counted
    # The Intel driver is loaded, so none of its probes went missing, which bpftrace would have
    # warned of in the unit's journal.
    said = machine.succeed("journalctl --boot --unit 'niri-off-cpu@*'")
    assert "xe_bo_shrink" not in said, said
    assert "xe_ttm_bo_destroy" not in said, said


def test_each_sleep_is_said_with_its_kernel_stack(traced: str) -> None:
    """Each sleep is said with its kernel stack."""
    sleeps = SLEEP.findall(traced)
    assert len(sleeps) >= LEAST_SLEEPS, traced
    assert all(int(us) >= LEAST_US for us in sleeps), traced
    assert "nanosleep" in traced, traced


def spells(traced: str, slept_in: str) -> list[str]:
    """Give each spell off the CPU the trace says whose own stack names where it slept."""
    return [spell for spell in traced.split("off at ")[1:] if slept_in in spell.split("woken")[0]]


def test_a_wait_in_epoll_says_who_woke_it_and_through_which_file(traced: str) -> None:
    """A wait in epoll says who woke it and through which file."""
    waits = spells(traced, "do_epoll_wait")
    assert len(waits) >= LEAST_SLEEPS, traced
    woken = [wait for wait in waits if WOKEN_THROUGH.search(wait)]
    assert len(woken) >= LEAST_SLEEPS, traced
    assert all("eventfd_write" in wait.split("woken")[1] for wait in woken), traced


def test_a_sleep_says_the_timer_that_woke_it(traced: str) -> None:
    """A sleep says the timer that woke it."""
    sleeps = spells(traced, "nanosleep")
    assert sleeps, traced
    assert all("hrtimer_wakeup" in sleep.split("woken")[1] for sleep in sleeps), traced


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
    # polkit refuses both, asking for a password nobody can give here,
    # rather than either failing for want of a niri. systemctl words the
    # refusal by whether it was asked from a terminal.
    status, said = machine.execute("runuser -u other -- niri-off-cpu 1 2>&1")
    assert status != 0, said
    assert "authentication" in said, said
    status, said = machine.execute(
        "runuser -u someone -- systemctl start niri-off-cpu@other.service 2>&1"
    )
    assert status != 0, said
    assert "authentication" in said, said
    machine.fail(f"runuser -u other -- cat {TRACE}")
