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
loop in the guest at the end of each turn, said beside the ticks and judged by nothing yet, to
say how fast the builder ran the guest in each.
"""

from __future__ import annotations

# The VM's processors, all busy under load and about one frozen, so the most the builder
# splitting what it has between them can stretch one; more would be the builder's own load
# changing between the turns, which the load's can't be told from.
PROCESSORS = 4
# PF_NO_SETAFFINITY, the flag on a thread whose processors the kernel lets no one set.
UNHOLDABLE = 0x04000000
# The turns of the bash loop timed at the end of each turn: about 200 ms of a busy laptop's on 20
# November, short beside a turn of keys.
LOOPED = 50_000


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


class Gauge:
    """The fixed loop's times in ms, one a turn, under load (True) and frozen (False)."""

    def __init__(self) -> None:
        """Start with no loop timed."""
        self.times: dict[bool, list[float]] = {True: [], False: []}

    def add(self, *, turn: bool, took: float) -> None:
        """Keep a loop's time with the kind of turn it ended."""
        self.times[turn].append(took)

    def said(self) -> str:
        """Say the middle loop of each kind of turn, the lower of two, and each loop's time."""

        def middle(times: list[float]) -> str:
            ranked = sorted(times)
            return f"{ranked[(len(ranked) - 1) // 2]:.0f} ms" if ranked else "none"

        def each(times: list[float]) -> list[int]:
            return [round(took) for took in sorted(times)]

        under, frozen = self.times[True], self.times[False]
        return (
            f"a fixed loop of {LOOPED} turns took a middle {middle(under)} under load against"
            f" {middle(frozen)} frozen, each under load {each(under)} and frozen {each(frozen)}"
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
