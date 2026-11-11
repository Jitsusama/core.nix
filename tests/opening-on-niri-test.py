"""An opening draws home from what the launcher kept, then fills it in where it stands, on niri.

modules/quickshell/checks.nix runs this through pytest inside tests/nested-niri.sh, with the
shell launcher_on_niri.py starts once a run. Through a storm of windows changing, every core
busy, an opening waited 25 to 48 ms on the shell waking for it before it could draw (2026-11-10),
so a closed launcher keeps home, and an opening draws that at once and wakes the rest after its
first frame. Kept, home is drawn as it stood, and the fill replaces in place only what changed:
a mark shown for a frame or two and then put back is a flash (VKCF3Z, 2026-11-10). Filled in,
the rows land where the kept ones stood and the cursor stays on its row, so nothing jumps or
flashes in (Y2YBTS, 2026-11-10). Every test starts with the launcher closed, and leaves it so.
"""

from __future__ import annotations

import contextlib
import json
import os
import signal
import time
from datetime import timedelta
from pathlib import Path
from typing import TYPE_CHECKING, cast

import launcher_on_niri
import pytest

if TYPE_CHECKING:
    from collections.abc import Iterator

    from launcher_on_niri import Json, Shell

# What a row being read says at its right (launcher/PlaceRow.js's unfilled).
READING = "\u2026"

# Past when a closing makes its places' spots, half a second after it (Launcher.qml's
# spotsLater).
SPOTS_DUE = timedelta(milliseconds=700)

# When they're made: a reopening asked only after it finds them made, rightly, and shows
# nothing of the race.
SPOTS_LATER = timedelta(milliseconds=500)

# How many closings and reopenings a case tries before a builder too busy to ask the reopening
# within SPOTS_LATER fails it.
REOPENINGS_TRIED = 3

# How long niri leaves a surface wholly off the screen before it answers a frame of its: once
# a second (niri.rs's FRAME_CALLBACK_THROTTLE), so an opening that waited on one is plain.
UNANSWERED_MS = 900

# How long a window opening is given to reach the closed shell, which hears niri's events
# within a few milliseconds of each.
SETTLING_S = 2

# How long a closing's spots are given to be freshened: a minute after they're made
# (Launcher.qml's spotsFreshened), and room for a loaded builder.
FRESHENING = timedelta(seconds=75)

# How long the closed launcher's frame count must hold to count as having drawn what it kept:
# keeping home says so before the pixel left on the screen draws it, which a loaded builder
# shows a frame late, and well inside the half second before a closing makes its spots.
KEPT_DRAWN = timedelta(milliseconds=200)


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
    return filled_after(shell, timed)


def filled_after(shell: Shell, timed: int) -> Json:
    """Give the opening after the one numbered, once it's ready and filled in, kept."""
    shell.launcher_ready()
    each: list[Json] = []

    def filled(_: Json) -> bool:
        each[:] = openings(shell)
        return latest(each) > timed and "filled" in each[-1]

    shell.launcher_until(filled)
    opening = each[-1]
    assert opening.get("kept"), f"the opening wasn't drawn from what was kept: {opening}"
    return opening


@contextlib.contextmanager
def niri_held() -> Iterator[None]:
    """Hold the nested niri stopped while what's done inside it runs, so it answers no frame."""
    held = [
        int(process.name)
        for process in Path("/proc").iterdir()
        if process.name.isdigit()
        and (process / "cmdline").read_bytes().split(b"\0")[1:2] == [b"-c"]
        and (process / "comm").read_text().strip() == "niri"
    ]
    assert len(held) == 1, f"there should be one niri to hold, not {held}"
    os.kill(held[0], signal.SIGSTOP)
    try:
        yield
    finally:
        os.kill(held[0], signal.SIGCONT)


def frames_once_kept(shell: Shell) -> int:
    """Wait until home is kept and drawn, and give how many frames the launcher drew by then."""
    shell.launcher_until(lambda shown: shown.get("kept"))
    drawn = shell.frames()
    quiet = time.monotonic()
    giving_up = quiet + 5
    while time.monotonic() - quiet < KEPT_DRAWN.total_seconds():
        assert time.monotonic() < giving_up, "closed, the launcher kept drawing"
        # Polled, since a frame says nothing as it swaps but the count it adds to.
        time.sleep(0.05)
        if (now := shell.frames()) != drawn:
            drawn, quiet = now, time.monotonic()
    return drawn


def settled(shell: Shell) -> Json:
    """Wait until the launcher's rows have held still, and give the rows it shows then."""
    assert shell.held_still(), "the launcher never held still after it opened"
    return cast("Json", shell.showing()["seen"])


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


def test_home_kept_fits_a_screen_that_shrank_while_it_was_shut(shell: Shell) -> None:
    """Kept at one scale and opened at a larger, the card fits inside the screen it opens on.

    The kept card was drawn at the height it was kept at, so kept at 1 and opened at 1.5, its
    foot ran 30 px off the bottom (vm-desktop-says-what-happens, 2026-11-10).
    """
    before = shell.showing()["card"]
    shell.as_account("niri msg output winit scale 2")
    try:
        shell.launcher_until(lambda shown: shown["card"]["screen"] < before["screen"])
        shell.launcher_until(lambda shown: shown.get("kept"))
        opened_from_kept(shell)
        card = shell.showing()["card"]
        assert card["foot"] < card["screen"], f"kept at 1, opened at 2, it's off the bottom: {card}"
    finally:
        shell.as_account("niri msg output winit scale 1")


def test_a_chord_into_a_place_finds_no_home_first(shell: Shell) -> None:
    """Opened into a place by its chord, the launcher searches the place alone, not home first.

    Home was found as every opening began and then put away for the place: Super+V's openings
    made home's 51 and 60 rows first (Joel's journal, 2026-11-10, 20:40). This account's home
    rows are kept unchanged, so the searches counted, which no refresh adds to, show it instead.
    """
    timed = latest(openings(shell))
    before = int(shell.as_account(shell.ipc("searched")).strip())
    shell.as_account(shell.ipc("collection", "on"))
    shell.launcher_ready()
    each: list[Json] = []

    def timed_after(_: Json) -> bool:
        each[:] = openings(shell)
        return latest(each) > timed

    shell.launcher_until(timed_after)
    searches = int(shell.as_account(shell.ipc("searched")).strip()) - before
    assert searches == 1, f"opening into On searched {searches} times: {each[-1]}"


def test_a_reopening_puts_off_the_spots_its_closing_left(shell: Shell) -> None:
    """Reopened as the closing's spots come due, the kept opening makes none before its frame.

    A closing makes its places' spots half a second after it, and one reopened then made
    Network's rows between its ask and its first frame (a gate of #2971, 2026-11-10). Niri is
    held from the closing until past that half second, so the reopening's first frame can't
    come before the spots are due, however quickly the shell turns. Rows follow only from a
    place changed while shut, so the spots being made at all is what fails. Each call starts a
    process, and on a busy builder the two took past the half second, so the spots were made
    before the reopening was asked (four gates, 2026-11-10): such a try shows nothing and is
    made again.
    """
    for _ in range(REOPENINGS_TRIED):
        first = opened_from_kept(shell)
        began = shell.mark()
        with niri_held():
            closed = time.monotonic()
            shell.as_account(shell.ipc("toggle"))
            # Home is kept a turn after the closing, before the next call can land.
            shell.as_account(shell.ipc("toggle"))
            asked = time.monotonic() - closed
            # Held until the spots are due, so they come due between the ask and its frame.
            time.sleep(max(0, closed + SPOTS_DUE.total_seconds() - time.monotonic()))
        opening = filled_after(shell, cast("int", first["nth"]))
        if asked < SPOTS_LATER.total_seconds():
            break
        shell.close_launcher()
    else:
        pytest.fail(f"no reopening was asked within {SPOTS_LATER} of its closing")
    said, _ = shell.said_since(began)
    spots = [one for one in said if one.startswith("spots made while closed ")]
    assert not spots, f"the closing's spots were made as the launcher reopened: {spots}"
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


def test_a_kept_opening_fills_in_on_its_first_frame(shell: Shell) -> None:
    """Kept at the card's size while closed, an opening moves it and fills on its first frame.

    Grown from a pixel, every opening waited for niri to answer its new size and for a second
    frame at it, 100 to 150 ms past its own work on Joel's laptop (2026-11-10). Closed, it's
    off the screen but for a pixel, and wholly off it, niri answered its frames only once a
    second, so an opening that waited on a frame drawn there took most of a second to its own.
    """
    opening = opened_from_kept(shell)
    assert opening["frames"] == 1, f"the kept opening waited on niri's size: {opening}"
    assert opening["took"] < UNANSWERED_MS, f"the opening waited on a frame unanswered: {opening}"


def test_the_closed_launcher_draws_nothing_as_windows_change(shell: Shell) -> None:
    """Closed, the launcher draws no frame as a window opens and closes.

    Kept at the card's size wholly off the screen, it drew a frame a second: niri answered each
    only after a second, and Qt took the surface for hidden until then and drew it again once
    shown, so a pixel of it stays on the screen.
    """
    drawn = frames_once_kept(shell)
    window = shell.opened("Godwit Notes")
    try:
        # The span measured, for the window's events to reach the shell, not a wait for an end.
        time.sleep(SETTLING_S)
    finally:
        shell.close(window)
    # The same span, for its closing.
    time.sleep(SETTLING_S)
    assert shell.frames() == drawn, f"closed, it drew {shell.frames() - drawn} frames"


def test_the_closed_launcher_draws_nothing_as_it_lets_its_closing_go(shell: Shell) -> None:
    """Closed inside a place, the launcher draws no frame as it lets go of what the closing held.

    A closing lets its steps go a quarter of a second after it (leaveSteps), makes its spots
    half a second after it, and their figures a minute after that, each while the card is off
    the screen but for a pixel, where nothing it draws is seen. Counted from once home is
    kept and drawn, since putting it back follows the closing at once.
    """
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    shell.type_in("programs", selecting="Programs")
    shell.step_in()
    since = shell.mark()
    shell.close_launcher()
    drawn = frames_once_kept(shell)
    shell.reported("spots made while closed ", lambda _: True, since)
    shell.reported("spots freshened while closed ", lambda _: True, since, FRESHENING)
    assert shell.frames() == drawn, f"closed, it drew {shell.frames() - drawn} frames"
