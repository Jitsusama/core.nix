"""What the keeps-pace area judges a loaded desktop by, apart from the machine judging it.

A launcher under load is held to its time with the load frozen, in the same machine, so a
builder that's busy slows both alike. It doesn't at the middle key: frozen, one of the VM's
processors is busy and the builder runs it as its own work allows, but under load all four are,
and the builder, which sees none of the guest's weights, splits its share between them. So the
launcher's processor runs a fraction of the time it did frozen, which no wait in the guest
counts. These builders run the VM nested and give it no steal time, and the guest's kernel
charges a processor's time off the builder to whatever task it was running. So the same work
takes a probe more of its own processor time as the builder holds the guest back
(pace-probe.py), and a wait in the guest's own queue isn't counted, as cpu.pressure counts that.

On the net from 16 to 19 November every key that failed did so with the frozen key at 24 to 69
ms, a busy builder's, where a quiet one's is 15 to 18, and the key under load four to six times
that, while the compositor and the shell waited for a processor only 25 to 35 ms more than
frozen, within what they may. So the bar is the guest's own, twice the time frozen and a little
more, stretched by the builder's share.
"""

from __future__ import annotations

from statistics import fmean
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Sequence

# Under load a time may take twice its time frozen, and a budget more, in the guest's own time.
SLOWER_MOST = 2
# The VM's processors, all busy under load and one frozen, so the most the builder splitting
# its share between them can stretch one; more would be the builder's own load changing
# between the turns, which the load's can't be told from.
PROCESSORS = 4


def builders_share(under: Sequence[float], frozen: Sequence[float]) -> float:
    """Give how much longer the probe's work took under load than frozen, from 1 to PROCESSORS.

    By the mean of each, not the middle: a piece of work is a couple of milliseconds, shorter
    than the builder's turns, so most pieces go untouched and the middle one reads as no share
    at all, while the mean is the share of the time the probe's processor ran. With no probe in
    either, as on a builder that gives steal time and so stretches none of it, it's 1, and the
    bar is the guest's alone.
    """
    if not under or not frozen:
        return 1.0
    return min(float(PROCESSORS), max(1.0, fmean(under) / fmean(frozen)))


def bound(frozen: float, more: float, share: float) -> float:
    """Give the most a time under load may take, against its time frozen, by the share."""
    return share * (SLOWER_MOST * frozen + more)


def probed(said: str) -> tuple[list[float], int]:
    """Give the probe's whole lines in what it said, and how many characters they took.

    A line the probe is still writing is left for the next read.
    """
    whole = said[: said.rfind("\n") + 1]
    return [float(line) for line in whole.splitlines() if line.strip()], len(whole)
