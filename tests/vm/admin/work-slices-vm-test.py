"""The machine's administrators set the work slices' properties with no password, and no more.

work-slices.nix, beside this, runs this through pytest inside the driver (tests/lib/vm_pytest.py).
An account in wheel sets build.slice's and machine.slice's weight and cores for as long as
the machine runs, as an agent measuring them would, outside any session. An account outside
wheel can't, and an administrator still can't change any other unit without a password.
A process where agents' work runs, agents.slice, jobs.service, jobs-work.slice, jobs-timed.slice
or shared.slice, is refused outright what would ask, so
no agent puts a password prompt on the screen, and keeps what a rule grants unasked.
Each test puts what it changed back as it ends, so any runs alone.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

import pytest

if TYPE_CHECKING:
    from collections.abc import Iterator

    from test_driver.machine import BaseMachine


def set_property(machine: BaseMachine, account: str, unit: str, setting: str) -> tuple[int, str]:
    """Set a unit's property for this boot as an account, and give the status and all it said."""
    return machine.execute(
        f"runuser -u {account} -- systemctl set-property --runtime {unit} {setting} 2>&1"
    )


def shown(machine: BaseMachine, unit: str, prop: str) -> str:
    """Give a unit's property as systemd has it."""
    return machine.succeed(f"systemctl show -P {prop} {unit}").strip()


@pytest.fixture(scope="module")
def up(machine: BaseMachine) -> BaseMachine:
    """Give the machine once it has booted, with both work slices loaded."""
    machine.wait_for_unit("multi-user.target")
    machine.succeed("systemctl start build.slice machine.slice")
    return machine


@pytest.fixture
def reverted(up: BaseMachine) -> Iterator[BaseMachine]:
    """Give the machine, and put every runtime property back as the test ends."""
    yield up
    for unit in ("build.slice", "machine.slice", "system.slice"):
        up.succeed(f"systemctl set-property --runtime {unit} CPUWeight= AllowedCPUs=")


@pytest.mark.parametrize("unit", ["build.slice", "machine.slice"])
def test_an_administrator_sets_a_work_slice_without_a_password(
    reverted: BaseMachine, unit: str
) -> None:
    status, said = set_property(reverted, "someone", unit, "CPUWeight=idle AllowedCPUs=0")
    assert status == 0, said
    assert shown(reverted, unit, "CPUWeight") == "idle"
    assert shown(reverted, unit, "AllowedCPUs") == "0"


def test_an_account_outside_wheel_is_refused(reverted: BaseMachine) -> None:
    status, said = set_property(reverted, "other", "build.slice", "CPUWeight=idle")
    assert status != 0, said
    assert shown(reverted, "build.slice", "CPUWeight") != "idle"


def checked_in(machine: BaseMachine, placed: str | None, action: str) -> int:
    """Give pkcheck's status for someone's shell, placed by systemd-run or not run by it."""
    check = f"pkcheck --action-id {action} --process $$ 2>&1"
    if placed is None:
        status, said = machine.execute(f"runuser -u someone -- sh -c '{check}'")
    else:
        # systemd reads a dollar in a unit's command as its own, so the
        # shell's pid is written with two, and the transient unit's PATH
        # holds only systemd's own programs.
        check = check.replace("$$", "$$$$").replace("pkcheck", "/run/current-system/sw/bin/pkcheck")
        status, said = machine.execute(
            f"systemd-run --quiet --wait --pipe {placed} --uid=someone sh -c '{check}'"
        )
    print(f"pkcheck from {placed}: {status} {said}")
    return status


# pkcheck says 1 for a refusal and 2 where polkit would ask for a password.
REFUSED = 1
ASKS = 2


@pytest.mark.parametrize(
    ("placed", "place"),
    [
        ("--slice=agents.slice", "/agents.slice/"),
        ("--slice=jobs.slice --unit=jobs", "/jobs.slice/jobs.service"),
        ("--slice=jobs-work.slice", "/jobs-work.slice/"),
        ("--slice=jobs-timed.slice", "/jobs-timed.slice/"),
        ("--slice=shared.slice", "/shared.slice/"),
    ],
)
def test_an_agent_is_refused_what_would_ask_for_a_password(
    up: BaseMachine, placed: str, place: str
) -> None:
    action = "org.freedesktop.systemd1.manage-units"
    assert checked_in(up, placed, action) == REFUSED
    said = up.succeed("journalctl -t polkit-asks --no-pager -o cat")
    assert f"refused {action} to pid" in said, said
    assert place in said, said


def test_outside_the_agents_slices_polkit_still_asks(up: BaseMachine) -> None:
    action = "org.freedesktop.systemd1.manage-units"
    assert checked_in(up, None, action) == ASKS
    assert checked_in(up, "--slice=other.slice", action) == ASKS
    assert checked_in(up, "--slice=jobs.slice --unit=awaited", action) == ASKS


def test_an_agent_keeps_what_a_rule_grants_unasked(reverted: BaseMachine) -> None:
    status, said = reverted.execute(
        "systemd-run --quiet --wait --pipe --slice=agents.slice --uid=someone "
        "systemctl set-property --runtime build.slice CPUWeight=idle 2>&1"
    )
    assert status == 0, said
    assert shown(reverted, "build.slice", "CPUWeight") == "idle"


def test_an_administrator_still_needs_a_password_for_any_other_unit(
    reverted: BaseMachine,
) -> None:
    status, said = set_property(reverted, "someone", "system.slice", "CPUWeight=idle")
    assert status != 0, said
    assert shown(reverted, "system.slice", "CPUWeight") != "idle"
