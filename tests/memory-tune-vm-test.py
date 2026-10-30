"""The machine's administrators tune the kernel's memory with no password, and no more.

tests/memory-tune.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py).
An account in wheel sets the watermarks' spacing, transparent huge pages and the multi-gen
LRU to a listed value, as an agent measuring them under load would, outside any session. An
account outside wheel can't, and an administrator can't set a value the rules don't list.
Each test puts what it changed back as it ends, so any runs alone.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

import pytest

if TYPE_CHECKING:
    from collections.abc import Iterator

    from test_driver.machine import BaseMachine

SCALE = "/proc/sys/vm/watermark_scale_factor"
HUGEPAGES = "/sys/kernel/mm/transparent_hugepage/enabled"
LRU_GEN = "/sys/kernel/mm/lru_gen/enabled"


def start(machine: BaseMachine, account: str, unit: str) -> tuple[int, str]:
    """Start a unit as an account, and give the status and all it said."""
    return machine.execute(f"runuser -u {account} -- systemctl start {unit} 2>&1")


def read(machine: BaseMachine, file: str) -> str:
    """Give a kernel setting as its file says it."""
    return machine.succeed(f"cat {file}").strip()


def chosen(setting: str) -> str:
    """Give the bracketed choice of a setting that lists every choice, as huge pages do."""
    return setting.split("[", 1)[1].split("]", 1)[0]


@pytest.fixture
def reverted(machine: BaseMachine) -> Iterator[BaseMachine]:
    """Give the booted machine, and put every setting back as the test ends."""
    machine.wait_for_unit("multi-user.target")
    scale = read(machine, SCALE)
    hugepages = chosen(read(machine, HUGEPAGES))
    lru_gen = read(machine, LRU_GEN)
    yield machine
    machine.succeed(f"sysctl -w vm.watermark_scale_factor={scale}")
    machine.succeed(f"echo {hugepages} > {HUGEPAGES}")
    machine.succeed(f"echo {lru_gen} > {LRU_GEN}")


def test_an_administrator_spaces_the_watermarks_without_a_password(
    reverted: BaseMachine,
) -> None:
    status, said = start(reverted, "someone", "memory-watermarks@50")
    assert status == 0, said
    assert read(reverted, SCALE) == "50"


def test_an_administrator_chooses_huge_pages_without_a_password(reverted: BaseMachine) -> None:
    status, said = start(reverted, "someone", "memory-hugepages@madvise")
    assert status == 0, said
    assert chosen(read(reverted, HUGEPAGES)) == "madvise"


def test_an_administrator_turns_the_multi_gen_lru_off_without_a_password(
    reverted: BaseMachine,
) -> None:
    status, said = start(reverted, "someone", "memory-lru-gen@n")
    assert status == 0, said
    assert read(reverted, LRU_GEN) == "0x0000"


def test_an_account_outside_wheel_is_refused(reverted: BaseMachine) -> None:
    before = read(reverted, SCALE)
    status, said = start(reverted, "other", "memory-watermarks@50")
    assert status != 0, said
    assert read(reverted, SCALE) == before


def test_a_value_the_rules_dont_list_needs_a_password(reverted: BaseMachine) -> None:
    before = read(reverted, SCALE)
    status, said = start(reverted, "someone", "memory-watermarks@3000")
    assert status != 0, said
    assert read(reverted, SCALE) == before
