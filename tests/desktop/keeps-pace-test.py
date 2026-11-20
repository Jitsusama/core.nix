"""How the keeps-pace area counts what the builder took from its VM, with no machine to load.

The area times a launcher under load against the load frozen, and each subtest says how much of
the guest's busy time the kernel counted stolen in each turn (keeps_pace.py), so a run that fails
on the net shows whether the builder took the time or the shell spent it.
"""

from __future__ import annotations

import math

from keeps_pace import (
    BUILDER_LOOP,
    LOOPED,
    PROCESSORS,
    SLOWEST_BUILDER,
    Gauge,
    Ledger,
    builders_share,
    stretched_bar,
    every_tick,
    holdable,
    looped,
    ticks,
)

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
# Every tick it counts, idle and iowait with the busy, but not a guest's, already in user and nice.
EVERY = 100 + 200 + 300 + 400 + 500 + 600 + 700 + 800
# A thread's stat line, its name holding a space and a parenthesis, with PF_NO_SETAFFINITY set
# among its flags, the seventh field after the name, and with the flag next to it set instead.
REFUSED = "812 (qemu (io) 1) S 1 812 812 0 -1 69238848 0 0"
ALLOWED = "812 (qemu (io) 1) S 1 812 812 0 -1 35684416 0 0"
# Every tick the ledger's lines below count under load, in two counts, and frozen, idle and
# iowait with the busy.
TICKED_UNDER = 60 + 30 + 10 + 20 + 30
TICKED_FROZEN = 5 + 40
# The guest's clock as the loop began and ended, in seconds, and the time between in ms.
LOOP_STAMPS = "1795163661.100000 1795163661.325500"
LOOP_TOOK = 225.5
# The keys subtest's frozen loops, in ms, as the slowest normal builder ran them at 9c79f541 on
# 20 November, whose middle was 103, and as the slow one did at b508ac2e, whose middle was 145.
NORMAL_LOOPS = (145.0, 174.0, 96.0, 95.0, 201.0, 103.0, 100.0)
SLOW_LOOPS = (156.0, 122.0, 145.0, 109.0, 240.0, 171.0, 127.0)
SLOWED = 145 / BUILDER_LOOP
# The keys bar as the VM test sets it, SLOWER_MOST times the frozen key and MIDDLE_MORE ms, and
# b508ac2e's keys: frozen 24 ms, and 102 under load against an unstretched bar of 98.
KEYS_SLOWER = 2
KEYS_MORE = 50
B508_FROZEN = 24.0
B508_KEYS = 102.0
B508_BAR = 98.0
# At the most a builder stretches it, the keys bar for b508ac2e's frozen key, which stretching
# KEYS_MORE as well would raise to 147, and keys six times as slow as that frozen key.
CAPPED_BAR = 122.0
SIX_TIMES = 6


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
    assert share is not None, "the held ticks gave no share"
    assert math.isclose(share, HELD_SHARE, abs_tol=0.005), f"the held ticks gave {share}"


def test_the_share_is_never_under_one_nor_over_the_processors() -> None:
    """The load can't speed the builder up, nor split it past the VM's processors."""
    faster = builders_share((0, BUSY), (BUSY // 2, BUSY))
    split = builders_share((BUSY * 9 // 10, BUSY), (0, BUSY))
    starved = builders_share((BUSY, BUSY), (0, BUSY))
    assert faster == 1.0, f"less stolen under load than frozen gave a share of {faster}"
    assert split == PROCESSORS, f"nine in ten ticks stolen under load gave a share of {split}"
    assert starved == PROCESSORS, f"every tick stolen under load alone gave a share of {starved}"


def test_a_turn_with_no_busy_ticks_gives_no_share() -> None:
    """With no busy ticks in a turn there's nothing to weigh, where a 1 would read as nothing."""
    unloaded = builders_share((0, 0), (0, BUSY))
    unfrozen = builders_share((0, BUSY), (0, 0))
    assert unloaded is None, f"no busy ticks under load gave a share of {unloaded}"
    assert unfrozen is None, f"no busy ticks frozen gave a share of {unfrozen}"


def test_every_tick_stolen_in_both_turns_gives_a_share_of_one() -> None:
    """A builder that ran the guest no more frozen than under load took nothing from the load."""
    share = builders_share((BUSY, BUSY), (BUSY, BUSY))
    assert share == 1.0, f"every tick stolen in both turns gave a share of {share}"


def test_stolen_and_busy_ticks_are_read_from_the_kernels_line() -> None:
    """The stolen ticks are steal's, and the busy ones leave out idle, iowait and guests'."""
    stolen, busy = ticks(STAT)
    assert stolen == STOLEN, f"{STAT!r} read as {stolen} stolen ticks"
    assert busy == BUSIED, f"{STAT!r} read as {busy} busy ticks"


def test_every_tick_is_read_from_the_kernels_line() -> None:
    """Every tick is the eight fields' up to steal, leaving out the guests' times they hold."""
    every = every_tick(STAT)
    assert every == EVERY, f"{STAT!r} read as {every} ticks"


def test_each_turn_keeps_every_tick_it_counted_and_a_turning_none() -> None:
    """A turn's every tick weighs its busy ones by the guest's own clock, whatever a read took."""
    ledger = Ledger("cpu  0 0 0 0 0 0 0 0 0 0")
    ledger.count("cpu  60 0 0 30 10 0 0 0 0 0", turn=True)
    ledger.count("cpu  90 0 0 80 10 0 0 0 0 0", turn=None)
    ledger.count("cpu  95 0 0 120 10 0 0 0 0 0", turn=False)
    ledger.turning()
    ledger.count("cpu  100 0 0 200 10 0 0 0 0 0", turn=True)
    ledger.turned()
    ledger.count("cpu  120 0 0 230 10 0 0 0 0 0", turn=True)
    under, frozen = ledger.ticked[True], ledger.ticked[False]
    assert under == TICKED_UNDER, f"under load the ledger kept {under} ticks"
    assert frozen == TICKED_FROZEN, f"frozen the ledger kept {frozen} ticks"


def test_each_count_goes_to_its_own_turn_and_a_turning_to_neither() -> None:
    """Ticks counted under load and frozen land in their turns, and those between, in neither."""
    ledger = Ledger(stat(0, 0))
    ledger.count(stat(10, 100), turn=True)
    ledger.count(stat(40, 400), turn=None)
    ledger.count(stat(45, 450), turn=False)
    ledger.count(stat(65, 650), turn=True)
    assert ledger.turns[True] == (30, 300), f"under load the ledger kept {ledger.turns[True]}"
    assert ledger.turns[False] == (5, 50), f"frozen the ledger kept {ledger.turns[False]}"


def test_a_turn_that_never_finished_keeps_what_follows_in_neither() -> None:
    """A freeze that fails partway leaves the ledger between turns, each count after in neither."""
    ledger = Ledger(stat(0, 0))
    ledger.count(stat(10, 100), turn=True)
    ledger.turning()
    ledger.count(stat(40, 400), turn=True)
    assert ledger.between, "a turn begun and never finished left the ledger out of it"
    assert ledger.turns[True] == (10, 100), f"under load the ledger kept {ledger.turns[True]}"
    ledger.turned()
    ledger.count(stat(45, 450), turn=False)
    assert not ledger.between, "a finished turn left the ledger between turns"
    assert ledger.turns[False] == (5, 50), f"frozen the ledger kept {ledger.turns[False]}"


def test_a_thread_the_kernel_refuses_to_hold_is_told_by_its_flags() -> None:
    """PF_NO_SETAFFINITY in its flags, read past a name holding parentheses, marks it unholdable."""
    assert not holdable(REFUSED), f"{REFUSED!r} read as a thread the kernel lets be held"
    assert holdable(ALLOWED), f"{ALLOWED!r} read as a thread the kernel refuses to hold"


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


def test_a_loops_time_is_read_from_the_guests_two_stamps() -> None:
    """The loop's time is its end less its start, in ms, by the guest's one clock."""
    took = looped(LOOP_STAMPS)
    assert math.isclose(took, LOOP_TOOK, abs_tol=0.001), f"{LOOP_STAMPS!r} read as {took} ms"


def test_the_gauge_says_each_turns_loops_and_their_middles() -> None:
    """What a subtest says keeps each kind's loops in the order they ran, and the middle one.

    In order, so a slow loop can be matched to the turn of keys it ended.
    """
    gauge = Gauge()
    for took in (300.4, 250.0, 279.6):
        gauge.add(turn=True, took=took)
    for took in (90.2, 80.0):
        gauge.add(turn=False, took=took)
    said = gauge.said()
    assert said == (
        f"a fixed loop of {LOOPED} rounds took a middle 280 ms under load against 80 ms frozen,"
        " each in the order run under load [300, 250, 280] and frozen [90, 80]"
    ), f"the gauge said {said!r}"


def test_the_gauge_says_none_for_a_turn_it_never_timed() -> None:
    """A kind of turn a failure never reached has no loop, which no time is said for."""
    gauge = Gauge()
    gauge.add(turn=True, took=300.0)
    said = gauge.said()
    assert said == (
        f"a fixed loop of {LOOPED} rounds took a middle 300 ms under load against none frozen,"
        " each in the order run under load [300] and frozen []"
    ), f"the gauge said {said!r}"


def gauged(*, under: tuple[float, ...] = (), frozen: tuple[float, ...] = ()) -> Gauge:
    """Give a gauge that timed these loops under load and these frozen."""
    gauge = Gauge()
    for took in under:
        gauge.add(turn=True, took=took)
    for took in frozen:
        gauge.add(turn=False, took=took)
    return gauge


def test_a_normal_builder_leaves_each_bar_as_it_was() -> None:
    """A builder running the frozen loop no slower than BUILDER_LOOP stretches nothing."""
    slowed = gauged(frozen=NORMAL_LOOPS).slowed()
    assert slowed == 1.0, f"a normal builder stretched each bar {slowed} times"


def test_a_slow_builder_stretches_each_bar_by_its_middle_frozen_loop() -> None:
    """The slow builder b508ac2e failed on stretches each bar by its middle frozen loop."""
    slowed = gauged(frozen=SLOW_LOOPS).slowed()
    assert math.isclose(slowed, SLOWED), f"b508ac2e's builder stretched each bar {slowed} times"


def test_the_middle_of_two_frozen_loops_is_the_lower() -> None:
    """One slow loop of two stretches nothing, as the gauge's said middle is the lower."""
    slowed = gauged(frozen=(BUILDER_LOOP, BUILDER_LOOP * 2)).slowed()
    assert slowed == 1.0, f"one slow loop of two stretched each bar {slowed} times"


def test_a_builder_past_the_slowest_stretches_a_bar_no_further() -> None:
    """A builder slower than SLOWEST_BUILDER times a normal one stretches no bar past that."""
    slowed = gauged(frozen=(BUILDER_LOOP * SLOWEST_BUILDER * 2,)).slowed()
    assert slowed == SLOWEST_BUILDER, f"a builder three times slow stretched each bar {slowed}"


def test_loops_under_load_stretch_no_bar() -> None:
    """Only the frozen loops weigh, since under load the guest's own load may slow them."""
    slowed = gauged(under=(BUILDER_LOOP * 3,) * len(NORMAL_LOOPS), frozen=NORMAL_LOOPS).slowed()
    assert slowed == 1.0, f"slow loops under load stretched each bar {slowed} times"


def test_a_subtest_with_no_frozen_loop_stretches_no_bar() -> None:
    """A failure before the first freeze leaves nothing to weigh, so the bar stays as written."""
    slowed = gauged(under=(BUILDER_LOOP * 3,)).slowed()
    assert slowed == 1.0, f"no frozen loop stretched each bar {slowed} times"


def test_b508ac2es_keys_fail_the_bar_unstretched() -> None:
    """On a normal builder the keys b508ac2e took on its slow one still fail."""
    bar = stretched_bar(B508_FROZEN, KEYS_SLOWER, KEYS_MORE, 1.0)
    assert bar == B508_BAR, f"b508ac2e's keys bar read {bar}, unstretched"
    assert B508_KEYS > bar, f"b508ac2e's keys passed {bar} unstretched"


def test_b508ac2es_keys_pass_the_bar_its_slow_builder_stretched() -> None:
    """The keys b508ac2e failed on its slow builder pass the bar its frozen loops stretched."""
    slowed = gauged(frozen=SLOW_LOOPS).slowed()
    bar = stretched_bar(B508_FROZEN, KEYS_SLOWER, KEYS_MORE, slowed)
    assert B508_KEYS <= bar, f"b508ac2e's keys failed {bar}, stretched {slowed} times"


def test_only_the_frozen_share_of_a_bar_is_stretched() -> None:
    """The fixed ms beside the frozen share stay as written, so a slow builder adds no more."""
    bar = stretched_bar(B508_FROZEN, KEYS_SLOWER, KEYS_MORE, SLOWEST_BUILDER)
    assert math.isclose(bar, CAPPED_BAR), f"the keys bar read {bar} at the most stretch"


def test_keys_six_times_as_slow_fail_on_the_slowest_builder() -> None:
    """At b508ac2e's frozen key, keys six times as slow fail however slow the builder."""
    slowed = gauged(frozen=(BUILDER_LOOP * SLOWEST_BUILDER * 2,)).slowed()
    bar = stretched_bar(B508_FROZEN, KEYS_SLOWER, KEYS_MORE, slowed)
    assert SIX_TIMES * B508_FROZEN > bar, f"keys six times as slow passed {bar}"


def test_the_ledger_says_no_share_for_a_turn_it_never_counted() -> None:
    """A turn a failure never reached has nothing to weigh, which a share of 1 would clear."""
    ledger = Ledger(stat(0, 0))
    ledger.count(stat(500, 1000), turn=True)
    said = ledger.said()
    assert said == (
        "the guest counted 500 of 1000 busy ticks stolen under load, against 0 of 0 frozen,"
        " so no share"
    ), f"the ledger said {said!r}"
    frozen = Ledger(stat(0, 0))
    frozen.count(stat(500, 1000), turn=False)
    said = frozen.said()
    assert said == (
        "the guest counted 0 of 0 busy ticks stolen under load, against 500 of 1000 frozen,"
        " so no share"
    ), f"the ledger said {said!r}"
