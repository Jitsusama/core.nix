"""The keeps-pace area's judging, proven on figures its runs gave, with no machine to load.

The area holds a launcher under load to its time with the load frozen (keeps_pace.py). Its keys
failed on a builder busy with other checks, which these figures are taken from, while the
desktop's own weights held: the compositor and the shell waited for a processor no longer than
they may (the net, 16 to 19 November).
"""

from __future__ import annotations

import pytest
from keeps_pace import SLOWER_MOST, bound, builders_share, probed

# The net's run of aaf54771c2db, whose middle key took 183 ms under load against 29 frozen.
UNDER = 183
FROZEN = 29
MIDDLE_MORE = 50
# A quiet builder's keys: 28 to 36 ms under load against 15 to 18 frozen.
QUIET_UNDER = 36
QUIET_FROZEN = 18
# A probe's work, in microseconds: as long under load as frozen, on a quiet builder, and on a
# builder holding the guest's processors back under load, twice as long.
WORK = 2000.0
HELD_BACK = 2.0
# The VM's processors, which the builder splits its share between: one busy frozen and every
# one under load, so the most the load's stretch can be.
PROCESSORS = 4


def test_a_key_the_builder_held_back_is_refused_without_its_share() -> None:
    """A key the builder held back fails the bar, as it did on the net, with no share known."""
    share = builders_share([], [])
    assert share == 1.0, f"with no probe, the builder's share read {share}"
    assert bound(FROZEN, MIDDLE_MORE, share) < UNDER, (
        f"{UNDER} ms against {FROZEN} frozen held with no share, at {share}"
    )


def test_a_key_the_builder_held_back_holds_by_the_probes_share() -> None:
    """The same key holds once the probe says the builder held the guest back by half."""
    share = builders_share([WORK * HELD_BACK] * 10, [WORK] * 10)
    assert share == HELD_BACK, f"the probe's work twice as long gave a share of {share}"
    assert bound(FROZEN, MIDDLE_MORE, share) >= UNDER, (
        f"{UNDER} ms against {FROZEN} frozen failed at a share of {share}"
    )


def test_a_launcher_the_load_slows_is_refused_on_a_quiet_builder() -> None:
    """A launcher the guest's own load slows fails where the probe's work wasn't slowed."""
    share = builders_share([WORK] * 10, [WORK] * 10)
    slowed = SLOWER_MOST * QUIET_UNDER + MIDDLE_MORE
    assert share == 1.0, f"the probe's work as long under load gave a share of {share}"
    assert bound(QUIET_FROZEN, MIDDLE_MORE, share) >= QUIET_UNDER, (
        f"a quiet builder's {QUIET_UNDER} ms against {QUIET_FROZEN} failed"
    )
    assert bound(QUIET_FROZEN, MIDDLE_MORE, share) < slowed, (
        f"a launcher slowed to {slowed} ms against {QUIET_FROZEN} frozen held"
    )


def test_the_share_is_the_mean_of_the_work_not_its_middle() -> None:
    """A builder holding back one piece of work in three stretches the share, by the mean.

    A piece is a couple of milliseconds, shorter than the builder's turns, so most pieces go
    untouched and the middle one reads as no share at all.
    """
    share = builders_share([WORK, WORK, WORK * 4], [WORK] * 3)
    assert share == pytest.approx(2.0), f"a piece in three held four times gave {share}"


def test_the_share_is_never_under_one_nor_over_the_processors() -> None:
    """The load can't speed the builder up, nor split it past the VM's processors."""
    faster = builders_share([WORK / 2] * 3, [WORK] * 3)
    split = builders_share([WORK * 10] * 3, [WORK] * 3)
    assert faster == 1.0, f"the probe's work quicker under load gave a share of {faster}"
    assert split == PROCESSORS, f"the probe's work ten times as long gave a share of {split}"


def test_the_probes_last_line_is_left_until_its_whole() -> None:
    """A line the probe is still writing is left for the next read, and the rest are given."""
    taken, used = probed("2000\n2100\n21")
    assert taken == [2000.0, 2100.0], f"the probe's lines read as {taken}"
    assert used == len("2000\n2100\n"), f"the probe's whole lines took {used} characters"
