"""The machine's administrators tune the kernel's memory with no password, and no more.

memory-tune.nix, beside this, runs this through pytest inside the driver (tests/lib/vm_pytest.py).
An account in wheel sets the watermarks' spacing, transparent huge pages, the multi-gen LRU,
swappiness, swap's read-ahead, proactive compaction, the free memory floor, defrag_mode and
zswap's shrinker to a listed value, as an agent measuring them under load would, outside any
session. An account outside wheel can't, and an administrator can't set a value the rules
don't list.
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
SWAPPINESS = "/proc/sys/vm/swappiness"
PAGE_CLUSTER = "/proc/sys/vm/page-cluster"
COMPACTION = "/proc/sys/vm/compaction_proactiveness"
MIN_FREE = "/proc/sys/vm/min_free_kbytes"
DEFRAG = "/proc/sys/vm/defrag_mode"
ZSWAP_SHRINKER = "/sys/module/zswap/parameters/shrinker_enabled"


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
    swappiness = read(machine, SWAPPINESS)
    page_cluster = read(machine, PAGE_CLUSTER)
    compaction = read(machine, COMPACTION)
    min_free = read(machine, MIN_FREE)
    defrag = read(machine, DEFRAG)
    zswap_shrinker = read(machine, ZSWAP_SHRINKER)
    yield machine
    machine.succeed(f"echo {zswap_shrinker} > {ZSWAP_SHRINKER}")
    machine.succeed(f"sysctl -w vm.defrag_mode={defrag}")
    machine.succeed(f"sysctl -w vm.min_free_kbytes={min_free}")
    machine.succeed(f"sysctl -w vm.compaction_proactiveness={compaction}")
    machine.succeed(f"sysctl -w vm.page-cluster={page_cluster}")
    machine.succeed(f"sysctl -w vm.swappiness={swappiness}")
    machine.succeed(f"sysctl -w vm.watermark_scale_factor={scale}")
    machine.succeed(f"echo {hugepages} > {HUGEPAGES}")
    machine.succeed(f"echo {lru_gen} > {LRU_GEN}")


def test_an_administrator_spaces_the_watermarks_without_a_password(
    reverted: BaseMachine,
) -> None:
    status, said = start(reverted, "someone", "memory-watermarks@100")
    assert status == 0, said
    assert read(reverted, SCALE) == "100"


def test_an_administrator_chooses_huge_pages_without_a_password(reverted: BaseMachine) -> None:
    status, said = start(reverted, "someone", "memory-hugepages@always")
    assert status == 0, said
    assert chosen(read(reverted, HUGEPAGES)) == "always"


def test_an_administrator_turns_the_multi_gen_lru_off_without_a_password(
    reverted: BaseMachine,
) -> None:
    status, said = start(reverted, "someone", "memory-lru-gen@n")
    assert status == 0, said
    assert read(reverted, LRU_GEN) == "0x0000"


def test_an_administrator_sets_swappiness_without_a_password(reverted: BaseMachine) -> None:
    status, said = start(reverted, "someone", "memory-swappiness@100")
    assert status == 0, said
    assert read(reverted, SWAPPINESS) == "100", "swappiness wasn't set"


def test_the_machine_boots_swapping_at_60(reverted: BaseMachine) -> None:
    swappiness = read(reverted, SWAPPINESS)
    assert swappiness == "60", f"booted with swappiness at {swappiness}"


def test_an_administrator_sets_swaps_read_ahead_without_a_password(reverted: BaseMachine) -> None:
    status, said = start(reverted, "someone", "memory-page-cluster@3")
    assert status == 0, said
    assert read(reverted, PAGE_CLUSTER) == "3", "page-cluster wasn't set"


def test_an_administrator_sets_proactive_compaction_without_a_password(
    reverted: BaseMachine,
) -> None:
    status, said = start(reverted, "someone", "memory-compaction@60")
    assert status == 0, said
    assert read(reverted, COMPACTION) == "60", "compaction_proactiveness wasn't set"


def test_the_machine_boots_compacting_ahead_at_40(reverted: BaseMachine) -> None:
    compaction = read(reverted, COMPACTION)
    assert compaction == "40", f"booted with compaction_proactiveness at {compaction}"


def test_an_administrator_sets_the_free_memory_floor_without_a_password(
    reverted: BaseMachine,
) -> None:
    status, said = start(reverted, "someone", "memory-min-free@131072")
    assert status == 0, said
    assert read(reverted, MIN_FREE) == "131072", "min_free_kbytes wasn't set"


def test_an_administrator_turns_defrag_mode_on_and_off_without_a_password(
    reverted: BaseMachine,
) -> None:
    for mode in ("1", "0"):
        status, said = start(reverted, "someone", f"memory-defrag-mode@{mode}")
        assert status == 0, said
        assert read(reverted, DEFRAG) == mode, f"defrag_mode wasn't set to {mode}"


def test_the_machine_boots_with_zswaps_shrinker_on(reverted: BaseMachine) -> None:
    shrinker = read(reverted, ZSWAP_SHRINKER)
    assert shrinker == "Y", f"booted with zswap's shrinker at {shrinker}"


def test_an_administrator_turns_zswaps_shrinker_on_and_off_without_a_password(
    reverted: BaseMachine,
) -> None:
    for given, said_back in (("n", "N"), ("y", "Y")):
        status, said = start(reverted, "someone", f"memory-zswap-shrinker@{given}")
        assert status == 0, said
        shrinker = read(reverted, ZSWAP_SHRINKER)
        assert shrinker == said_back, f"zswap's shrinker read {shrinker} after {given}"


def test_an_account_outside_wheel_is_refused(reverted: BaseMachine) -> None:
    before = read(reverted, SCALE)
    status, said = start(reverted, "other", "memory-watermarks@100")
    assert status != 0, said
    assert read(reverted, SCALE) == before


def test_a_value_the_rules_dont_list_needs_a_password(reverted: BaseMachine) -> None:
    before = read(reverted, SCALE)
    status, said = start(reverted, "someone", "memory-watermarks@3000")
    assert status != 0, said
    assert read(reverted, SCALE) == before


def test_a_defrag_mode_the_rules_dont_list_needs_a_password(reverted: BaseMachine) -> None:
    before = read(reverted, DEFRAG)
    status, said = start(reverted, "someone", "memory-defrag-mode@2")
    assert status != 0, said
    assert read(reverted, DEFRAG) == before, "an unlisted defrag_mode was written"


def test_a_zswap_shrinker_value_the_rules_dont_list_needs_a_password(
    reverted: BaseMachine,
) -> None:
    # It boots on, so a 0 written through would read back changed, as N.
    before = read(reverted, ZSWAP_SHRINKER)
    status, said = start(reverted, "someone", "memory-zswap-shrinker@0")
    assert status != 0, said
    assert read(reverted, ZSWAP_SHRINKER) == before, "an unlisted shrinker value was written"
