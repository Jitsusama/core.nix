"""An opening draws home from what the launcher kept, then fills it in where it stands, on niri.

modules/quickshell/checks/core.nix runs this through pytest inside tests/nested-niri.sh, with the
shell launcher_on_niri.py starts once a run. Through a storm of windows changing, every core
busy, an opening waited 25 to 48 ms on the shell waking for it before it could draw (2026-11-10),
so a closed launcher keeps home, and an opening draws that at once and wakes the rest after its
first frame. Kept, home is drawn as it stood, and the fill replaces in place only what changed:
a mark shown for a frame or two and then put back is a flash (VKCF3Z, 2026-11-10). Filled in,
the rows land where the kept ones stood and the cursor stays on its row, so nothing jumps or
flashes in (Y2YBTS, 2026-11-10). Home is kept a few seconds after the shell starts too, so its
first opening is drawn the same way. Every test starts with the launcher closed, and leaves it
so.

Long because every case here opens from what one shell kept, started once a run, through the
fixture that keeps home after each case and the helpers that read its openings, so a second
file would start a second shell and share them by import.
"""

from __future__ import annotations

import json
import os
import socket
import subprocess
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

# What the shell says as it makes a closing's spots while shut, or makes them again a minute
# after (Launcher.qml's spotsMadeClosed and spotsFreshened).
SPOTS_MADE_SHUT = ("spots made while closed ", "spots freshened while closed ")

# How many closings and reopenings a case tries before a builder too busy to ask the reopening
# within SPOTS_LATER fails it.
REOPENINGS_TRIED = 3

# How often the shell's log is read for its closing, between a closing and its reopening.
CLOSING_READ = 0.005

# What the shell says once when it can't write the open mark (Launcher.qml's markFailed).
MARK_UNWRITTEN = "launcher: couldn't write the open mark"

# The title a window takes while the launcher is shut, so its row is another.
RETITLED = "Gannet, retitled"

# How long niri leaves a surface wholly off the screen before it answers a frame of its: once
# a second (niri.rs's FRAME_CALLBACK_THROTTLE), so an opening that waited on one is plain.
UNANSWERED_MS = 900

# How long a kept opening's sources are woken for in a frame (Launcher.qml's wakeBudget), less
# the millisecond its times are rounded by.
WAKE_BUDGET_MS = 7

# How long a window opening is given to reach the closed shell, which hears niri's events
# within a few milliseconds of each.
SETTLING_S = 2

# How far the shell's minute may stand from this clock's, in seconds: its clock's timer can
# fire a little off the minute, and a loaded shell handles it late, so a minute this clock saw
# turn just before or after an act may be the one the shell turned during it. A For You case
# guarded by this clock alone failed so, on a minute turned around its act (FEH1HE,
# 2026-11-11, at 8c5ff8bf).
CLOCK_SLACK = 5

# How far into a minute an act timed against the clock may begin, in seconds, so the rest of
# the minute, less CLOCK_SLACK, holds the act: begun anywhere, both of a For You case's tries
# crossed a minute on a busy builder and it judged the second's (2026-11-12, at c015284d3).
MINUTE_BEGUN = 20

# How long a closing's spots are given to be freshened: a minute after they're made
# (Launcher.qml's spotsFreshened), and room for a loaded builder.
FRESHENING = timedelta(seconds=75)

# What fills a kept opening in (Launcher.qml's filled): its frame drawn where niri can't say it
# was presented, niri saying it was or not saying in time, or a key before any of them.
FILLERS = frozenset({"frame", "presented", "unpresented", "key"})

# How many windows a case opens, so Windows has rows whose keys are weighed as it opens.
WINDOWS_WEIGHED = 3

# How many kept openings a case times its fill against niri's presenting in.
PRESENTED_OPENINGS = 5

# How long the closed launcher's frame count must hold to count as having drawn what it kept:
# keeping home says so before the pixel left on the screen draws it, which a loaded builder
# shows a frame late, and well inside the half second before a closing makes its spots.
KEPT_DRAWN = timedelta(milliseconds=200)

# How many rows down a case moves the cursor before a letter, so the letter has a row to take
# it from.
ROWS_DOWN = 3

# What the shell says once it has kept home and every chord's place as it started
# (Launcher.qml's warmNext), and how long after a restart a case waits for it: the pass is due
# a second and a half after the shell's start and takes about two more in a loaded builder.
WARMED_SAID = "places kept as the shell started"
WARMED = timedelta(seconds=20)

# How long after a start every source here has first answered: what warming once waited out by
# beginning at 6 s. The restart case passed waiting 5 s past home kept (2026-11-12), but on a
# gate's loaded builder Networks, Traffic and Displays answered later still, so past it the
# case waits until no source has said a change for QUIET, for at most SETTLED.
ANSWERED = timedelta(seconds=6)
QUIET = timedelta(seconds=2)
SETTLED = timedelta(seconds=30)

# How long after his first chord into a restart Joel, seeing nothing open, presses it again.
PRESSED_AGAIN = timedelta(milliseconds=300)

# How long chords said into a restart are given to open the launcher once the new shell starts,
# and how long it's then watched staying open.
RESTARTED = timedelta(seconds=30)
STAYS = timedelta(seconds=1)


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


def told(line: str) -> None:
    """Say a line down the launcher's socket from this process, as its chord's launcher-say does.

    No process is started for it, so two said in a row are as far apart as the shell makes them.
    """
    with socket.socket(socket.AF_UNIX) as said:
        said.connect(str(Path(os.environ["XDG_RUNTIME_DIR"]) / "launcher.sock"))
        said.sendall(f"{line}\n".encode())


def heard_closing(shell: Shell, since: int, by: float) -> None:
    """Wait, reading the shell's log alone, until it says it's closing, or the clock reaches by."""
    while time.monotonic() < by:
        said, _ = shell.said_since(since)
        if "closing" in said:
            return
        time.sleep(CLOSING_READ)


def minute_begun() -> None:
    """Wait, if the clock is past MINUTE_BEGUN or within CLOCK_SLACK of its minute, for the next."""
    into = time.time() % 60
    if into < CLOCK_SLACK or into > MINUTE_BEGUN:
        time.sleep((CLOCK_SLACK - into) % 60)


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


def presented_after(shell: Shell, nth: int) -> Json:
    """Give the opening numbered once niri has said it presented it, or that it closed first."""
    each: list[Json] = []

    def presented(_: Json) -> bool:
        each[:] = [one for one in openings(shell) if one.get("nth") == nth]
        return bool(each) and "presented" in each[0]

    shell.launcher_until(presented)
    return each[0]


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
    # None where the place was kept as the shell started, and drawn from it.
    assert searches <= 1, f"opening into On searched {searches} times: {each[-1]}"


def test_a_chord_into_a_place_makes_only_its_rows(shell: Shell) -> None:
    """Opened into a place by its chord, no other source makes rows as it opens.

    The launcher was opened before the place's step was held, so for that instant it stood at
    home and every source home names made its rows, put away at once: each Super+Tab reopening
    made 35, of Capture, Today, Time and Agents (3TXW9E's measure, 2026-11-10). Now, Agents and
    Capture also made theirs as any opening began, by handlers of their own, until #3111, #3112
    and #3083 put each off until it's read or changed. The last case's closing makes its spots
    half a second after it, only while the launcher is shut, so they can land between reading
    the rows made and the ask, as Displays' 19 did in a gate of #3148 (RO9YDD, 2026-11-11):
    such a try counts nothing of the opening's, so it's closed, its own spots waited for, and
    made again.
    """
    made = made_by_others(shell, "windows")
    assert not made, f"opening into Windows, other sources made rows: {made}"


def test_a_chord_first_after_a_restart_makes_only_its_places_rows(shell: Shell) -> None:
    """The first chord after a restart, with home kept as the shell started, makes only its rows.

    Home kept as the shell started wasn't counted as kept for an opening into a place, so the
    first chord after each switch found home again whole on its way into the place, as no
    later one does.
    """
    restarted = time.monotonic()
    shell.restart()
    shell.launcher_until(lambda shown: shown.get("kept"))
    answered_since(shell, restarted)
    made = made_by_others(shell, "windows")
    assert not made, f"first into Windows after a restart, other sources made rows: {made}"


def test_a_chord_first_after_a_restart_is_drawn_from_its_place_kept_as_it_started(
    shell: Shell,
) -> None:
    """The first chord into a place after a restart draws it as the shell kept it, as it started.

    Only a place a chord had opened before was kept, so each place's first chord since a start
    was made whole: all 22 of Joel's on 2026-11-11, read by the shell's pid, took 70 to 413 ms
    to the frame, while every later chord drew from what was kept in 13 to 35.
    """
    shell.restart()
    shell.launcher_until(lambda shown: shown.get("kept"))
    timed = latest(openings(shell))
    shell.as_account(shell.ipc("collection", "on"))
    shell.launcher_ready()
    each: list[Json] = []

    def timed_after(_: Json) -> bool:
        each[:] = openings(shell)
        return latest(each) > timed

    shell.launcher_until(timed_after)
    opening = each[-1]
    assert opening.get("into") == "on", f"the opening didn't say its chord: {opening}"
    assert opening.get("kept"), f"the first chord into On since the start was made whole: {opening}"
    assert opening["rows"] == 0, f"the kept opening made rows before its frame: {opening}"


def test_chords_said_as_the_shell_restarts_open_it_once_it_listens(shell: Shell) -> None:
    """Chords said while the shell restarts open the launcher once the new one listens, and stay.

    launcher-say fell to qs ipc as soon as the socket refused it, and that fails too until the
    new shell has loaded, so Joel's chords through a switch's restart went unanswered for over
    a minute (2026-11-12, 17:08). He presses again when nothing opens, so two are said, the
    second shutting nothing the first opened.
    """
    assert shell.process is not None, "the shell was never started"
    shell.process.terminate()
    shell.process.wait(timeout=10)
    # The launcher-say the check gives, as Joel's chords run it.
    said = [subprocess.Popen([os.environ["LAUNCHER_SAY"], "toggle"])]  # noqa: S603
    time.sleep(PRESSED_AGAIN.total_seconds())
    # The same launcher-say again, the second chord.
    said.append(subprocess.Popen([os.environ["LAUNCHER_SAY"], "toggle"]))  # noqa: S603
    # Started as Shell.launch does, but not waited on to answer closed, as these chords open it.
    with shell.log.open("ab") as log:
        shell.process = subprocess.Popen(  # noqa: S603, the check's own shell folder
            ["qs", "-p", str(shell.folder)],  # noqa: S607, qs from the check's PATH
            stdout=log,
            stderr=subprocess.STDOUT,
        )
    exits = [each.wait(timeout=RESTARTED.total_seconds()) for each in said]
    deadline = time.monotonic() + RESTARTED.total_seconds()
    while (answered := shell.execute(shell.ipc("showing")))[0] != 0:
        assert time.monotonic() < deadline, f"the shell never answered:\n{shell.tail()}"
        time.sleep(launcher_on_niri.POLL)
    # As Shell.restart does, so the next word wtype types is read by its own keymap.
    subprocess.run(["wtype", "-k", "F24"], check=True)  # noqa: S607
    assert exits == [0, 0], f"launcher-say gave up on the restarting shell, exiting {exits}"
    shell.launcher_until(lambda shown: shown["open"], RESTARTED)
    time.sleep(STAYS.total_seconds())
    assert shell.showing()["open"], (
        f"the second chord shut what the first opened, first showing {answered[1]}:\n{shell.tail()}"
    )
    shell.launcher_ready()
    shell.close_launcher()
    shell.launcher_closed()


def test_an_opening_as_the_shell_starts_puts_off_keeping_its_places(shell: Shell) -> None:
    """An opening before the shell has kept its places puts that off until it's shut again.

    It ended the pass for good instead, as an opening or home kept by its closing came first, so
    Joel's first chord 14 s after a switch made 1,328 rows and was presented 1,024 ms after it
    (2026-11-12, 13:16).
    """
    since = shell.mark()
    shell.restart()
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_closed()
    deadline = time.monotonic() + WARMED.total_seconds()
    while WARMED_SAID not in shell.log.read_bytes()[since:].decode(errors="replace"):
        assert time.monotonic() < deadline, (
            f"opened as the shell started, it never kept its places once shut:\n{shell.tail()}"
        )
        time.sleep(launcher_on_niri.POLL)
    shell.launcher_until(lambda shown: shown.get("kept"))
    timed = latest(openings(shell))
    shell.as_account(shell.ipc("collection", "on"))
    shell.launcher_ready()
    each: list[Json] = []

    def timed_after(_: Json) -> bool:
        each[:] = openings(shell)
        return latest(each) > timed

    shell.launcher_until(timed_after)
    assert each[-1].get("kept"), f"the first chord into On was made whole: {each[-1]}"


def test_a_first_letter_after_a_restart_is_drawn_on_the_rows_it_lets_go(shell: Shell) -> None:
    """The first letter after a restart, listing home afresh, builds no row, as home's are let go.

    A view lets go of the rows a change removed only after it has made those the change put
    in, so a letter listing everything afresh, with no row yet let go, as on a shell's first
    key, built every row it drew: Joel's "m" built 8 in 15 ms of laying (2026-11-11). Home
    draws a card of rows and so does the letter, so the letter can draw on home's.
    """
    shell.restart()
    opened_from_kept(shell)
    before = json.loads(shell.as_account(shell.ipc("made")))
    shell.send_chars("c")
    shell.launcher_until(lambda shown: shown["typed"] == "c")
    settled(shell)
    after = json.loads(shell.as_account(shell.ipc("made")))
    built = int(after["built"]) - int(before["built"])
    drawn = int(after["drawn"]) - int(before["drawn"])
    assert built == 0, f"the letter built {built} of the {drawn} rows it drew"


def test_a_first_search_after_a_restart_makes_no_spots_on_its_key(shell: Shell) -> None:
    """Opened as the shell starts, before its spots are made, no key makes them.

    The first search of three letters made every spot when none were made yet: the first "bat"
    of a shell's life spent 82 ms making them, every later one 0 to 1 (Y2YBTS's lab,
    2026-11-11). Now they're made after the opening's frames, in slices.
    """
    since = shell.mark()
    shell.restart()
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    said, _ = shell.said_since(since)
    assert not any(line.startswith(SPOTS_MADE_SHUT[0]) for line in said), (
        f"the spots were made before the launcher opened, so nothing's tried: {said}"
    )
    shell.type_in("bat")
    shell.reported("spots made after frames in ", lambda _: True, since)
    said, _ = shell.said_since(since)
    assert "spots made on a key" not in said, f"a key made the spots: {said}"


def answered_since(shell: Shell, restarted: float) -> None:
    """Wait until every source has first answered since the restart at `restarted`.

    Home and the chords' places are kept a second and a half after a start, before Networks
    and Displays first answer here, so the next opening's fill makes their rows again, as it
    catches up any source changed since it was kept: 6 here, which aren't what this counts.
    It waits out ANSWERED, then until the changes each source has said hold still for QUIET.
    """
    time.sleep(max(0.0, restarted + ANSWERED.total_seconds() - time.monotonic()))
    deadline = time.monotonic() + SETTLED.total_seconds()
    said = json.loads(shell.as_account(shell.ipc("changed")))
    still = time.monotonic()
    while time.monotonic() - still < QUIET.total_seconds():
        if time.monotonic() > deadline:
            pytest.fail(f"sources still changing {SETTLED} after a restart: {said}")
        time.sleep(launcher_on_niri.POLL)
        now = json.loads(shell.as_account(shell.ipc("changed")))
        if now != said:
            said, still = now, time.monotonic()


def made_by_others(shell: Shell, place: str) -> dict[str, int]:
    """Open into the place by its chord, and give the rows each other source made meanwhile.

    A try a closing's spots landed in counts nothing of the opening's, so it's closed, its own
    spots waited for, and made again.
    """
    for _ in range(REOPENINGS_TRIED):
        began = shell.mark()
        before = json.loads(shell.as_account(shell.ipc("made")))["rows"]
        shell.as_account(shell.ipc("collection", place))
        shell.launcher_ready()
        after = json.loads(shell.as_account(shell.ipc("made")))["rows"]
        said, _ = shell.said_since(began)
        if not any(line.startswith(SPOTS_MADE_SHUT) for line in said):
            break
        closing = shell.mark()
        shell.close_launcher()
        shell.reported(SPOTS_MADE_SHUT[0], lambda _: True, closing)
    else:
        pytest.fail(f"a closing's spots were made as each of {REOPENINGS_TRIED} tries opened")
    return {
        name: rows - before.get(name, 0)
        for name, rows in after.items()
        if name != place and rows > before.get(name, 0)
    }


def test_a_place_its_chord_opened_is_drawn_as_it_stood(shell: Shell) -> None:
    """Its chord pressed again, a place is drawn from what was kept, there, with nothing made.

    A chord opened its place whole every time, 203 to 597 ms to its frame, while home drew from
    what was kept in 27 to 56 (Joel, 2026-11-10, 20:39 to 20:43).
    """
    shell.as_account(shell.ipc("collection", "on"))
    shell.launcher_ready()
    left = settled(shell)
    there = shell.showing()["path"]
    shell.close_launcher()
    shell.launcher_until(lambda shown: shown.get("kept"))
    timed = latest(openings(shell))
    shell.as_account(shell.ipc("collection", "on"))
    opening = filled_after(shell, timed)
    assert opening.get("into") == "on", f"the opening didn't say its chord: {opening}"
    assert opening["shown"]["keys"] == left["keys"], f"kept, the rows moved: {left} to {opening}"
    assert opening["rows"] == 0, f"the kept opening made rows before its frame: {opening}"
    assert shell.showing()["path"] == there, "filled in, it left the place its chord opened"


def test_a_place_its_chord_opened_weighs_its_keys_once_before_its_frame(shell: Shell) -> None:
    """Closed by :q and its chord pressed again, a place weighs its rows' keys once, not twice.

    Shut in normal, a reopening weighed Windows' keys for normal as its kept rows landed, then
    again for the find as it took the keys, 5 and 4 ms of its opening over 40 windows on a
    builder (ZINAWZ, 2026-11-11).
    """
    opening = windows_reopened(shell)
    weighings = cast("int | None", opening.get("weighings"))
    assert weighings is not None, f"the opening didn't say how often it weighed: {opening}"
    assert weighings <= 1, f"the kept opening weighed its keys {weighings} times: {opening}"


def windows_reopened(shell: Shell) -> Json:
    """Open Windows by its chord, close it, and give its next opening, from what it kept."""
    opened = [shell.opened(f"Reopened {at}") for at in range(WINDOWS_WEIGHED)]
    try:
        shell.as_account(shell.ipc("collection", "windows"))
        shell.launcher_ready()
        shell.launcher_until(lambda shown: len(shown["rows"]) >= WINDOWS_WEIGHED - 1)
        settled(shell)
        shell.close_launcher()
        shell.launcher_until(lambda shown: shown.get("kept"))
        timed = latest(openings(shell))
        shell.as_account(shell.ipc("collection", "windows"))
        return filled_after(shell, timed)
    finally:
        for window in opened:
            shell.close(window)


def test_a_place_shown_before_draws_no_words_to_measure_its_head(shell: Shell) -> None:
    """A place's head drawn before is measured from what was kept, not drawn again to be.

    Each kept opening drew its head's words in a Text to measure them, three or four, and
    worked out its path six times, about 3 ms of Windows' opening over 40 windows on a builder
    (ZINAWZ, 2026-11-11).
    """
    opening = windows_reopened(shell)
    measured = cast("int | None", opening.get("measured"))
    assert measured == 0, f"the kept opening drew {measured} words to measure: {opening}"


def test_a_kept_opening_tells_only_its_own_rows_where_they_stand(shell: Shell) -> None:
    """A kept opening tells its rows their place once they're drawn, not home's before they go.

    Told as the rows were set, the place reached home's rows still drawn, each saying its
    kind again just before it gave way, about 3 ms of Windows' opening over 40 windows on a
    builder (ZINAWZ, 2026-11-11).
    """
    opening = windows_reopened(shell)
    told = cast("int | None", opening.get("toldEarly"))
    assert told == 0, f"the kept opening told rows about to go {told} times: {opening}"


def test_a_kept_opening_says_it_is_open_from_its_first_frame(shell: Shell) -> None:
    """A kept opening's mark reads open from its first frame, written without holding it.

    The mark said open only at the fill, which waits for niri to present the first frame or
    half a second, so for that long a switch waiting on it read closed with the card on the
    screen and could restart the shell under it (Y2YBTS, 2026-11-11). Written blocking at the
    first frame, it held the fill 3 to 121 ms on a loaded builder instead.
    """
    opening = opened_from_kept(shell)
    marked = cast("int", opening.get("marked", -1))
    assert 0 <= marked <= cast("int", opening["took"]), f"not marked open by its frame: {opening}"
    blocked = cast("int", opening.get("blocked", -1))
    assert blocked == 0, f"the opening waited on {blocked} writes of its mark: {opening}"


def test_an_open_mark_it_cannot_write_is_said_once(shell: Shell) -> None:
    """An open mark the shell can't write is said once, though each opening and closing fails.

    Its readers go on reading the last word written, so without it nothing would say why a
    switch waited on the launcher, or didn't (Y2YBTS, 2026-11-11). A folder in its place
    refuses every write.
    """
    mark = Path(os.environ["XDG_RUNTIME_DIR"]) / "quickshell-launcher"
    began = shell.mark()
    mark.unlink(missing_ok=True)
    mark.mkdir()
    try:
        for _ in range(2):
            shell.as_account(shell.ipc("collection", "windows"))
            shell.launcher_ready()
            shell.close_launcher()
    finally:
        mark.rmdir()
    with shell.log.open("rb") as log:
        log.seek(began)
        said = log.read().decode(errors="replace").count(MARK_UNWRITTEN)
    assert said == 1, f"an unwritten mark was said {said} times over two openings"


def weighed_reopening(shell: Shell, shown: Callable[[Json], bool]) -> int:
    """Reopen Windows by its chord from what it kept, and give how many rows its foot weighed."""
    before = cast("int", shell.showing()["weighed"])
    timed = latest(openings(shell))
    shell.as_account(shell.ipc("collection", "windows"))
    filled_after(shell, timed)
    shell.launcher_until(shown)
    settled(shell)
    weighed = cast("int", shell.showing()["weighed"]) - before
    shell.close_launcher()
    shell.launcher_until(lambda now: now.get("kept"))
    return weighed


def test_a_kept_place_shown_again_weighs_only_the_row_that_changed(shell: Shell) -> None:
    """Reopened from what it kept, a place weighs none of its rows' keys but one that changed.

    Going home and back weighed every one of Windows' rows again on each kept opening, 3.5 to
    4.8 ms of it over 40 windows on a builder (ZINAWZ, 2026-11-11). A window retitled while
    the launcher is shut is another row, so its keys are weighed again, and only its.
    """
    retitling = Path(os.environ["XDG_RUNTIME_DIR"]) / "reweighing"
    go = retitling.with_suffix(".go")
    # foot takes its title from what runs in it, once the test says to.
    retitling.write_text(
        f"while [ ! -e {go} ]; do sleep 0.1; done\nprintf '\\033]2;{RETITLED}\\007'\nsleep 3600\n"
    )
    # Opened first, as Windows leaves out the window that has the focus.
    opened = [shell.opened("Gannet", running=f"sh {retitling}")]
    opened.extend(shell.opened(f"Reweighed {at}") for at in range(WINDOWS_WEIGHED))
    try:
        shell.as_account(shell.ipc("collection", "windows"))
        shell.launcher_ready()
        shell.launcher_until(lambda shown: "Gannet" in shown["rows"])
        settled(shell)
        shell.close_launcher()
        shell.launcher_until(lambda shown: shown.get("kept"))
        again = weighed_reopening(shell, lambda shown: "Gannet" in shown["rows"])
        go.touch()
        changed = weighed_reopening(shell, lambda shown: RETITLED in shown["rows"])
    finally:
        for window in opened:
            shell.close(window)
        go.unlink(missing_ok=True)
    assert again == 0, f"Windows shown again weighed {again} rows' keys"
    assert changed == 1, f"Windows with one window retitled weighed {changed} rows' keys"


def test_a_place_kept_over_one_window_is_drawn_as_kept_over_another(shell: Shell) -> None:
    """Kept over one window, a place whose rows follow none is drawn as kept over another's.

    A place was drawn as kept only over a window of the app it was kept over, though only
    Shortcuts' rows follow the window's keys and Windows' the window itself, so a chord pressed
    over any other program's window made its place whole.
    """
    shell.as_account(shell.ipc("collection", "on"))
    shell.launcher_ready()
    left = settled(shell)
    shell.close_launcher()
    window = shell.opened("Plover Notes", app="plover")
    try:
        # The span measured, for the window's focus to reach the shut shell, not a wait for an end.
        time.sleep(SETTLING_S)
        shell.launcher_until(lambda shown: shown.get("kept"))
        timed = latest(openings(shell))
        shell.as_account(shell.ipc("collection", "on"))
        opening = filled_after(shell, timed)
        assert opening["shown"]["keys"] == left["keys"], f"kept, the rows moved: {opening}"
    finally:
        shell.close(window)


def test_windows_kept_over_one_window_is_drawn_as_kept_over_another(shell: Shell) -> None:
    """Windows' chord over another window draws it from what was kept, leaving that one out.

    Kept only over the window it was kept over, Windows was made whole on every Super+Tab, since
    it's pressed over the window it last went to: 84 ms to its frame on Joel's (2026-11-11,
    16:23), never drawn from what was kept since Tuesday.
    """
    gull = shell.opened("Gull Notes")
    tern = shell.opened("Tern Notes")
    try:
        shell.as_account(shell.ipc("collection", "windows"))
        shell.launcher_ready()
        shell.launcher_until(lambda shown: "Gull Notes" in shown["rows"])
        settled(shell)
        shell.close_launcher()
        shell.as_account(f"niri msg action focus-window --id {gull['id']}")
        # The span measured, for the window's focus to reach the shut shell, not a wait for an end.
        time.sleep(SETTLING_S)
        shell.launcher_until(lambda shown: shown.get("kept"))
        timed = latest(openings(shell))
        shell.as_account(shell.ipc("collection", "windows"))
        keys = cast("list[str]", filled_after(shell, timed)["shown"]["keys"])
        assert keys, "the kept opening drew no windows"
        assert keys[0] == f"window:{tern['id']}", f"the window used last wasn't first: {keys}"
        assert f"window:{gull['id']}" not in keys, f"it listed the window under it: {keys}"
    finally:
        shell.close(tern)
        shell.close(gull)


def test_a_reopening_puts_off_the_spots_its_closing_left(shell: Shell) -> None:
    """Reopened as the closing's spots come due, the kept opening makes none before its frame.

    A closing makes its places' spots half a second after it, and one reopened then made
    Network's rows between its ask and its first frame (a gate of #2971, 2026-11-10). Niri is
    held from the closing until past that half second, so the reopening's first frame can't
    come before the spots are due, however quickly the shell turns. Rows follow only from a
    place changed while shut, so the spots being made at all is what fails. Both toggles are
    said down the launcher's socket from here, the reopening once the shell's log says it's
    closing: as two IPC calls, each starting a process, a busy builder took past the half
    second on every try (five gates, 10 to 12 November), and such a try shows nothing.
    """
    for _ in range(REOPENINGS_TRIED):
        first = opened_from_kept(shell)
        began = shell.mark()
        with launcher_on_niri.niri_held():
            closed = time.monotonic()
            told("toggle")
            # Home is kept in the turn after the closing says so, before the next line is read.
            heard_closing(shell, began, closed + SPOTS_LATER.total_seconds())
            told("toggle")
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


def test_a_letter_reordering_a_kept_place_lists_it_afresh_from_its_first_row(
    shell: Shell,
) -> None:
    """A first letter in a kept Emoji, ranking hundreds of its rows anew, starts on its first.

    Those rows moved into place one at a time, each move reading the rows in view again, so the
    letter's frame came 2.4 to 7.3 s after it (H6EDLC, 2026-11-11); listed afresh instead, the
    selection stands on the first row found, drawn there rather than gliding, and the list
    starts at its top and holds every row found.
    """
    shell.as_account(shell.ipc("collection", "emoji"))
    shell.launcher_ready()
    settled(shell)
    shell.close_launcher()
    shell.launcher_until(lambda shown: shown.get("kept"))
    timed = latest(openings(shell))
    shell.as_account(shell.ipc("collection", "emoji"))
    filled_after(shell, timed)
    settled(shell)
    for _ in range(ROWS_DOWN):
        shell.send_key("down")
    shell.launcher_until(lambda shown: cast("int", shown["seen"]["at"]) == ROWS_DOWN)
    shell.send_chars("f")
    shell.launcher_until(lambda shown: shown["typed"] == "f")
    laid = json.loads(shell.as_account(shell.ipc("laidOut")))
    selection = laid["selection"]
    assert selection is not None, f"the letter left no selection drawn: {laid['rows']}"
    assert selection["y"] == selection["row"], f"the selection glided to its row: {selection}"
    assert settled(shell)["at"] == 0, f"the letter left the cursor off the first row: {laid}"
    laid = json.loads(shell.as_account(shell.ipc("laidOut")))
    assert laid["firstAt"] == 0, f"the list didn't start at its first row: {laid['firstAt']}"
    found = cast("int", shell.showing()["count"])
    assert laid["listed"] == found, f"it listed {laid['listed']} of the {found} rows found"


def test_a_backspace_to_a_kept_places_whole_list_lists_it_again(shell: Shell) -> None:
    """A backspace back to kept Emoji's whole list is the list it showed, worked out once.

    Each key walked all 1,914 of its rows again to key them, measure their sides and weigh their
    keys, though they were the very rows it listed before the letter (H6EDLC, 2026-11-11).
    """
    shell.as_account(shell.ipc("collection", "emoji"))
    shell.launcher_ready()
    settled(shell)
    shell.close_launcher()
    shell.launcher_until(lambda shown: shown.get("kept"))
    timed = latest(openings(shell))
    shell.as_account(shell.ipc("collection", "emoji"))
    filled_after(shell, timed)
    whole = settled(shell)
    before = cast("int", shell.showing().get("listedAgain", 0))
    shell.send_chars("h")
    shell.launcher_until(lambda shown: shown["typed"] == "h")
    shell.send_key("backspace")
    shell.launcher_until(lambda shown: shown["typed"] == "")
    back = settled(shell)
    assert back["keys"] == whole["keys"], f"the backspace listed {back}, not {whole}"
    again = cast("int", shell.showing().get("listedAgain", 0)) - before
    assert again == 1, f"the backspace listed the whole list again {again} times, not once"


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
    assert opening.get("filledBy") in FILLERS, f"nothing filled it in: {opening}"


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


def test_a_kept_opening_fills_in_once_niri_has_presented_its_frame(shell: Shell) -> None:
    """A kept opening fills in only once niri has presented the frame it was drawn in.

    Filled as soon as that frame was swapped, the fill's work held the frame back from the
    screen: niri presented it a median 132 ms after it was drawn, against 33 ms for a letter's,
    and 20 ms once the fill waited (24 kept openings a side, 2026-11-11). Each of a few openings
    begins its fill no sooner than niri presented it, but one niri never said it presented
    within the launcher's wait, filled anyway.
    """
    waited: list[Json] = []
    seen: list[Json] = []
    for _ in range(PRESENTED_OPENINGS):
        opening = presented_after(shell, opened_from_kept(shell)["nth"])
        seen.append(opening)
        assert opening["presented"] >= 0, f"niri never presented the opening: {opening}"
        if opening["filledBy"] != "unpresented":
            assert opening["fillFrom"] >= opening["presented"], (
                f"the opening filled in before niri presented its frame: {opening}"
            )
            waited.append(opening)
        shell.launcher_left_closed()
    assert waited, f"niri presented no opening within the launcher's wait: {seen}"


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
    That was the first making since the start: each row is kept by the key the notices daemon
    serves it by (ReadyRows.qml), so an opening makes none again and a new notice makes its own.
    Each is counted once its card has rested: a notice that rests is changed, so its row is
    made again, and one resting between openings made the second one 1 (merge-net's run at
    a1be64a0fc77, 2026-11-11).
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

    def sent(summary: str, body: str) -> None:
        """Send a notice, and give it once its card has shown and its time has run out."""
        with shell.told(f"rests {summary}"), shell.told(f"shows card {summary}"):
            shell.as_account(f"notify-send --expire-time=1000 '{summary}' '{body}'")

    for summary in ["Dunlin", "Sanderling"]:
        sent(summary, "a notice kept")
    searched("dunlin")
    again = searched("sanderling")
    sent("Turnstone", "a notice come since")
    came = searched("turnstone")
    assert again == 0, f"opened again with no notice come, it made {again} notices' rows"
    assert came == 1, f"one notice come, an opening made {came} notices' rows"


def test_for_you_makes_only_the_rows_whose_source_changed(shell: Shell) -> None:
    """Opened into For You again, and told a notice while open, it makes only what changed.

    Opening into For You again with nothing come made Time's 7 rows, Today's 10 and Location's
    (2026-11-11): each opening placed the time anew, and Location's row was made anew as Now
    was wanted again. Each is kept now while it says the same, so a kept opening makes none of
    them, and a notice heard while it's open makes the one row of the source it came to:
    Desktop's, which holds the notices unread (Y2YBTS, 2026-11-11). Each reopening comes once
    the closing before it has made its spots, as Joel's do: reopened within half a second, one
    made Desktop's two rows inside once, as they're first listed.
    """

    def made() -> dict[str, int]:
        return cast("dict[str, int]", json.loads(shell.as_account(shell.ipc("made")))["rows"])

    def since(before: dict[str, int]) -> dict[str, int]:
        now = made()
        return {
            name: now[name] - before.get(name, 0) for name in now if now[name] > before.get(name, 0)
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

    def shut() -> None:
        """Close, and wait for what the closing makes half a second on (spotsLater)."""
        since_closing = shell.mark()
        shell.close_launcher()
        shell.reported("spots made while closed ", lambda _: True, since_closing)

    def within_a_minute(act: Callable[[], dict[str, int]]) -> dict[str, int]:
        """Give the rows the act made, done again once if a minute turned, which remakes some.

        A turn counts if the clock says one came, or if the shell said it heard one, however
        late: a shell that heard the turn after the clock's slack made Time's and Today's rows
        again rightly, and failed the case (Y2YBTS, 2026-11-11, at 0cb9591c). Each try begins
        early in a minute (MINUTE_BEGUN), so one only a turned clock remade is rare.
        """
        for _ in range(2):
            minute_begun()
            began = time.time()
            heard_from = shell.mark()
            made_by_it = act()
            turned = "the clock turned a minute" in shell.said_since(heard_from)[0]
            same = int((began - CLOCK_SLACK) // 60) == int((time.time() + CLOCK_SLACK) // 60)
            if same and not turned:
                break
        return made_by_it

    def reopened() -> dict[str, int]:
        """Open For You and open it again, giving what the second made.

        The minute is read before the first of the two places the time, since one turning
        between them remakes the time rightly, which a minute read only around the second
        missed (Y2YBTS, 2026-11-11, at 1f562159).
        """
        shut()
        in_for_you()
        shut()
        before = made()
        in_for_you()
        return since(before)

    def notified() -> dict[str, int]:
        before = made()
        with shell.told("shows card Whimbrel"):
            shell.as_account("notify-send --expire-time=1000 'Whimbrel' 'a notice heard open'")
        settled(shell)
        return since(before)

    in_for_you()
    again = within_a_minute(reopened)
    heard = within_a_minute(notified)
    shell.close_launcher()
    assert again == {}, f"opened into For You again with nothing come, it made {again}"
    assert heard == {"on": 1}, f"a notice heard in For You made {heard}, not Desktop's row"


CHANGES_SAID = " from the line, the slowest "  # a close's line for each place's timed changes


def test_a_reopening_with_nothing_changed_times_no_change(shell: Shell) -> None:
    """A kept opening times no change when nothing changed (watches' answers were, 2026-11-11)."""
    window = shell.opened("Tern Notes")
    try:
        for _ in range(2):
            opened_from_kept(shell)
            settled(shell)
            since = shell.mark()
            shell.close_launcher()
    finally:
        shell.close(window)
    said = shell.log.read_bytes()[since:].decode(errors="replace").splitlines()
    timed = [line for line in said if CHANGES_SAID in line]
    assert timed == [], f"a reopening timed what its watches caught up on: {timed}"


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
        told(f"toggle @{time.time():.6f}")
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
