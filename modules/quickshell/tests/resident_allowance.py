"""How much higher the shell's heap may hold resident a round on, for pages the kernel took.

The niri check of the shell staying light lets the second round's resident be higher by what the
kernel may have taken of it before the third (shell-stays-light-on-niri-test.py says why). It
lives apart from that test, which runs only on a nested niri, so its cases run off any VM
(resident-allowance-test.py), and it reads nothing of the machine, being given what was read.
"""

from __future__ import annotations

from typing import TYPE_CHECKING, NamedTuple

if TYPE_CHECKING:
    from collections.abc import Iterable

# A MB, in bytes, as each closing's line counts what's held.
MB = 2**20


class Allowance(NamedTuple):
    """What the kernel may have taken of the shell's heap, in MB, and the bounds it's the least of.

    Machine is what it took anywhere, shell the most any closing held but hadn't resident, and
    allowed the lesser.
    """

    machine: float
    shell: int
    allowed: float


def allowance(
    *, started: int, taken: int, page_size: int, closings: Iterable[tuple[int, int]]
) -> Allowance:
    """Give how much the kernel may have taken of the heap, from what was read of it.

    Started and taken are the pages the kernel had taken anywhere before the shell started and by
    the end of the round weighed, page_size the bytes in a page, and closings the MB each of the
    round's closings said the heap held and held resident, resident -1 where it was unknown. No
    more was taken than the machine took, nor than the shell held but hadn't resident at any one
    closing, since a page taken of its heap stays held: the machine's count alone would let a
    busy builder pass any climb. An unknown resident says nothing of what was taken, so it's
    skipped rather than read as none resident; a closing holding less than it has resident, as
    its chunks' headers can make it, says none was taken; and a round with no closing known
    allows nothing, so a round with nothing to weigh fails rather than passing.
    """
    machine = (taken - started) * page_size / MB
    shell = max(
        (max(held - resident, 0) for held, resident in closings if resident >= 0), default=0
    )
    return Allowance(machine=machine, shell=shell, allowed=min(machine, float(shell)))
