"""How much higher the shell's heap may hold resident for pages taken (resident_allowance.py).

The niri check of the shell staying light is the only other place the allowance is worked out,
on a nested niri, so a wrong bound, page size or sign there would pass unseen; here it runs off
any VM.
"""

from __future__ import annotations

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
# What the machine took, in MB, less and more than that.
LESS = 3
MORE = 20
# A closing whose resident was unknown, holding far more than any other.
UNKNOWN = (300, -1)
# A closing with none of what it held resident, holding more than the others hold unresident.
NONE_RESIDENT = (30, 0)


def taken(megabytes: int) -> int:
    """Give the pages of PAGE bytes taken by the round's end, these MB more than at the start."""
    return STARTED + megabytes * PAGES_IN_A_MB


def test_the_machine_taking_less_than_the_shell_held_unresident_allows_the_machines() -> None:
    room = allowance(started=STARTED, taken=taken(LESS), page_size=PAGE, closings=CLOSINGS)
    wanted = Allowance(machine=LESS, shell=HELD_UNRESIDENT, allowed=LESS)
    assert room == wanted, f"the machine taking {LESS} MB allowed {room}"


def test_the_shell_holding_less_unresident_than_the_machine_took_allows_the_shells() -> None:
    room = allowance(started=STARTED, taken=taken(MORE), page_size=PAGE, closings=CLOSINGS)
    wanted = Allowance(machine=MORE, shell=HELD_UNRESIDENT, allowed=HELD_UNRESIDENT)
    assert room == wanted, f"the machine taking {MORE} MB allowed {room}"


def test_an_unknown_resident_is_skipped_rather_than_counted_as_all_held() -> None:
    room = allowance(
        started=STARTED, taken=taken(MORE), page_size=PAGE, closings=[*CLOSINGS, UNKNOWN]
    )
    assert room.shell == HELD_UNRESIDENT, f"a closing of unknown resident allowed {room}"
    assert room.allowed == HELD_UNRESIDENT, f"a closing of unknown resident allowed {room}"


def test_a_closing_with_none_resident_counts_all_it_held() -> None:
    held, _ = NONE_RESIDENT
    room = allowance(
        started=STARTED, taken=taken(MORE), page_size=PAGE, closings=[*CLOSINGS, NONE_RESIDENT]
    )
    wanted = Allowance(machine=MORE, shell=held, allowed=MORE)
    assert room == wanted, f"a closing holding {held} MB, none resident, allowed {room}"


def test_a_closing_holding_less_than_it_has_resident_counts_none() -> None:
    closings = [(40, 42), (41, 43)]
    room = allowance(started=STARTED, taken=taken(MORE), page_size=PAGE, closings=closings)
    wanted = Allowance(machine=MORE, shell=0, allowed=0)
    assert room == wanted, f"closings holding less than they had resident allowed {room}"


def test_a_round_with_no_closings_allows_nothing() -> None:
    room = allowance(started=STARTED, taken=taken(MORE), page_size=PAGE, closings=[])
    wanted = Allowance(machine=MORE, shell=0, allowed=0)
    assert room == wanted, f"a round with no closings allowed {room}"


def test_the_page_size_is_used_as_given_and_a_mb_is_two_to_the_twentieth_bytes() -> None:
    for page_size, pages in ((PAGE, PAGES_IN_A_MB), (LARGE_PAGE, LARGE_PAGES_IN_A_MB)):
        room = allowance(
            started=STARTED, taken=STARTED + pages, page_size=page_size, closings=CLOSINGS
        )
        assert room.machine == 1, f"{pages} pages of {page_size} bytes took {room.machine} MB"
