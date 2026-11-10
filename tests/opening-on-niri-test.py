"""An opening draws home from what the launcher kept, then fills it in where it stands, on niri.

modules/quickshell/checks.nix runs this through pytest inside tests/nested-niri.sh, with the
shell launcher_on_niri.py starts once a run. Through a storm of windows changing, every core
busy, an opening waited 25 to 48 ms on the shell waking for it before it could draw (2026-11-10),
so a closed launcher keeps home, and an opening draws that at once and wakes the rest after its
first frame. Kept, home never shows a reading as now that's as old as the opening that read it:
each row says how it stands as being read, "\u2026", as Room Light does. Filled in, the rows land
where the kept ones stood and the cursor stays on its row, so nothing jumps or flashes in
(Y2YBTS, 2026-11-10). Every test starts with the launcher closed, and leaves it so.
"""

from __future__ import annotations

import json
from typing import TYPE_CHECKING, cast

import launcher_on_niri
import pytest

if TYPE_CHECKING:
    from collections.abc import Iterator

    from launcher_on_niri import Json, Shell

# What a row being read says at its right (launcher/PlaceRow.js's kept).
READING = "\u2026"


@pytest.fixture(scope="module")
def shell() -> Iterator[Shell]:
    """Give the shell, started once a run, opened and closed once so it has home to keep."""
    started = launcher_on_niri.started()
    started.as_account(started.ipc("toggle"))
    started.launcher_ready()
    started.close_launcher()
    yield started
    started.stop()


@pytest.fixture(autouse=True)
def launcher_closed_after(shell: Shell) -> Iterator[None]:
    """Close the launcher after each test, so one failing with it open fails alone."""
    yield
    shell.launcher_left_closed()


def openings(shell: Shell) -> list[Json]:
    """Give the last openings the launcher timed, each from its ask to its first frame."""
    return cast("list[Json]", json.loads(shell.as_account(shell.ipc("openings"))))


def opened_from_kept(shell: Shell) -> Json:
    """Open the launcher as its chord does, and give that opening as it was timed and filled."""
    timed = len(openings(shell))
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    each: list[Json] = []

    def filled(_: Json) -> bool:
        each[:] = openings(shell)
        return len(each) > timed and "filled" in each[-1]

    shell.launcher_until(filled)
    opening = each[-1]
    assert opening["kept"], f"the opening wasn't drawn from what was kept: {opening}"
    return opening


def test_home_kept_says_each_reading_is_being_read_and_fills_in_where_it_stood(
    shell: Shell,
) -> None:
    """Kept home says each reading is being read, and filled in, its rows stand where they did.

    Left with a word typed and its rows found, the launcher keeps home all the same.
    """
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    shell.type_in("comp")
    shell.close_launcher()
    opening = opened_from_kept(shell)
    shown, filled = opening["shown"], opening["filled"]
    assert shell.showing()["typed"] == "", "the opening kept what was typed before"
    assert shown["keys"] == filled["keys"], f"filled in, the rows moved: {shown} to {filled}"
    assert shown["at"] == filled["at"] == 0, f"filled in, the cursor moved: {shown} to {filled}"
    assert all(side in {"", READING} for side in shown["sides"]), (
        f"kept home showed an old reading as now: {shown}"
    )
    assert READING in shown["sides"], f"no kept row said it was being read: {shown}"
    assert READING not in filled["sides"], f"filled in, a row still said it was read: {filled}"
    assert opening["rows"] == 0, f"the kept opening made rows before its frame: {opening}"


def test_keys_typed_as_it_opens_land_and_are_found(shell: Shell) -> None:
    """Keys typed the moment an opening holds them all land, and what they spell is found.

    A key that comes before the kept card's first frame is drawn fills it in first (filled),
    so it acts on home as it stands now. Keys typed before niri gives the card the keys reach
    the window that had them, as they would on any opening, so they're typed once it holds them.
    """
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_until(lambda shown: shown["keys"])
    shell.send_chars("comp")
    # What's found lands a frame after the word it was found for, so each is waited on.
    shell.launcher_until(lambda shown: shown["open"] and shown["typed"] == "comp")
    shell.launcher_until(lambda shown: shown["typed"] == "comp" and bool(shown["rows"]))
