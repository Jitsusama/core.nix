"""An opening draws home from what the launcher kept, then fills it in where it stands, on niri.

modules/quickshell/checks.nix runs this through pytest inside tests/nested-niri.sh, with the
shell launcher_on_niri.py starts once a run. Through a storm of windows changing, every core
busy, an opening waited 25 to 48 ms on the shell waking for it before it could draw (2026-11-10),
so a closed launcher keeps home, and an opening draws that at once and wakes the rest after its
first frame. Kept, home is drawn as it stood, and the fill replaces in place only what changed:
a mark shown for a frame or two and then put back is a flash (VKCF3Z, 2026-11-10). Filled in,
the rows land where the kept ones stood and the cursor stays on its row, so nothing jumps or
flashes in (Y2YBTS, 2026-11-10). Home is kept a few seconds after the shell starts too, so its
first opening is drawn the same way. Every test starts with the launcher closed, and leaves it
so.
"""

from __future__ import annotations

import json
from typing import TYPE_CHECKING, cast

import launcher_on_niri
import pytest

if TYPE_CHECKING:
    from collections.abc import Iterator

    from launcher_on_niri import Json, Shell

# What a row being read says at its right (launcher/PlaceRow.js's unfilled).
READING = "\u2026"


@pytest.fixture(scope="module")
def shell() -> Iterator[Shell]:
    """Give the shell, started once a run and never opened before the first test opens it."""
    started = launcher_on_niri.started()
    yield started
    started.stop()


@pytest.fixture(autouse=True)
def home_kept_after(shell: Shell) -> Iterator[None]:
    """Close the launcher after each test, and wait until it has kept home for the next.

    Home is put back a turn after the card has faded out, so the next test's opening would
    otherwise race it, and one failing with the launcher open fails alone.
    """
    yield
    shell.launcher_left_closed()
    shell.launcher_until(lambda shown: shown.get("kept"))


def openings(shell: Shell) -> list[Json]:
    """Give the last openings the launcher timed, each from its ask to its first frame."""
    return cast("list[Json]", json.loads(shell.as_account(shell.ipc("openings"))))


def latest(timed: list[Json]) -> int:
    """Give the number of the last opening timed, as only the last 20 are kept."""
    return cast("int", timed[-1].get("nth", len(timed))) if timed else 0


def opened_from_kept(shell: Shell) -> Json:
    """Open the launcher as its chord does, and give that opening as it was timed and filled."""
    shell.launcher_until(lambda shown: shown.get("kept"))
    timed = latest(openings(shell))
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    each: list[Json] = []

    def filled(_: Json) -> bool:
        each[:] = openings(shell)
        return latest(each) > timed and "filled" in each[-1]

    shell.launcher_until(filled)
    opening = each[-1]
    assert opening.get("kept"), f"the opening wasn't drawn from what was kept: {opening}"
    return opening


def settled(shell: Shell) -> Json:
    """Wait until the launcher's rows have held still, and give the rows it shows then."""
    assert shell.held_still(), "the launcher never held still after it opened"
    return cast("Json", shell.showing()["seen"])


def test_the_first_opening_after_a_start_is_drawn_from_home_kept_as_it_started(
    shell: Shell,
) -> None:
    """The shell keeps home a few seconds after it starts, so its first opening draws from it.

    Made on the chord instead, the first opening after each switch made every row home reads
    before its frame: 489 on Joel's at 14:05, which took 3,521 ms (2026-11-10). Filled in, a
    source nothing changed since the start makes none of its rows again.
    """
    shell.launcher_until(lambda shown: shown.get("kept"))
    assert openings(shell) == [], "the launcher was opened before the test opened it"
    opening = opened_from_kept(shell)
    assert opening.get("first"), f"the opening wasn't the first since the shell started: {opening}"
    assert opening.get("madeBy", {}).get("applications", 0) == 0, (
        f"filling in remade applications' rows, which nothing changed since the start: {opening}"
    )


def test_home_kept_is_drawn_as_it_stood(shell: Shell) -> None:
    """Kept home is drawn as it was left, each row saying what it said, with nothing made first."""
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    left = settled(shell)
    shell.close_launcher()
    opening = opened_from_kept(shell)
    shown = opening["shown"]
    assert shown["keys"] == left["keys"], f"kept, the rows moved: {left} to {shown}"
    assert shown["sides"] == left["sides"], f"kept, a row's side changed: {left} to {shown}"
    assert opening["rows"] == 0, f"the kept opening made rows before its frame: {opening}"


def test_home_kept_with_a_word_typed_fills_in_where_it_stood(shell: Shell) -> None:
    """Left with a word typed, the launcher keeps home, and its rows stand once it has settled."""
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    shell.type_in("comp")
    shell.close_launcher()
    opening = opened_from_kept(shell)
    assert shell.showing()["typed"] == "", "the opening kept what was typed before"
    shown, after = opening["shown"], settled(shell)
    assert shown["keys"] == after["keys"], f"filled in, the rows moved: {shown} to {after}"


def test_the_cursor_stays_on_its_row_as_home_fills_in(shell: Shell) -> None:
    """Moved down as the card opens, the cursor stays on that row's key once home has settled."""
    shell.launcher_until(lambda shown: shown.get("kept"))
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_until(lambda shown: shown["keys"])
    shell.send_key("down")
    moved: list[Json] = []

    def down(shown: Json) -> bool:
        moved[:] = [shown["seen"]]
        return cast("int", shown["seen"]["at"]) == 1

    shell.launcher_until(down)
    on = moved[0]["keys"][1]
    after = settled(shell)
    assert after["keys"][after["at"]] == on, f"filled in, the cursor left {on}: {after}"


def test_no_frame_flashes_as_home_fills_in(shell: Shell) -> None:
    """From its first frame until it settles, no row is emptied and filled, nor a side put back.

    The launcher reports what it shows after each frame that changed it, so each frame between
    the kept one and the settled one is read.
    """
    since = shell.mark()
    opened_from_kept(shell)
    settled(shell)
    reports, _ = shell.said_since(since)
    frames = [
        cast("Json", json.loads(said[len("showing ") :]))["seen"]
        for said in reports
        if said.startswith("showing ")
    ]
    frames = [seen for seen in frames if seen["keys"]]
    assert frames, "the launcher reported no frame with rows as it opened"
    for key in {key for seen in frames for key in seen["keys"]}:
        present = [key in seen["keys"] for seen in frames]
        first, last = present.index(True), len(present) - present[::-1].index(True)
        assert all(present[first:last]), f"{key} was emptied and filled again: {frames}"
        sides = [seen["sides"][seen["keys"].index(key)] for seen in frames if key in seen["keys"]]
        said = [at for at, side in enumerate(sides) if side not in {"", READING}]
        assert not said or READING not in sides[said[0] :], (
            f"{key} showed a value and then put it back as being read: {sides}"
        )


def test_keys_typed_as_it_opens_land_and_are_found(shell: Shell) -> None:
    """Keys typed the moment an opening holds them all land, and what they spell is found.

    A key that comes before the kept card's first frame is drawn fills it in first (filled),
    so it acts on home as it stands now. Keys typed before niri gives the card the keys reach
    the window that had them, as they would on any opening, so they're typed once it holds them.
    """
    shell.launcher_until(lambda shown: shown.get("kept"))
    timed = latest(openings(shell))
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_until(lambda shown: shown["keys"])
    shell.send_chars("agents")
    # What's found lands a frame after the word it was found for, and the sources woken by the
    # opening may find again as they answer, so the row is waited on as it settles.
    shell.launcher_until(
        lambda shown: (
            shown["typed"] == "agents" and (shown["selected"] or {}).get("name") == "Agents"
        )
    )
    each = openings(shell)
    assert latest(each) > timed, f"the opening was never timed: {each}"
    opening = each[-1]
    assert opening.get("kept"), f"the opening wasn't drawn from what was kept: {opening}"
    assert opening.get("filledBy") in {"frame", "key"}, f"nothing filled it in: {opening}"
