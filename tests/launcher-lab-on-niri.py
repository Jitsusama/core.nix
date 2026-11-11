"""Time each chord's opening the one way every launcher change quotes, on niri.

A lab, not a check: modules/quickshell/checks.nix builds it as the `lab` of
the-launcher-opens-on-what-it-kept-on-niri, which no gate runs, and docs/testing.md's Timing the
Launcher says how its runs are taken a side at a time and read. Two labs gave chords 43 to 73 ms
and 173 to 225 ms presented (2026-11-11), for they opened different places, kept and whole, with
one shell or two sharing a builder, so neither said where chords stood. This one opens every
chord's place in turn, a round at a time, each from shut as its chord does, and prints each
opening's presented time from the ask with whether it was kept, so a run's places and rounds are
the same on both sides. Each place's line also splits its time at the shell's swap: what the
shell took to its frame, and what niri took after it to present it, since the VM's niri draws on
the CPU and once held about half its frames some 150 ms past the swap, which read as the
launcher's when only the whole was said (RTOIMQ, 2026-11-11). It fails only when an opening
never says it was presented.
"""

from __future__ import annotations

import json
import os
import statistics
from typing import TYPE_CHECKING, cast

import launcher_on_niri
import pytest

if TYPE_CHECKING:
    from collections.abc import Iterator

    from launcher_on_niri import Json, Shell

# Each chord's place as niri's binds ask for it (modules/niri/binds.nix), and home as the
# launcher's own chord toggles it.
PLACES = (
    "home",
    "on",
    "windows",
    "clipboard",
    "emoji",
    "spend",
    "shortcuts",
    "displays",
    "capture",
)

# How many times each place is opened in a run, unless the run says otherwise.
ROUNDS = 10

# How many windows are open as it runs, so Windows has rows to make.
WINDOWS = 4


@pytest.fixture(scope="module")
def shell() -> Iterator[Shell]:
    """Give the shell once it has kept home by itself, started once a run and never opened."""
    with launcher_on_niri.running() as started:
        started.launcher_until(lambda shown: shown.get("kept"))
        yield started


def openings(shell: Shell) -> list[Json]:
    """Give the last openings the launcher timed, each from its ask to its first frame."""
    return cast("list[Json]", json.loads(shell.as_account(shell.ipc("openings"))))


def latest(timed: list[Json]) -> int:
    """Give the number of the last opening timed, as only the last 20 are kept."""
    return cast("int", timed[-1].get("nth", len(timed))) if timed else 0


def opened(shell: Shell, place: str) -> Json:
    """Open a place from shut as its chord asks, and give that opening once niri presented it.

    It's asked once the shut launcher has kept home, as Joel's openings come seconds apart.
    """
    shell.launcher_until(lambda shown: not shown["open"] and shown.get("kept"))
    before = latest(openings(shell))
    words = ("toggle",) if place == "home" else ("collection", place)
    shell.as_account(shell.ipc(*words))
    each: list[Json] = []

    def presented(_: Json) -> bool:
        each[:] = openings(shell)
        return latest(each) > before and "presented" in each[-1]

    shell.launcher_until(presented)
    return each[-1]


def p90(spans: list[int]) -> int:
    """Give the span nine in ten openings came within."""
    ordered = sorted(spans)
    return ordered[min(len(ordered) - 1, round(0.9 * (len(ordered) - 1)))]


def said_of(place: str, presented: list[int], took: list[int]) -> str:
    """Say a place's openings: presented, and that split at the shell's swap of its frame."""
    niri = [whole - shell for whole, shell in zip(presented, took, strict=True)]
    return (
        f"lab-place {place}: {len(presented)} presented a median "
        f"{round(statistics.median(presented))} ms, p90 {p90(presented)}; the shell's frame a "
        f"median {round(statistics.median(took))} ms, niri's after it a median "
        f"{round(statistics.median(niri))} ms, p90 {p90(niri)}"
    )


def test_each_chord_opening_is_timed(shell: Shell, capsys: pytest.CaptureFixture[str]) -> None:
    """Open each chord's place from shut, round after round, and print each one's times.

    They're printed past pytest's capture, so the build's log holds them when it passes.
    """
    places = os.environ.get("LAB_PLACES", " ".join(PLACES)).split()
    rounds = int(os.environ.get("LAB_ROUNDS", str(ROUNDS)))
    windows = [shell.opened(f"Lab {nth}", app="foot") for nth in range(WINDOWS)]
    # Opened once untimed first: a fresh niri blurs behind its first surface only as that
    # shows, which held the first opening of every run 368 to 902 ms from its ask though it was
    # drawn in 11, against 38 and 42 with the blur off (H6EDLC, 2026-11-11). Joel's niri has
    # blurred since he logged in, so his openings never pay it.
    opened(shell, "home")
    shell.close_launcher()
    timed: dict[str, list[int]] = {place: [] for place in places}
    # From the ask to the shell's swap of the frame at the card's size, which `presented` runs
    # on from to niri's presented frame.
    took: dict[str, list[int]] = {place: [] for place in places}
    for round_ in range(rounds):
        # Each round starts one place further on, so no place always follows the same one.
        for place in places[round_ % len(places) :] + places[: round_ % len(places)]:
            opening = opened(shell, place)
            said = {
                "place": place,
                "round": round_,
                "presented": opening["presented"],
                "kept": bool(opening.get("kept")),
                "took": opening.get("took"),
                "niri": opening["presented"] - cast("int", opening["took"]),
                "rows": opening.get("rows"),
            }
            with capsys.disabled():
                print("lab-opening " + json.dumps(said), flush=True)
            if opening["presented"] < 0:
                pytest.fail(f"{place}'s opening closed unpresented: {opening}")
            timed[place].append(opening["presented"])
            took[place].append(cast("int", opening["took"]))
            shell.close_launcher()
            shell.launcher_until(lambda shown: not shown["open"])
    for window in windows:
        shell.close(window)
    for place, spans in timed.items():
        with capsys.disabled():
            print(said_of(place, spans, took[place]), flush=True)
