"""The machine's administrators set the work slices' properties with no password, and no more.

tests/work-slices.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py).
An account in wheel sets build.slice's and machine.slice's weight and cores for as long as
the machine runs, as an agent measuring them would, outside any session. An account outside
wheel can't, and an administrator still can't change any other unit without a password.
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


def test_an_administrator_still_needs_a_password_for_any_other_unit(
    reverted: BaseMachine,
) -> None:
    status, said = set_property(reverted, "someone", "system.slice", "CPUWeight=idle")
    assert status != 0, said
    assert shown(reverted, "system.slice", "CPUWeight") != "idle"
