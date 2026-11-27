"""How much higher the shell's heap may hold resident for pages taken (resident_allowance.py).

The niri check of the shell staying light is the only other place the allowance is worked out,
on a nested niri, so a wrong bound, page size, sign or round there would pass unseen; here it
runs off any VM.
"""

from __future__ import annotations

import pytest
from resident_allowance import Allowance, allowance

# A page as most machines have it, and as some ARM ones do.
PAGE = 4096
LARGE_PAGE = 16384
# The pages that make a MB of each, a MB being 2**20 bytes.
PAGES_IN_A_MB = 256
LARGE_PAGES_IN_A_MB = 64
# Pages the kernel had taken anywhere before the shell started.
STARTED = 1000
# The MB held and held resident at a round's closings, the most held unresident 10 MB.
CLOSINGS = [(50, 40), (52, 45), (48, 46)]
HELD_UNRESIDENT = 10
# What the machine took, in MB, less and more than that; less by a part of a MB, so a MB
# rounded either way reads otherwise.
LESS = 3.5
MORE = 20
# Half a MB, which a MB rounded either way reads as none or one.
HALF = 0.5
# A closing whose resident was unknown, holding far more than any other.
UNKNOWN = (300, -1)
# A closing with none of what it held resident, holding more than the others hold unresident.
NONE_RESIDENT = (30, 0)
# The first and last rounds about the one weighed: in the first the machine took nothing and in
# the last 100 MB more, and each held more unresident than any closing of the one weighed.
FIRST = [(80, 0)]
LAST = [(90, 0)]
LAST_TAKEN = 100


def taken(megabytes: float) -> int:
    """Give the pages of PAGE bytes taken by a round's end, these MB more than at the start."""
    return STARTED + int(megabytes * PAGES_IN_A_MB)


def weighed(closings: list[tuple[int, int]], pages: int, page_size: int = PAGE) -> Allowance:
    """Give the allowance of three rounds, the second of these closings and pages taken by it.

    The first and last rounds read otherwise than any round weighed here, so a case reading
    either fails.
    """
    return allowance(
        started=STARTED,
        taken=[STARTED, pages, pages + LAST_TAKEN * PAGES_IN_A_MB],
        page_size=page_size,
        rounds=[FIRST, closings, LAST],
    )


def test_the_round_before_the_last_is_weighed_rather_than_the_last_or_the_first() -> None:
    room = allowance(
        started=STARTED,
        taken=[STARTED, taken(LESS), taken(MORE)],
        page_size=PAGE,
        rounds=[FIRST, CLOSINGS, LAST],
    )
    wanted = Allowance(machine=LESS, shell=HELD_UNRESIDENT, allowed=LESS)
    assert room == wanted, f"of three rounds, the second taking {LESS} MB allowed {room}"


def test_the_machine_taking_less_than_the_shell_held_unresident_allows_the_machines() -> None:
    room = weighed(CLOSINGS, taken(LESS))
    wanted = Allowance(machine=LESS, shell=HELD_UNRESIDENT, allowed=LESS)
    assert room == wanted, f"the machine taking {LESS} MB allowed {room}"


def test_the_shell_holding_less_unresident_than_the_machine_took_allows_the_shells() -> None:
    room = weighed(CLOSINGS, taken(MORE))
    wanted = Allowance(machine=MORE, shell=HELD_UNRESIDENT, allowed=HELD_UNRESIDENT)
    assert room == wanted, f"the machine taking {MORE} MB allowed {room}"


def test_an_unknown_resident_is_skipped_rather_than_counted_as_all_held() -> None:
    room = weighed([*CLOSINGS, UNKNOWN], taken(MORE))
    assert room.shell == HELD_UNRESIDENT, f"a closing of unknown resident allowed {room}"
    assert room.allowed == HELD_UNRESIDENT, f"a closing of unknown resident allowed {room}"


def test_a_closing_with_none_resident_counts_all_it_held() -> None:
    held, _ = NONE_RESIDENT
    room = weighed([*CLOSINGS, NONE_RESIDENT], taken(MORE))
    wanted = Allowance(machine=MORE, shell=held, allowed=MORE)
    assert room == wanted, f"a closing holding {held} MB, none resident, allowed {room}"


def test_a_closing_holding_less_than_it_has_resident_counts_none() -> None:
    room = weighed([(40, 42), (41, 43)], taken(MORE))
    wanted = Allowance(machine=MORE, shell=0, allowed=0)
    assert room == wanted, f"closings holding less than they had resident allowed {room}"


def test_a_round_with_no_closings_allows_nothing() -> None:
    room = weighed([], taken(MORE))
    wanted = Allowance(machine=MORE, shell=0, allowed=0)
    assert room == wanted, f"a round with no closings allowed {room}"


def test_half_a_mb_taken_counts_as_half() -> None:
    room = weighed(CLOSINGS, taken(HALF))
    assert room.machine == HALF, f"{taken(HALF) - STARTED} pages of {PAGE} bytes allowed {room}"


@pytest.mark.parametrize(
    ("page_size", "pages"),
    [(PAGE, PAGES_IN_A_MB), (LARGE_PAGE, LARGE_PAGES_IN_A_MB)],
    ids=["4096-byte pages", "16384-byte pages"],
)
def test_the_page_size_is_used_as_given_and_a_mb_is_two_to_the_twentieth_bytes(
    page_size: int, pages: int
) -> None:
    room = weighed(CLOSINGS, STARTED + pages, page_size)
    assert room.machine == 1, f"{pages} pages of {page_size} bytes took {room.machine} MB"
