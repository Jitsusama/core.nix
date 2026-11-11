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

import contextlib
import json
import os
import signal
import socket
import time
from datetime import timedelta
from pathlib import Path
from typing import TYPE_CHECKING, cast

import launcher_on_niri
import pytest

if TYPE_CHECKING:
    from collections.abc import Callable, Iterator

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

# How long a kept opening's sources are woken for in a frame (Launcher.qml's wakeBudget), less
# the millisecond its times are rounded by.
WAKE_BUDGET_MS = 7

# How long a window opening is given to reach the closed shell, which hears niri's events
# within a few milliseconds of each.
SETTLING_S = 2

# How long a closing's spots are given to be freshened: a minute after they're made
# (Launcher.qml's spotsFreshened), and room for a loaded builder.
FRESHENING = timedelta(seconds=75)

# The sources that make rows as any opening begins, by a handler of their own on the launcher's
# open rather than as they're wanted: Capture reads what was taken (2026-11-11). Now's retime
# and Agents' own rows did too, until #3111 and #3112 put them off until they're read.
MADE_AS_ANY_OPENS = frozenset({"capture"})

# How long the closed launcher's frame count must hold to count as having drawn what it kept:
# keeping home says so before the pixel left on the screen draws it, which a loaded builder
# shows a frame late, and well inside the half second before a closing makes its spots.
KEPT_DRAWN = timedelta(milliseconds=200)


@pytest.fixture(scope="module")
def shell() -> Iterator[Shell]:
    """Give the shell once it has kept home by itself, started once a run and never opened."""
    started = launcher_on_niri.started()
    started.launcher_until(lambda shown: shown.get("kept"))
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


def test_the_first_opening_after_a_start_is_drawn_from_home_kept_as_it_started(
    shell: Shell,
) -> None:
    """The shell keeps home a few seconds after it starts, so its first opening draws from it.

    Made on the chord instead, the first opening after each switch made every row home reads
    before its frame: 489 on Joel's at 14:05, which took 3,521 ms (2026-11-10). Filled in, a
    source nothing changed since the start makes none of its rows again.
    """
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


def test_home_kept_fits_a_screen_that_shrank_while_it_was_shut(shell: Shell) -> None:
    """Kept at one scale and opened at a larger, the card fits inside the screen it opens on.

    The kept card was drawn at the height it was kept at, so kept at 1 and opened at 1.5, its
    foot ran 30 px off the bottom (vm-desktop-says-what-happens, 2026-11-10). How many rows fit
    reads the card's height but for its rows, and read from the card's height as it followed
    them, Qt warned of a loop for rows (the net, 2026-11-10, at bf0d237ec).
    """
    before = shell.showing()["card"]
    since = shell.mark()
    shell.as_account("niri msg output winit scale 2")
    try:
        shell.launcher_until(lambda shown: shown["card"]["screen"] < before["screen"])
        shell.launcher_until(lambda shown: shown.get("kept"))
        opened_from_kept(shell)
        card = shell.showing()["card"]
        assert card["foot"] < card["screen"], f"kept at 1, opened at 2, it's off the bottom: {card}"
        assert card["chrome"] == card["drawn"], f"its rows were fitted to a card it isn't: {card}"
        warned = shell.log.read_text(errors="replace")[since:].splitlines()
        looped = [line for line in warned if "Binding loop" in line]
        assert not looped, f"refitting, it looped: {looped}"
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


def test_a_chord_into_a_place_makes_only_its_rows(shell: Shell) -> None:
    """Opened into a place by its chord, no other source makes rows as it opens.

    The launcher was opened before the place's step was held, so for that instant it stood at
    home and every source home names made its rows, put away at once: each Super+Tab reopening
    made 35, of Capture, Today, Time and Agents (3TXW9E's measure, 2026-11-10). Those that
    make rows as any opening begins, whatever it opens into, are their owners' to put off
    (MADE_AS_ANY_OPENS), so they're left out here.
    """
    before = json.loads(shell.as_account(shell.ipc("made")))["rows"]
    shell.as_account(shell.ipc("collection", "windows"))
    shell.launcher_ready()
    after = json.loads(shell.as_account(shell.ipc("made")))["rows"]
    made = {
        name: rows - before.get(name, 0)
        for name, rows in after.items()
        if name != "windows" and name not in MADE_AS_ANY_OPENS and rows > before.get(name, 0)
    }
    assert not made, f"opening into Windows, other sources made rows: {made}"


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


def test_a_kept_opening_wakes_its_sources_a_frame_apart(shell: Shell) -> None:
    """A kept opening's sources wake a few a frame once it's filled in, home's first row's first.

    Woken at once as it filled in, they held Joel's shell 195 to 220 ms past its first frame,
    each key in that time waiting on them (2026-11-10). A slice ends once it's run past its
    budget, so every one but the last has, and each source wakes once.
    """
    nth = opened_from_kept(shell)["nth"]
    woke: list[Json] = []

    def slept(_: Json) -> bool:
        woke[:] = [each for each in openings(shell) if each.get("nth") == nth]
        return bool(woke) and "slices" in woke[0]

    shell.launcher_until(slept)
    slices = cast("list[Json]", woke[0]["slices"])
    names = [name for each in slices for name in each["woke"]]
    assert names[:1] == ["on"], f"home's first row's source didn't wake first: {slices}"
    assert len(names) == len(set(names)), f"a source woke twice: {slices}"
    assert all(each["took"] >= WAKE_BUDGET_MS for each in slices[:-1]), (
        f"a slice ended before its budget: {slices}"
    )


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


def test_openings_make_only_the_notices_rows_that_came(shell: Shell) -> None:
    """Opened again and searched, the launcher makes only the notice rows of notices come since.

    Joel's first opening after a restart made 942 notices' rows in its settling (2026-11-10).
    That was the first making since the start: each row is kept by its notice and its sender's
    window and program as niri and the desktop entries say (launcher/PastNotices.qml), so an
    opening makes none again, and a notice arriving makes its own row alone.
    """

    def notices_rows() -> int:
        return int(json.loads(shell.as_account(shell.ipc("made")))["rows"].get("notices", 0))

    def searched(word: str) -> int:
        """Open home, search the notices for the word, close, and give the rows that made."""
        shell.launcher_until(lambda shown: shown.get("kept"))
        before = notices_rows()
        shell.as_account(shell.ipc("toggle"))
        shell.launcher_ready()
        settled(shell)
        shell.type_in(word, selecting=word)
        shell.close_launcher()
        return notices_rows() - before

    for summary in ["Dunlin", "Sanderling"]:
        with shell.told(f"shows card {summary}"):
            shell.as_account(f"notify-send --expire-time=1000 '{summary}' 'a notice kept'")
    searched("dunlin")
    again = searched("sanderling")
    with shell.told("shows card Turnstone"):
        shell.as_account("notify-send --expire-time=1000 'Turnstone' 'a notice come since'")
    came = searched("turnstone")
    assert again == 0, f"opened again with no notice come, it made {again} notices' rows"
    assert came == 1, f"one notice come, an opening made {came} notices' rows"


# The sources whose rows are still made at every opening, by their owners' leave until they
# keep them: Agents' own rows are bound to whether they're read, which each opening flips, until
# the agents daemon makes them ready (Fri 13 Nov); Capture's Recent is read afresh at each.
MADE_AT_EACH_OPENING = {"agents", "capture"}


def test_for_you_makes_only_the_rows_whose_source_changed(shell: Shell) -> None:
    """Opened into For You again, and told a notice while open, it makes only what changed.

    Opening into For You again with nothing come made Time's 7 rows, Today's 10 and Location's
    (2026-11-11): each opening placed the time anew, and Location's row was made anew as Now
    was wanted again. Each is kept now while it says the same, so a kept opening makes none of
    them, and a notice heard while it's open makes the one row of the source it came to:
    Desktop's, which holds the notices unread (Y2YBTS, 2026-11-11).
    """

    def made() -> dict[str, int]:
        return cast("dict[str, int]", json.loads(shell.as_account(shell.ipc("made")))["rows"])

    def since(before: dict[str, int]) -> dict[str, int]:
        now = made()
        return {
            name: now[name] - before.get(name, 0)
            for name in now
            if now[name] > before.get(name, 0) and name not in MADE_AT_EACH_OPENING
        }

    def in_for_you() -> None:
        shell.launcher_until(lambda shown: shown.get("kept"))
        shell.as_account(shell.ipc("collection", "on"))
        shell.launcher_ready()
        shell.launcher_until(lambda shown: "For You" in shown["rows"])
        shell.down_to("For You")
        shell.step_in()
        shell.launcher_until(lambda shown: shown["held"][-1:] == ["For You"])
        settled(shell)

    def within_a_minute(act: Callable[[], None]) -> dict[str, int]:
        """Give the rows the act made, done again once if a minute turned, which remakes some."""
        for _ in range(2):
            minute, before = int(time.time() // 60), made()
            act()
            made_by_it = since(before)
            if int(time.time() // 60) == minute:
                break
        return made_by_it

    def notified() -> None:
        with shell.told("shows card Whimbrel"):
            shell.as_account("notify-send --expire-time=1000 'Whimbrel' 'a notice heard open'")
        settled(shell)

    def reopened() -> None:
        shell.close_launcher()
        in_for_you()

    in_for_you()
    again = within_a_minute(reopened)
    heard = within_a_minute(notified)
    shell.close_launcher()
    assert again == {}, f"opened into For You again with nothing come, it made {again}"
    assert heard == {"on": 1}, f"a notice heard in For You made {heard}, not Desktop's row"


# How many times the shell is restarted back to back, as five switches in half an hour did
# on 9 November, after which the launcher stopped taking keys.
RESTARTS = 5


def test_the_launcher_takes_keys_after_each_of_five_restarts(shell: Shell) -> None:
    """Restarted five times running, as switches restart it, the launcher opens and takes keys.

    A switch restarts the shell while niri, its daemons and Joel's windows go on, and on 9
    November the launcher stopped taking keys after five switches in half an hour, so switches
    were held 30 minutes apart (update-to-main's follow.rs). Each restart here is followed by an
    opening from the chord's socket, as Joel's chord opens it, and a word typed and found.
    """
    for restart in range(1, RESTARTS + 1):
        shell.restart()
        with socket.socket(socket.AF_UNIX) as said:
            said.connect(str(Path(os.environ["XDG_RUNTIME_DIR"]) / "launcher.sock"))
            said.sendall(f"toggle @{time.time():.6f}\n".encode())
        shell.launcher_ready()
        shell.send_chars("agents")
        try:
            shell.launcher_until(
                lambda shown: (
                    shown["typed"] == "agents" and (shown["selected"] or {}).get("name") == "Agents"
                )
            )
        except AssertionError as failed:
            message = f"after restart {restart} of {RESTARTS}, typing found nothing: {failed}"
            raise AssertionError(message) from failed
        shell.close_launcher()
