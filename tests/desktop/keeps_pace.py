"""How the keeps-pace area counts what the builder took from its VM, apart from the machine.

A launcher under load is timed against its time with the load frozen, in the same machine. The
guest's weights decide what it waits for inside the machine, and cpu.pressure counts that wait.
They can't decide what the builder running the VM gives it: under load all four of the VM's
processors are busy and frozen about one is, and a builder short of its own splits what it has
between them. The guest's kernel counts that time as stolen, time a processor of its own was
ready to run and the builder ran something else, and charges it to no task. So each subtest
says, from /proc/stat, how much of the guest's busy time was stolen in each turn, and the share
that gives: how much more of its busy time the guest ran frozen than under load.

The net failed keys from 16 to 19 November only with the frozen key at 24 to 69 ms, a busy
builder's, where a quiet one's is 15 to 18, while the compositor and the shell waited for a
processor within what they may. So a run that fails on the net says what was stolen. Steal is
what the builder kept from the guest's own processors, so little stolen under load clears a
builder short of processors for those, but not one keeping QEMU's main loop or virtiofsd
waiting, which draw and serve the shell's frames while the guest counts itself idle. Much
stolen may be the builder's, or QEMU's own threads taking its processors as the shell works
harder, so that's read against the builder's own load at the time. No bar is stretched by the share:
held to two of the builder's processors on 19 November, in an earlier held subtest whose load
froze and ran by turns as the timed subtests' does, the guest's frozen turn lost time too and
the share read 1.27, too little for the four to six times the failing keys took. The held
subtest that stayed counts with the load running alone, so it can't measure that again.

The builders are cloud VMs, whose own host's taking of their processors they don't count as
stolen, so the guest counts that time as its own and steal can't show it. On the net from 16 to
20 November the frozen key took 15 to 31 ms, and each ms more frozen gave about 7 more under
load, where the bar allows 2: 9ca37a94 failed keys at 122 ms against 30 on 20 November and
passed its retry at 31 against 16, its launcher the same. So each subtest also times a fixed
loop in the guest each time the load turns and as it finishes, said beside the ticks, to say
how fast the builder ran the guest in each. On a builder on 20 November it took 96 to 114 ms
under load against 96 to 99 frozen in most turns, so the load barely slows it and it reads the
builder's own speed. At b508ac2e the keys subtest's frozen loop took a middle 145 ms and its
keys 102 against a bar of 98, where its frozen key took 24, as at 9c79f541, whose keys took 42
under load on a builder whose loop took 103. So the frozen key can't see a slow builder and the
loop can, and the keys bar's share of the frozen time is stretched by how many times slower
than a normal builder the keys subtest's frozen loops ran (Gauge.slowed, stretched_bar). Only
its loops run with the shell caught up and idle: the storm's one frozen loop runs through its
own storm, and the first opening's beside the shell's start, so theirs read the shell as well
as the builder, and those bars aren't stretched. No other subtest has failed on a slow builder.
"""

from __future__ import annotations

# The VM's processors, all busy under load and about one frozen, so the most the builder
# splitting what it has between them can stretch one; more would be the builder's own load
# changing between the turns, which the load's can't be told from.
PROCESSORS = 4
# PF_NO_SETAFFINITY, the flag on a thread whose processors the kernel lets no one set.
UNHOLDABLE = 0x04000000
# The rounds of the bash loop timed as each turn ends: about 200 ms of a busy laptop's on 20
# November, short beside a turn of keys.
LOOPED = 50_000
# The slowest a normal builder runs the frozen loop, in ms: its middle read 95 to 103 in the
# keys subtest of each of the net's 8 runs from e31a9832 to 4c4b837e on 20 November, whose keys
# held at 31 to 42 ms under load, and 145 at b508ac2e, whose keys took 102 against a bar of 98.
# It's a time of the guest's own bash, so a bash whose loop runs slower would stretch the keys
# bar on every builder: each run says its stretch, which reads 1.00 on a normal builder.
BUILDER_LOOP = 105
# The most a slow builder may stretch the keys bar, a little past b508ac2e's 1.38. A builder
# slower than that stretches it no further, so with b508ac2e's frozen key of 24 ms, keys six
# times as slow under load still fail on any builder.
SLOWEST_BUILDER = 1.5


def builders_share(under: tuple[int, int], frozen: tuple[int, int]) -> float | None:
    """Give how much more the builder ran the guest frozen than under load, from 1 to PROCESSORS.

    Each turn is its stolen ticks and its busy ones, stolen included, over all the guest's
    processors, and the guest ran the part of its busy time not stolen. A builder stealing
    from both turns alike gives 1, as one stealing from neither does. A turn with no busy ticks,
    as when a subtest fails before its first freeze, has nothing to weigh, so gives none: a 1
    would read as nothing stolen, on the run it may explain.
    """
    (stolen_under, busy_under), (stolen_frozen, busy_frozen) = under, frozen
    if not busy_under or not busy_frozen:
        return None
    ran_under = 1 - stolen_under / busy_under
    ran_frozen = 1 - stolen_frozen / busy_frozen
    # Compared first, so turns stolen alike, wholly so too, give 1 rather than the cap.
    if ran_frozen <= ran_under:
        return 1.0
    if ran_under * PROCESSORS <= ran_frozen:
        return float(PROCESSORS)
    return ran_frozen / ran_under


def ticks(stat: str) -> tuple[int, int]:
    """Give the stolen ticks on /proc/stat's first line, and the busy ticks with them.

    Its fields are user, nice, system, idle, iowait, irq, softirq, steal, then the time spent
    running guests of its own, which user and nice already hold, so those aren't added again.
    """
    user, nice, system, _idle, _iowait, irq, softirq, steal = (
        int(field) for field in stat.split()[1:9]
    )
    return steal, user + nice + system + irq + softirq + steal


def looped(said: str) -> float:
    """Give the loop's time in ms from the guest's clock as it began and ended, in seconds."""
    began, ended = (float(stamp) for stamp in said.split())
    return (ended - began) * 1000


def every_tick(stat: str) -> int:
    """Give every tick on /proc/stat's first line, idle and iowait with the busy.

    It's the guest's own clock, so its busy ticks weigh against it whatever a read of them took.
    """
    return sum(int(field) for field in stat.split()[1:9])


def holdable(stat: str) -> bool:
    """Say from a thread's /proc stat line whether the kernel lets it be held to processors.

    Its flags are the seventh field after its name, which may hold spaces and parentheses, so
    the fields are read past the last parenthesis.
    """
    flags = int(stat.rsplit(")", 1)[1].split()[6])
    return not flags & UNHOLDABLE


def stretched_bar(frozen: float, slower: float, more: float, slowed: float) -> float:
    """Give the most a middle under load may take: `slower` times its frozen and `more`.

    Only the share of the frozen time is stretched `slowed` times for a slow builder, since
    that's all b508ac2e needed (a bar of 116 against its 102), and stretching `more` too would
    let a slower launcher through on a slow builder.
    """
    return slowed * slower * frozen + more


def lower_middle(times: list[float]) -> float | None:
    """Give the middle of these times, the lower of two, or none for no time."""
    ranked = sorted(times)
    return ranked[(len(ranked) - 1) // 2] if ranked else None


class Gauge:
    """The fixed loop's times in ms, under load (True) and frozen (False), in the order run."""

    def __init__(self) -> None:
        """Start with no loop timed."""
        self.times: dict[bool, list[float]] = {True: [], False: []}

    def add(self, *, turn: bool, took: float) -> None:
        """Keep a loop's time with the kind of turn it ended."""
        self.times[turn].append(took)

    def slowed(self) -> float:
        """Give how many times slower than a normal builder the frozen loops ran, 1 at least.

        It's their middle, the lower of two, over BUILDER_LOOP, held to SLOWEST_BUILDER. Only
        the frozen loops weigh, since under load the guest's own load may slow them, and none
        weighs nothing, as when a subtest fails before its first freeze. A builder no slower
        than BUILDER_LOOP tightens nothing and stretches nothing.
        """
        frozen = lower_middle(self.times[False])
        if frozen is None:
            return 1.0
        return min(max(frozen / BUILDER_LOOP, 1.0), SLOWEST_BUILDER)

    def said(self) -> str:
        """Say the middle loop of each kind of turn, the lower of two, and each loop's time.

        Each kind's loops keep the order they ran in, so a slow one can be matched to its turn.
        """

        def middle(times: list[float]) -> str:
            took = lower_middle(times)
            return "none" if took is None else f"{took:.0f} ms"

        def each(times: list[float]) -> list[int]:
            return [round(took) for took in times]

        under, frozen = self.times[True], self.times[False]
        return (
            f"a fixed loop of {LOOPED} rounds took a middle {middle(under)} under load against"
            f" {middle(frozen)} frozen, each in the order run under load {each(under)} and"
            f" frozen {each(frozen)}"
        )


class Ledger:
    """The guest's stolen and busy ticks in each turn, under load (True) and frozen (False)."""

    def __init__(self, stat: str) -> None:
        """Start counting from the kernel's line as it stands."""
        self.last = ticks(stat)
        self.turns: dict[bool, tuple[int, int]] = {True: (0, 0), False: (0, 0)}
        # Every tick on the last line, as last holds its stolen and busy ones.
        self.last_every = every_tick(stat)
        self.ticked: dict[bool, int] = {True: 0, False: 0}
        # Whether the load is between turns, from asking it to turn until it has.
        self.between = False

    def turning(self) -> None:
        """Keep what's counted in neither turn until it's turned, so a failed turn keeps none."""
        self.between = True

    def turned(self) -> None:
        """Keep what's counted with its turn again, the load having turned."""
        self.between = False

    def count(self, stat: str, *, turn: bool | None) -> None:
        """Count the ticks since the last line into this turn, or into none while it turned."""
        now = ticks(stat)
        every = every_tick(stat)
        if turn is not None and not self.between:
            stolen, busy = self.turns[turn]
            self.turns[turn] = (stolen + now[0] - self.last[0], busy + now[1] - self.last[1])
            self.ticked[turn] += every - self.last_every
        self.last = now
        self.last_every = every

    def said(self) -> str:
        """Say each turn's stolen and busy ticks, and the share they give, if any."""
        under, frozen = self.turns[True], self.turns[False]
        (stolen_under, busy_under), (stolen_frozen, busy_frozen) = under, frozen
        share = builders_share(under, frozen)
        shared = "so no share" if share is None else f"a share of {share:.2f}"
        return (
            f"the guest counted {stolen_under} of {busy_under} busy ticks stolen under load,"
            f" against {stolen_frozen} of {busy_frozen} frozen, {shared}"
        )
