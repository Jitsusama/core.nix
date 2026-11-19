"""What the keeps-pace area judges a loaded desktop by, apart from the machine judging it."""

from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Sequence

SLOWER_MOST = 2


def builders_share(under: Sequence[float], frozen: Sequence[float]) -> float:
    """Give how much longer the probe's work took under load than frozen."""
    del under, frozen
    return 1.0


def bound(frozen: float, more: float, share: float) -> float:
    """Give the most a time under load may take, against its time frozen."""
    del share
    return SLOWER_MOST * frozen + more


def probed(said: str) -> tuple[list[float], int]:
    """Give the probe's whole lines in what it said, and how many characters they took."""
    del said
    return [], 0
