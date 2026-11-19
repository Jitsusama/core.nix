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
processor within what they may. So a run that fails on the net says what was stolen: little
stolen under load clears the builder, while much stolen may be the builder's or QEMU's own
threads, which draw the shell's frames and may steal from the guest as the shell works harder,
so that's read against the builder's own load at the time. No bar is stretched by the share:
held to two of the builder's processors, the guest's frozen turn lost time too and the share
read 1.27, too little for the four to six times the failing keys took.
"""

from __future__ import annotations

# The VM's processors, all busy under load and about one frozen, so the most the builder
# splitting what it has between them can stretch one; more would be the builder's own load
# changing between the turns, which the load's can't be told from.
PROCESSORS = 4


def builders_share(under: tuple[int, int], frozen: tuple[int, int]) -> float:
    """Give how much more the builder ran the guest frozen than under load, from 1 to PROCESSORS.

    Each turn is its stolen ticks and its busy ones, stolen included, over all the guest's
    processors, and the guest ran the part of its busy time not stolen. A builder stealing
    from both turns alike gives 1, as one stealing from neither does, and so does a turn with no
    busy ticks, which has nothing to weigh.
    """
    (stolen_under, busy_under), (stolen_frozen, busy_frozen) = under, frozen
    if not busy_under or not busy_frozen:
        return 1.0
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


class Ledger:
    """The guest's stolen and busy ticks in each turn, under load (True) and frozen (False)."""

    def __init__(self, stat: str) -> None:
        """Start counting from the kernel's line as it stands."""
        self.last = ticks(stat)
        self.turns: dict[bool, tuple[int, int]] = {True: (0, 0), False: (0, 0)}

    def count(self, stat: str, *, turn: bool | None) -> None:
        """Count the ticks since the last line into this turn, or into none while it turned."""
        now = ticks(stat)
        if turn is not None:
            stolen, busy = self.turns[turn]
            self.turns[turn] = (stolen + now[0] - self.last[0], busy + now[1] - self.last[1])
        self.last = now

    def said(self) -> str:
        """Say each turn's stolen and busy ticks, and the share they give."""
        under, frozen = self.turns[True], self.turns[False]
        (stolen_under, busy_under), (stolen_frozen, busy_frozen) = under, frozen
        share = builders_share(under, frozen)
        return (
            f"the guest counted {stolen_under} of {busy_under} busy ticks stolen under load,"
            f" against {stolen_frozen} of {busy_frozen} frozen, a share of {share:.2f}"
        )
