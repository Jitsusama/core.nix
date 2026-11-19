"""What the keeps-pace area judges a loaded desktop by, apart from the machine judging it.

A launcher under load is held to its time with the load frozen, in the same machine, so a
builder that's busy slows both alike. It doesn't at the middle key: frozen, one of the VM's
processors is busy and the builder runs it as its own work allows, but under load all four are,
and the builder, which sees none of the guest's weights, splits its share between them. So the
launcher's processor runs a fraction of the time it did frozen, which no wait in the guest
counts. The guest's kernel counts it as stolen: time a processor of its own was ready to run and
the builder ran something else. It charges that to no task, so a task's own processor time never
shows it, and it's read from /proc/stat instead, where a wait in the guest's own queue isn't,
as cpu.pressure counts that.

On the net from 16 to 19 November every key that failed did so with the frozen key at 24 to 69
ms, a busy builder's, where a quiet one's is 15 to 18, and the key under load four to six times
that, while the compositor and the shell waited for a processor only 25 to 35 ms more than
frozen, within what they may. So the bar is the guest's own, twice the time frozen and a little
more, stretched by the builder's share.
"""

from __future__ import annotations

# Under load a time may take twice its time frozen, and a budget more, in the guest's own time.
SLOWER_MOST = 2
# The budget a key's middle may take more than twice its time frozen, in ms.
MIDDLE_MORE = 50
# The VM's processors, all busy under load and one frozen, so the most the builder splitting
# its share between them can stretch one; more would be the builder's own load changing
# between the turns, which the load's can't be told from.
PROCESSORS = 4


def builders_share(under: tuple[int, int], frozen: tuple[int, int]) -> float:
    """Give how much more the builder ran the guest frozen than under load, from 1 to PROCESSORS.

    Each turn is its stolen ticks and its busy ones, stolen included, over all the guest's
    processors, and the guest ran the part of its busy time not stolen. A builder stealing
    from both turns alike stretches nothing; one stealing from neither gives 1, and the bar is
    the guest's alone, as it is with no busy ticks in either.
    """
    (stolen_under, busy_under), (stolen_frozen, busy_frozen) = under, frozen
    if not busy_under or not busy_frozen:
        return 1.0
    ran_under = 1 - stolen_under / busy_under
    ran_frozen = 1 - stolen_frozen / busy_frozen
    if ran_under * PROCESSORS <= ran_frozen:
        return float(PROCESSORS)
    return max(1.0, ran_frozen / ran_under)


def bound(frozen: float, more: float, share: float) -> float:
    """Give the most a time under load may take, against its time frozen, by the share.

    The budget is stretched with the doubled time, since it's work the guest does in its own
    processor time too, which the builder holding the guest back stretches alike.
    """
    return share * (SLOWER_MOST * frozen + more)


def ticks(stat: str) -> tuple[int, int]:
    """Give the stolen ticks on /proc/stat's first line, and the busy ticks with them.

    Its fields are user, nice, system, idle, iowait, irq, softirq, steal, then the time spent
    running guests of its own, which user and nice already hold, so those aren't added again.
    """
    user, nice, system, _idle, _iowait, irq, softirq, steal = (
        int(field) for field in stat.split()[1:9]
    )
    return steal, user + nice + system + irq + softirq + steal
