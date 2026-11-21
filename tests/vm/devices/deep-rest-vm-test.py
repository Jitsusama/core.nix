"""The cores' deep-rest hold bounds their exit latency at 20 us from boot, and only while it runs.

deep-rest.nix, beside this, runs this through pytest inside the driver (tests/lib/vm_pytest.py), on
a machine with the XPS 14's hold and nothing else. Reading /dev/cpu_dma_latency gives the tightest
bound any open request asks for, as a four-byte number, and opening it to read asks for none, so
the read shows the hold's own. The fixture reads the machine as it booted, before any test starts
or stops the hold, and each test then sets the hold as it needs, so any runs alone with `-k`.
Whether the hold's bound stops short of C6 on the laptop, and whether its setting turns it off, are
checks of the configuration (hardware/dell-xps-14-da14260/checks.nix). The tests need no value from
Nix.
"""

from __future__ import annotations

from typing import TYPE_CHECKING, NamedTuple

import pytest

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine

UNIT = "cores-kept-from-deep-rest.service"
RESIDENCY = "cores-rest-residency.service"
# The bound the hold asks for, in microseconds, under C6S's 300 and above C1E's 10.
HELD = 20
# The kernel's bound when nothing asks for one, PM_QOS_CPU_LATENCY_DEFAULT_VALUE.
UNBOUND = 2_000_000_000
READ = "od -A n -t d4 -N 4 /dev/cpu_dma_latency"


class Booted(NamedTuple):
    """The machine, with the hold's state and the kernel's bound as it booted."""

    machine: BaseMachine
    state: str
    bound: int


def bound(machine: BaseMachine) -> int:
    """Read the kernel's processor latency bound now, in microseconds."""
    return int(machine.succeed(READ).strip())


@pytest.fixture(scope="module")
def booted(machine: BaseMachine) -> Booted:
    """Boot, and read the hold and the bound before anything touches them."""
    machine.wait_for_unit("multi-user.target")
    state = machine.execute(f"systemctl is-active {UNIT}")[1].strip()
    return Booted(machine, state, bound(machine))


def test_boot_starts_the_hold(booted: Booted) -> None:
    """A boot starts the hold, and the cores' exit latency is bounded at 20 us."""
    assert booted.state == "active"
    assert booted.bound == HELD


def test_stopped_it_lets_them_sink_at_once(booted: Booted) -> None:
    """Stopped, the bound goes with it, so the cores rest as deep as they can again."""
    machine = booted.machine
    machine.succeed(f"systemctl stop {UNIT}")
    assert bound(machine) == UNBOUND


def test_started_it_holds_by_the_time_the_start_returns(booted: Booted) -> None:
    """Started by hand, the bound is 20 us as the start returns, and stays."""
    machine = booted.machine
    # Ready only once the script says so, after it reads the bound back.
    assert machine.succeed(f"systemctl show -P Type {UNIT}").strip() == "notify"
    machine.succeed(f"systemctl stop {UNIT}")
    held = machine.succeed(f"systemctl start {UNIT} && {READ}").strip()
    assert int(held) == HELD
    machine.succeed("sleep 2")  # A moment for a hold that let go to show it.
    machine.succeed(f"systemctl is-active {UNIT}")
    assert bound(machine) == HELD


@pytest.mark.parametrize("signal", ["TERM", "KILL"])
def test_a_killed_hold_comes_back(booted: Booted, signal: str) -> None:
    """A hold whose sleep is killed, by a plain kill or by force, starts again."""
    machine = booted.machine
    machine.succeed(f"systemctl start {UNIT}")
    first = machine.succeed(f"systemctl show -P InvocationID {UNIT}").strip()
    machine.succeed(f"systemctl kill --signal={signal} {UNIT}")
    machine.wait_until_succeeds(
        f'[ "$(systemctl show -P InvocationID {UNIT})" != {first} ] && systemctl is-active {UNIT}',
        timeout=30,
    )
    assert bound(machine) == HELD


def test_the_journal_says_how_long_each_state_rested(booted: Booted) -> None:
    """The residency line, written in its confinement, names the states root sees.

    deep-rest.nix gives the machine haltpoll's rest states, so the line is read over real ones,
    and a confinement that hid them would fail the unit rather than pass this.
    """
    machine = booted.machine
    driver = machine.succeed("cat /sys/devices/system/cpu/cpuidle/current_driver").strip()
    assert driver == "haltpoll", f"the machine's cpuidle driver is {driver}, not haltpoll"
    first = machine.succeed("cat /sys/devices/system/cpu/cpu0/cpuidle/state0/name").strip()
    machine.succeed(f"systemctl start {RESIDENCY}")
    machine.succeed("journalctl --sync")
    said = machine.succeed(f"journalctl -b -o cat -u {RESIDENCY}")
    cores = machine.succeed("ls -d /sys/devices/system/cpu/cpu[0-9]*/cpuidle | wc -l").strip()
    assert f"At rest since boot, {cores} cores: {first} (out in " in said, said
