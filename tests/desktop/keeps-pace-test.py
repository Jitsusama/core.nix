"""How the keeps-pace area counts what the builder took from its VM, with no machine to load.

The area times a launcher under load against the load frozen, and each subtest says how much of
the guest's busy time the kernel counted stolen in each turn (keeps_pace.py), so a run that fails
on the net shows whether the builder took the time or the shell spent it.
"""

from __future__ import annotations

import math

from keeps_pace import PROCESSORS, Ledger, builders_share, ticks

# A turn's busy ticks, and a builder that ran the guest half of them under load.
BUSY = 1000
HALVED = 2.0
# The VM held to two of the builder's processors on 19 November: stolen and busy ticks under
# load and frozen, and the share they give, the time it ran frozen over the time under load.
HELD_UNDER = (2136, 4760)
HELD_FROZEN = (146, 491)
HELD_SHARE = 1.27
# /proc/stat's first line, for 100 ticks of each field in turn, guests' times included.
STAT = "cpu  100 200 300 400 500 600 700 800 900 1000"
# Its stolen ticks, and its busy ones: user, nice, system, irq, softirq and steal, where idle and
# iowait aren't busy and a guest's time is already in user and nice.
STOLEN = 800
BUSIED = 100 + 200 + 300 + 600 + 700 + 800


def stat(stolen: int, busy: int) -> str:
    """Give a /proc/stat first line counting these stolen ticks and these busy ones."""
    return f"cpu  {busy - stolen} 0 0 5000 0 0 0 {stolen} 0 0"


def test_nothing_stolen_gives_a_share_of_one() -> None:
    """A builder stealing nothing in either turn took nothing the guest's weights couldn't."""
    share = builders_share((0, BUSY * 4), (0, BUSY))
    assert share == 1.0, f"nothing stolen gave a share of {share}"


def test_half_stolen_under_load_alone_gives_a_share_of_two() -> None:
    """A builder running the loaded guest half the time it ran the frozen one gives two."""
    share = builders_share((BUSY // 2, BUSY), (0, BUSY))
    assert share == HALVED, f"half the time stolen under load gave a share of {share}"


def test_what_was_stolen_frozen_weighs_against_what_was_under_load() -> None:
    """The held VM's ticks, stolen from both turns, give much less than half stolen would."""
    share = builders_share(HELD_UNDER, HELD_FROZEN)
    assert math.isclose(share, HELD_SHARE, abs_tol=0.005), f"the held ticks gave {share}"


def test_the_share_is_never_under_one_nor_over_the_processors() -> None:
    """The load can't speed the builder up, nor split it past the VM's processors."""
    faster = builders_share((0, BUSY), (BUSY // 2, BUSY))
    split = builders_share((BUSY * 9 // 10, BUSY), (0, BUSY))
    starved = builders_share((BUSY, BUSY), (0, BUSY))
    assert faster == 1.0, f"less stolen under load than frozen gave a share of {faster}"
    assert split == PROCESSORS, f"nine in ten ticks stolen under load gave a share of {split}"
    assert starved == PROCESSORS, f"every tick stolen under load alone gave a share of {starved}"


def test_a_turn_with_no_busy_ticks_gives_a_share_of_one() -> None:
    """With no busy ticks in a turn there's nothing to weigh, rather than a division by none."""
    unloaded = builders_share((0, 0), (0, BUSY))
    unfrozen = builders_share((0, BUSY), (0, 0))
    assert unloaded == 1.0, f"no busy ticks under load gave a share of {unloaded}"
    assert unfrozen == 1.0, f"no busy ticks frozen gave a share of {unfrozen}"


def test_every_tick_stolen_in_both_turns_gives_a_share_of_one() -> None:
    """A builder that ran the guest no more frozen than under load took nothing from the load."""
    share = builders_share((BUSY, BUSY), (BUSY, BUSY))
    assert share == 1.0, f"every tick stolen in both turns gave a share of {share}"


def test_stolen_and_busy_ticks_are_read_from_the_kernels_line() -> None:
    """The stolen ticks are steal's, and the busy ones leave out idle, iowait and guests'."""
    stolen, busy = ticks(STAT)
    assert stolen == STOLEN, f"{STAT!r} read as {stolen} stolen ticks"
    assert busy == BUSIED, f"{STAT!r} read as {busy} busy ticks"


def test_each_count_goes_to_its_own_turn_and_a_turning_to_neither() -> None:
    """Ticks counted under load and frozen land in their turns, and those between, in neither."""
    ledger = Ledger(stat(0, 0))
    ledger.count(stat(10, 100), turn=True)
    ledger.count(stat(40, 400), turn=None)
    ledger.count(stat(45, 450), turn=False)
    ledger.count(stat(65, 650), turn=True)
    assert ledger.turns[True] == (30, 300), f"under load the ledger kept {ledger.turns[True]}"
    assert ledger.turns[False] == (5, 50), f"frozen the ledger kept {ledger.turns[False]}"


def test_the_ledger_says_each_turns_ticks_and_the_share() -> None:
    """What a subtest says names each turn's stolen and busy ticks, and the share they give."""
    ledger = Ledger(stat(0, 0))
    ledger.count(stat(500, 1000), turn=True)
    ledger.count(stat(500, 2000), turn=False)
    said = ledger.said()
    assert said == (
        "the guest counted 500 of 1000 busy ticks stolen under load, against 0 of 1000 frozen,"
        " a share of 2.00"
    ), f"the ledger said {said!r}"


def test_the_ledger_says_no_share_for_a_turn_it_never_counted() -> None:
    """A turn a failure never reached has nothing to weigh, which a share of 1 would clear."""
    ledger = Ledger(stat(0, 0))
    ledger.count(stat(500, 1000), turn=True)
    said = ledger.said()
    assert said == (
        "the guest counted 500 of 1000 busy ticks stolen under load, against 0 of 0 frozen,"
        " so no share"
    ), f"the ledger said {said!r}"
