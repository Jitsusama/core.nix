"""The keeps-pace area's judging, proven on figures its runs gave, with no machine to load.

The area holds a launcher under load to its time with the load frozen (keeps_pace.py). Its keys
failed on a builder busy with other checks, which these figures are taken from, while the
desktop's own weights held: the compositor and the shell waited for a processor no longer than
they may (the net, 16 to 19 November).
"""

from __future__ import annotations

import math

from keeps_pace import MIDDLE_MORE, PROCESSORS, bound, builders_share, ticks

# The net's run of aaf54771c2db, whose middle key took 183 ms under load against 29 frozen.
UNDER = 183
FROZEN = 29
# Its bar before the share, in ms: twice 29 and the budget's 50.
OLD_BAR = 108
# A quiet builder's keys: 28 to 36 ms under load against 15 to 18 frozen, and the key the old
# bar refused, at 87 ms, one past its twice the frozen time and the budget.
QUIET_UNDER = 36
QUIET_FROZEN = 18
SLOWED = 87
# A turn's busy ticks, and a builder that ran the guest half of them under load.
BUSY = 1000
HELD_BACK = 2.0
# The VM held to two of the builder's processors on 19 November: stolen and busy ticks under
# load and frozen, and the share they give, the time it ran frozen over the time under load.
HELD_UNDER = (2136, 4760)
HELD_FROZEN = (146, 491)
HELD_SHARE = (1 - 146 / 491) / (1 - 2136 / 4760)
# /proc/stat's first line, for 100 ticks of each field in turn, guests' times included.
STAT = "cpu  100 200 300 400 500 600 700 800 900 1000"
# Its stolen ticks, and its busy ones: user, nice, system, irq, softirq and steal, where idle and
# iowait aren't busy and a guest's time is already in user and nice.
STOLEN = 800
BUSIED = 100 + 200 + 300 + 600 + 700 + 800


def test_a_key_the_builder_held_back_is_refused_without_its_share() -> None:
    """A key the builder held back fails the bar, as it did on the net, with nothing stolen."""
    share = builders_share((0, 0), (0, 0))
    assert share == 1.0, f"with no ticks, the builder's share read {share}"
    assert bound(FROZEN, MIDDLE_MORE, share) < UNDER, (
        f"{UNDER} ms against {FROZEN} frozen held with no share, at {share}"
    )


def test_a_key_the_builder_held_back_holds_by_its_share() -> None:
    """The same key holds once the guest counts half its time under load stolen."""
    share = builders_share((BUSY // 2, BUSY), (0, BUSY))
    assert share == HELD_BACK, f"half the time stolen under load gave a share of {share}"
    assert bound(FROZEN, MIDDLE_MORE, share) >= UNDER, (
        f"{UNDER} ms against {FROZEN} frozen failed at a share of {share}"
    )


def test_a_launcher_the_load_slows_is_refused_on_a_quiet_builder() -> None:
    """A launcher the guest's own load slows fails where the builder stole nothing."""
    share = builders_share((0, BUSY * 4), (0, BUSY))
    assert share == 1.0, f"nothing stolen gave a share of {share}"
    assert bound(QUIET_FROZEN, MIDDLE_MORE, share) >= QUIET_UNDER, (
        f"a quiet builder's {QUIET_UNDER} ms against {QUIET_FROZEN} failed"
    )
    assert bound(QUIET_FROZEN, MIDDLE_MORE, share) < SLOWED, (
        f"a launcher slowed to {SLOWED} ms against {QUIET_FROZEN} frozen held"
    )


def test_the_share_stretches_the_bar_by_itself_and_no_more() -> None:
    """A share of 1 leaves the old bar as it was, and a share of two makes it twice that."""
    unstretched = bound(FROZEN, MIDDLE_MORE, 1.0)
    stretched = bound(FROZEN, MIDDLE_MORE, HELD_BACK)
    assert unstretched == OLD_BAR, f"a share of 1 made the bar {unstretched} ms, not {OLD_BAR}"
    assert stretched == HELD_BACK * OLD_BAR, (
        f"a share of {HELD_BACK} made the bar {stretched} ms, not {HELD_BACK * OLD_BAR}"
    )


def test_the_share_weighs_what_was_stolen_frozen_too() -> None:
    """A builder stealing from both turns stretches the bar by the time it ran frozen over under.

    These are the ticks of a VM held to two of the builder's processors, which stole from the
    frozen turn too, so the share is less than the half stolen under load alone would give.
    """
    share = builders_share(HELD_UNDER, HELD_FROZEN)
    assert math.isclose(share, HELD_SHARE), f"the held VM's ticks gave a share of {share}"


def test_the_share_is_never_under_one_nor_over_the_processors() -> None:
    """The load can't speed the builder up, nor split it past the VM's processors."""
    faster = builders_share((0, BUSY), (BUSY // 2, BUSY))
    split = builders_share((BUSY, BUSY), (0, BUSY))
    assert faster == 1.0, f"less stolen under load than frozen gave a share of {faster}"
    assert split == PROCESSORS, f"every tick stolen under load gave a share of {split}"


def test_stolen_and_busy_ticks_are_read_from_the_kernels_line() -> None:
    """The stolen ticks are steal's, and the busy ones leave out idle, iowait and guests'."""
    stolen, busy = ticks(STAT)
    assert stolen == STOLEN, f"{STAT!r} read as {stolen} stolen ticks"
    assert busy == BUSIED, f"{STAT!r} read as {busy} busy ticks"
