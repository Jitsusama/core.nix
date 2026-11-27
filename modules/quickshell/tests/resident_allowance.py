"""How much higher the shell's heap may hold resident a round on, for pages the kernel took.

The niri check of the shell staying light lets the resident of the round before the last be
higher by what the kernel may have taken of it before the last (shell-stays-light-on-niri-test.py
says why). It lives apart from that test, which runs only on a nested niri, so its cases run off
any VM (resident-allowance-test.py), and it reads nothing of the machine, being given what was
read.
"""

from __future__ import annotations

from typing import TYPE_CHECKING, NamedTuple

if TYPE_CHECKING:
    from collections.abc import Sequence

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
    *,
    started: int,
    taken: Sequence[int],
    page_size: int,
    rounds: Sequence[Sequence[tuple[int, int]]],
) -> Allowance:
    """Give how much the kernel may have taken of the heap by the round before the last.

    Started is the pages the kernel had taken anywhere before the shell started, taken the pages
    it had by the end of each round, page_size the bytes in a page, and rounds the MB each
    round's closings said the heap held and held resident, resident -1 where it was unknown.
    Only the round before the last is weighed, since the last is held to it and a take in the
    last only lowers it; it's chosen here rather than by the caller so its cases hold the choice.
    No more was taken than the machine took, a part of a MB counting as that part, nor than the
    shell held but hadn't resident at any one closing, since a page taken of its heap stays held:
    the machine's count alone would let a busy builder pass any climb. An unknown resident says
    nothing of what was taken, so it's skipped rather than read as none resident; a closing
    holding less than it has resident, as its chunks' headers can make it, says none was taken;
    and a round with no closing known allows nothing, so a round with nothing to weigh fails
    rather than passing. The niri test's own asserts already fail a round with no closings or
    an unknown resident, so the cases of those two pin what this gives, not that test's verdict.
    """
    machine = (taken[-2] - started) * page_size / MB
    shell = max(
        (max(held - resident, 0) for held, resident in rounds[-2] if resident >= 0), default=0
    )
    return Allowance(machine=machine, shell=shell, allowed=min(machine, float(shell)))
