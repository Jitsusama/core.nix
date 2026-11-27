"""The launcher answers what's typed, on the account's whole shell on niri.

modules/quickshell/checks/core.nix runs this through pytest inside tests/lib/nested-niri.sh, with
the shell launcher_on_niri.py starts once a run. Each case here needs only the launcher or the
notices, what they're typed and what they draw, so none boots a machine; a case needing the
machine's battery, lights, services, other programs or niri's binds pressed stays in
tests/desktop/. Every test starts with the
launcher closed and leaves it so, so any test runs alone.
"""

from __future__ import annotations

import contextlib
import errno
import functools
import itertools
import json
import logging
import math
import os
import re
import shlex
import shutil
import socket
import subprocess
import threading
import time
from datetime import timedelta
from pathlib import Path
from typing import TYPE_CHECKING, cast

import launcher_on_niri
import pytest

if TYPE_CHECKING:
    from collections.abc import Callable, Iterator

    from launcher_on_niri import Json, Shell

# How long a place asked cold may take to be read: a daemon's first answer, well under one.
READ = timedelta(seconds=5)
# How long a terminal's show counts as seeing its rows (Launcher.qml's terminalSeeing), and a
# second past it. Nothing says the window closed, so the case waits it out.
SEEING_ENDS = timedelta(seconds=4)
# How long a place's members take to say their first answers once it's stepped into, each of
# which makes the step again: a change told after it is said only by the row it moves.
SETTLED = timedelta(seconds=3)
# How long before its line a chord is said to have run, well past any opening's own time here.
CHORD_AGO = timedelta(seconds=2)
# How long the shell is held stopped just after a chord, well past any opening's own time here.
HELD = timedelta(seconds=2)
# How long from its key a step out is held at most: its key's own work takes milliseconds.
HELD_WITHIN = timedelta(seconds=1)
# How much of niri's screen an opening's card changes at least, where nothing else moves.
CARD_SHARE = 0.02
# How many bytes of niri's screen are compared at once, a few rows of pixels.
STRETCH = 4096
# How long before the time said niri's screen may already show a change: niri's screencopy copies
# what the shell committed as it's asked, while niri presents it only with its next frame of the
# output, which waits on the headless sway it's nested in for its turn to draw. On a quiet builder
# a copy led the time said by 23 ms, and on a loaded one by 78 and 133 ms (2026-11-10), so the
# lead says how far a nested niri may trail, not how late the measure may be. A time tied to a
# drawing well after the card's first, such as the closing's, still fails; a one-frame miscount
# shows on Joel's own screen, where niri presents at the display's flip.
COPIED_BEFORE_PRESENTED = timedelta(milliseconds=250)

# How a key's line says its drawing, and what its render thread's times make of it (Trials.qml's
# drawingApart), each part rounded to a millisecond apart, so the three may miss the whole by two.
DRAWING_SPLIT = re.compile(
    r"(?P<drawing>\d+) drawing it \((?P<syncing>\d+) syncing, (?P<rendering>\d+) rendering,"
    r" (?P<heard>\d+) before the shell heard it swapped\)"
)
ROUNDED = 2
# How a key's line says the frames begun since it was heard (Trials.js's framesBeforeSaid), read
# back from its own frame's number, which a wrong number or clock leaves unsaid.
FRAMES_BEFORE = re.compile(r", after (?:at least )?\d+ other frames? since it was heard")
# How the close says each key's press, swap and screen in monotonic ms (Presented.keysClockSaid),
# the press in whole ms as niri stamps it and the rest to a tenth, "-" for one not timed, so it
# lines up with niri's own log of its frames.
KEYS_CLOCK = re.compile(
    r"launcher: \d+ keys shown, each pressed, swapped and shown in monotonic ms: (.+)\n"
)
CLOCK_KEY = re.compile(
    r"(-|\d+) (-|[\d.]+) ([\d.]+)(?:, (\d+) frames? of the shell's after its own)?"
)
# A tenth's rounding, with room for the float either side of it.
TENTH = 0.051
# The most facts a home network thing's drawer holds, the rest its step in's (modules/eero's
# drawers.rs GLANCE).
GLANCE = 6


@pytest.fixture(scope="module")
def shell() -> Iterator[Shell]:
    """Give the shell, started once a run."""
    started = launcher_on_niri.started()
    yield started
    started.stop()


@pytest.fixture(autouse=True)
def launcher_closed_after(shell: Shell) -> Iterator[None]:
    """Close the launcher after each test, so one failing with it open fails alone."""
    yield
    shell.launcher_left_closed()


def now(*form: str) -> str:
    """Give the time now, as date gives it in this form."""
    done = subprocess.run(["date", *form], capture_output=True, text=True, check=True)
    return done.stdout.strip()


def copied(shell: Shell, typed: str, expected: str) -> None:
    """Ask the launcher, take its answer with Return, and check that it copied this."""
    shell.answer(typed)
    # It says so as it closes.
    with shell.told("parted copied"):
        shell.send_key("ret")
    shell.launcher_closed()
    assert shell.copied() == expected


def yanked(shell: Shell, typed: str) -> str:
    """Ask the launcher, copy its answer by yy from normal, and give what was copied.

    An answer holding parts is gone into by Return, as every row holding more is, so y is
    what copies it, and the launcher stays until it's closed.
    """
    shell.answer(typed)
    shell.normal()
    since = shell.mark()
    shell.send_chars("yy")
    shell.reported("yanked 1", lambda rest: rest == "", since)
    taken = shell.copied()
    shell.close_launcher()
    return taken


def answer_is_whole(shell: Shell, answer: str) -> None:
    """Wait for the answer to come, and check it's drawn whole in the row it held.

    The row says "Ask" until the answer comes, and is kept as it does, so what it then says
    must not be cut to the room "Ask" took, as the gallery's launcher-units once showed it.
    """
    shell.launcher_until(lambda shown: shown["rows"][:1] == [answer])
    assert shell.held_still(), "the launcher never came to rest"
    words = json.loads(shell.as_account(shell.ipc("laidOut")))["rows"][0]["words"]
    name = next(word for word in words if re.sub("<[^>]*>", "", word["text"]) == answer)
    assert not name["cut"], words


def shown_to_terminal(shell: Shell, *steps: str) -> Json:
    """Give what `launcher show` is answered at these steps, asked as the program asks."""
    # Each step after the mark, since qs takes no empty argument, and after `--`, since
    # `show` is also qs ipc's own command otherwise.
    path = shlex.quote("".join(f"\u203a{step}" for step in steps) or "\u203a")
    asked = f"qs -p {shell.folder} ipc call -- launcher show {path}"
    return cast("Json", json.loads(shell.as_account(asked)))


def test_a_place_asked_cold_from_a_terminal_is_read_for_it(shell: Shell) -> None:
    """A place nothing has shown yet is read for a terminal, with the launcher closed.

    Machine asks its daemon only while its rows are seen (launcher/Machine.qml's shown), and
    a terminal's walk counts as seeing them for a few seconds (Launcher.qml's terminalSees),
    so the answer says `reading` until the daemon's first answer comes and the next ask has
    the figures. It runs first, before anything opens Machine, so the first ask meets the
    placeholder. Once the window ends, Machine stops watching, so a second cold ask is read
    afresh rather than said from what the first left, which may be hours old.
    """

    def processor(answer: Json) -> object:
        return next(row.get("side") for row in answer["rows"] if row["name"] == "Processor")

    def read_after(first: Json) -> Json:
        answer = first
        deadline = time.monotonic() + READ.total_seconds()
        while answer["reading"] and time.monotonic() < deadline:
            time.sleep(launcher_on_niri.POLL)
            answer = shown_to_terminal(shell, "machine")
        assert not answer["reading"], answer
        return answer

    first = shown_to_terminal(shell, "machine")
    assert first["reading"], first
    assert processor(first) == "\u2026", first["rows"]
    assert processor(read_after(first)) not in {None, "\u2026"}
    time.sleep(SEEING_ENDS.total_seconds())
    again = shown_to_terminal(shell, "machine")
    assert again["reading"], "the window never ended, or the watch kept its first answer"
    assert processor(read_after(again)) not in {None, "\u2026"}
    assert not shell.showing()["open"]


def test_units_zones_colours_and_whats_offline_are_answered(shell: Shell) -> None:
    """Units, zones and colours are answered, and what needs a network says it couldn't."""
    shell.answer("350 f in c")
    answer_is_whole(shell, "176.667 \u00b0C")
    shell.close_launcher()
    # The zones come from the list built from tzdata, so this proves it's
    # there. The minute may turn while it's asked, so either will do.
    before = now("-u", "+%H:%M")
    assert yanked(shell, "time in utc") in [before, now("-u", "+%H:%M")]
    # A theme's colour is found by its name, as the theme exports it.
    theme = json.loads(shell.as_account("cat $XDG_CONFIG_HOME/jitsusama/theme.json"))
    assert yanked(shell, "colour accent") == theme["colours"]["accent"].lower()
    # Slack takes its theme only by hand, so the launcher copies it,
    # in the theme's own colours.
    slack = ["accent", "selection", "success", "warning"]
    assert yanked(shell, "slack theme") == ",".join(theme["colours"][c] for c in slack)
    # The build has no network, so what has to ask one says it couldn't,
    # and nothing breaks. Each waits for its answer to settle, failed;
    # asked again while that failure stands, it says so at once. When a
    # failure is asked again is the launcher's answers check's to say.
    for typed in ["8.8.8.8", "weather tokyo", "100 usd in cad", "address"]:
        shell.answer(typed)
        shell.close_launcher()
    shell.answer("weather tokyo")
    # Come to nothing, it says why once, as its one row, quietly: the
    # field finishes nothing and its end adds nothing (VKCF3Z, 2026-11-02:
    # "= no forecast" beside "Open-Meteo didn't a..." said it twice).
    shell.launcher_until(
        lambda shown: (
            shown["rows"] == ["Open-Meteo didn't answer."]
            and shown["completion"] == ""
            and shown["footer"] == ""
            and shown["kind"] == ""
        )
    )
    shell.close_launcher()
    # The rain is the weather's to answer: here asks where here is, since
    # nothing here says, and a place's rain asks the same forecast. That
    # the next rain is said from the hours is the launcher's rain check's
    # to show.
    shell.answer("rain")
    shell.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "where?")
    shell.close_launcher()
    shell.answer("rain in tokyo")
    shell.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name") == "Open-Meteo didn't answer."
            and shown["footer"] == ""
        )
    )
    shell.close_launcher()


def test_return_goes_into_an_answer_with_parts_and_y_copies_it(shell: Shell) -> None:
    """Return goes into an answer holding parts, and y copies it, as a sum's Return does.

    Return is the one way in, so a copy on it hid every part (Joel, 2026-11-06: "i type
    memory in the launcher, hit enter and it copies and does not go into more details").
    """
    for typed, part in [("memory", "Used"), ("processor", "Load")]:
        shell.answer(typed)
        shell.launcher_until(lambda shown: shown["kind"] == "answer")
        shown = shell.showing()
        assert (shown["selected"] or {}).get("told", [""])[0] == "\u21b5 open", shown
        shell.step_in()
        shell.launcher_until(lambda shown: part in shown["rows"])  # noqa: B023
        shell.close_launcher()
    # From normal, y copies the answer Return went into, and keeps the launcher.
    shell.answer("memory")
    shell.launcher_until(lambda shown: "B used" in (shown["selected"] or {}).get("name", ""))
    shell.normal()
    assert "y copy" in shell.showing()["selected"]["told"], shell.showing()["selected"]
    since = shell.mark()
    shell.send_chars("yy")
    shell.reported("yanked 1", lambda rest: rest == "", since)
    assert "B used" in shell.copied(), shell.copied()
    shell.close_launcher()
    # A sum holds nothing to go into, so its Return copies it still.
    copied(shell, "12*7", "84")


def test_arming_goes_as_the_launcher_shuts(shell: Shell) -> None:
    """A row armed is let go as the launcher shuts, by a key the line never takes.

    Only a key the line took let arming go, so Return on Reboot, Super+Space to shut it, and an
    hour later Super+Space and a click on Reboot, no key between, rebooted (Y2YBTS,
    2026-11-12).
    """
    shell.answer("reboot")
    shell.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Reboot")
    shell.send_key("ret")
    shell.launcher_until(
        lambda shown: shown["armed"] == "again reboots" and shown["armedRow"] != ""
    )
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_until(lambda shown: not shown["open"] and shown["armedRow"] == "")
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    shell.launcher_until(lambda shown: shown["armedRow"] == "")
    shell.close_launcher()


MOON_PHASES = (
    "new moon|waxing crescent|first quarter|waxing gibbous|full moon|waning gibbous"
    "|last quarter|waning crescent"
)


def test_the_dates_drawer_says_its_week_and_its_moon_copies_on_return(shell: Shell) -> None:
    """The date's drawer says its week and season in a sentence, and the moon copies on Return.

    The date and its parts had no drawer, and the moon, the season and the zone each opened a
    list of figures to copy one by one (Joel, 2026-11-11: "side drawers for just about every
    item", and "if they have nothing under them, we shouldn't open a submenu for them").
    """
    shell.answer("date")
    shell.launcher_until(
        lambda shown: (
            bool(shown["drawer"]["shown"])
            and str(shown["drawer"]["body"]).startswith("Week ")
            and {"ISO Date", "Season", "Moon", "Quarter"} <= set(shown["drawer"]["facts"])
        )
    )
    shell.step_in()
    shell.launcher_until(lambda shown: "Moon" in shown["rows"])
    shell.down_to("Moon")
    shell.launcher_until(lambda shown: shown["drawer"]["facts"][:1] == ["Lit"])
    shell.send_key("ret")
    shell.launcher_closed()
    assert re.fullmatch(MOON_PHASES, shell.copied()), shell.copied()


def test_an_openings_first_drawer_is_kept_once_the_launcher_shuts(shell: Shell) -> None:
    """How long an opening's first drawer took is kept as the launcher shuts, past a restart.

    Learnt in memory alone, and only from a drawer already out, a kind Joel steps onto a few times
    a day had none at most keys after a restart, so its cheap drawer came the frame after: Wi-Fi's,
    made in a median of 6 ms, came a median of 53 ms from its key (F8TT8O, 2026-11-18).
    """
    state = Path(os.environ.get("XDG_STATE_HOME") or Path.home() / ".local/state")

    def kept() -> dict[str, Json]:
        said: dict[str, Json] = {}
        for path in state.glob("quickshell/by-shell/*/launcher-drawers.json"):
            if path.read_text().strip():
                said.update(cast("dict[str, Json]", json.loads(path.read_text())))
        return said

    def learnt() -> set[str]:
        """Say the kinds the file keeps a making for, each a list of times."""
        return {kind for kind, times in kept().items() if isinstance(times, list) and times}

    shell.answer("date")
    shell.launcher_until(
        lambda shown: (
            bool(shown["drawer"]["shown"]) and str(shown["drawer"]["body"]).startswith("Week ")
        )
    )
    shell.close_launcher()
    deadline = time.monotonic() + 5
    while not learnt() and time.monotonic() < deadline:
        time.sleep(0.05)
    kinds = learnt()
    assert kinds, f"the launcher kept no drawer's making as it shut: {kept()}"

    # What's kept is read as the shell starts again, before any drawer is made: the restarted
    # launcher knows each kind the file holds as it opens.
    shell.restart()
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    try:
        shell.launcher_until(lambda shown: kinds <= set(shown["drawer"]["kinds"]))
    finally:
        shell.close_launcher()


def test_a_drawers_nerdy_bits_open_on_za_and_stay_as_left(shell: Shell) -> None:
    """A drawer's nerdy bits open and fold on za, and they stay as left from row to row.

    A drawer ends with the raw figures behind its facts, folded until za (VKCF3Z's drawer spec,
    part 7), and a fold he opened stays open into the next drawer, as a Vim fold he chose.
    """
    shell.answer("date")
    shell.launcher_until(
        lambda shown: (
            shown["drawer"]["nerd"] == ["ISO Week Date", "Julian Day"]
            and shown["drawer"]["folded"] is True
        )
    )
    shell.normal()
    shell.send_chars("za")
    shell.launcher_until(lambda shown: shown["drawer"]["folded"] is False)
    shell.close_launcher()
    shell.answer("time")
    shell.launcher_until(
        lambda shown: (
            shown["drawer"]["nerd"] == ["Unix Time", "TAI"] and shown["drawer"]["folded"] is False
        )
    )
    shell.normal()
    shell.send_chars("za")
    shell.launcher_until(lambda shown: shown["drawer"]["folded"] is True)
    shell.close_launcher()


def test_a_drawers_prose_shows_five_lines_and_za_opens_the_rest(shell: Shell) -> None:
    """A notice's long words show five lines in its drawer, the rest opened by za.

    Only the drawer wraps, and its prose five lines at most, the rest in its fold (VKCF3Z's
    wrap rule 4), so a long message no longer stands in for the rest of its drawer.
    """
    # How many lines its prose shows while the fold is shut (Drawer.qml's proseMost).
    most = 5
    words = " ".join(f"sentence {at} of a notice that runs on." for at in range(1, 41))
    with shell.told("shows card Quillwort"):
        shell.as_account(f"notify-send --expire-time=1000 Quillwort {shlex.quote(words)}")
    shell.notices_gone()
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    shell.type_in("quillwort", selecting="quillwort")
    shell.launcher_until(
        lambda shown: (
            shown["drawer"]["out"]
            and shown["drawer"]["title"] == "Quillwort"
            and shown["drawer"]["folded"] is True
            and shown["drawer"]["bodyLines"] == most
        )
    )
    drawer = shell.showing()["drawer"]
    said = str(drawer["bodyDrawn"])
    assert said.endswith("\u2026"), drawer
    assert words.startswith(said[:-1]), drawer
    shell.normal()
    shell.send_chars("za")
    shell.launcher_until(
        lambda shown: (
            shown["drawer"]["folded"] is False
            and shown["drawer"]["bodyDrawn"] == words
            and shown["drawer"]["bodyLines"] > most
        )
    )
    shell.send_chars("za")
    shell.launcher_until(lambda shown: shown["drawer"]["folded"] is True)
    shell.close_launcher()


# How many lines going into a drawer's page laid for its frame, as the shell says once niri
# presents it (launcher/Presented.js's intoSaid), with the page it named.
LAID_GOING_IN = re.compile(
    r'launcher: into the drawer for "(?:[^"\\]|\\.)*"(?: on ("(?:[^"\\]|\\.)*"))? presented .* '
    r"(\d+) lines? laid first"
)


def laid_going_in(shell: Shell, mark: int, page: str) -> int:
    """Wait for the shell to say it went into the page since the mark, and give the lines laid.

    Only the page's own line is read, since another going-in under the same row, as its list
    of keys is, may be said after the mark, its frame presented late.
    """
    deadline = time.monotonic() + launcher_on_niri.WAITING.total_seconds()
    while time.monotonic() < deadline:
        with shell.log.open("rb") as log:
            log.seek(mark)
            written = log.read().decode(errors="replace")
        # An untitled page's line names none, so it's never the page waited for.
        for found in LAID_GOING_IN.finditer(written):
            if found.group(1) is not None and json.loads(found.group(1)) == page:
                return int(found.group(2))
        # The line is written once niri presents the frame, so it's read again a poll apart.
        time.sleep(launcher_on_niri.POLL)
    failed = f"going into {page} was never said presented"
    raise AssertionError(failed)


def test_a_places_run_is_laid_whole_for_its_keys_and_kept_where_they_left(
    shell: Shell,
) -> None:
    """Emoji's drawer lays its run short at rest, whole once Ctrl+W l goes in, short as left.

    A place's members are laid at rest only as far as the drawer shows, since Emoji's 1,914
    names laid whole cost about 200 ms a drawer (Drawer.qml's runMost). The keys read every
    line, so they lay the rest a part a turn, and all at once for a key that reads past
    what's laid; leaving it scrolled deep keeps the view there, so the run is laid that far
    again, with more below, rather than leaving the view past its end.
    """
    # Far fewer lines than Emoji's names take whole, and far more than a drawer shows.
    whole_least = 100
    # Lines down the run the keys leave it at: twice past the view's 35 at rest, far short of
    # the end.
    deep = 80
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    shell.type_in("desktop", selecting="Desktop")
    shell.step_in()
    shell.down_to("Emoji")
    shell.launcher_until(
        lambda shown: shown["drawer"]["title"] == "Emoji" and shown["drawer"]["more"]
    )
    rest = shell.showing()["drawer"]["bodyLines"]
    assert rest < whole_least, f"at rest, {rest} of Emoji's lines were laid, not its view's"
    shell.normal()
    going_in = shell.mark()
    # Ctrl+W then l in one batch, as he types it, so the chord never waits on its second key.
    shell.as_account("wtype -M ctrl -k w -m ctrl -k l")
    # The rest is laid a part a turn with no key to ask it, until it's whole.
    shell.launcher_until(
        lambda shown: (
            (shown["drawer"]["cursor"] or {}).get("lines", 0) > whole_least
            and shown["drawer"]["laying"] is False
        )
    )
    # The cursor counts its lines from one, so it's told by where it began.
    entered = shell.showing()["drawer"]
    began, whole = entered["cursor"]["line"], entered["bodyLines"]
    whole_lines = entered["cursor"]["lines"]
    # The frame the keys came in laid only what shows, the rest after it, as counted at its
    # swap: Emoji's whole run laid first held that frame back a median 558 ms in the drawer
    # lab (2026-11-19).
    laid = laid_going_in(shell, going_in, "Emoji")
    assert laid < whole_least, f"going in laid {laid} lines before its frame, not its view's"
    shell.send_chars("j" * deep)
    shell.launcher_until(
        lambda shown: (shown["drawer"]["cursor"] or {}).get("line") == began + deep
    )
    shell.as_account("wtype -M ctrl -k w -m ctrl -k h")
    shell.launcher_until(lambda shown: not shown["drawer"]["keys"])
    drawer = shell.showing()["drawer"]
    assert drawer["scrolled"] > 0, f"leaving {deep} lines down didn't keep the view: {drawer}"
    # The view still shows the lines the cursor left, so they're laid; `more` alone can't
    # tell, since it weighs the run against the view at rest.
    assert drawer["bodyLines"] > deep, (
        f"left {deep} lines down, only {drawer['bodyLines']} were laid: {drawer}"
    )
    # Nor at rest again is it laid whole, which every step past it would pay to take down.
    assert drawer["bodyLines"] < whole, f"left, all {whole} lines stayed laid: {drawer}"
    # The foot counts what's below where the view was left, so it still says there's more.
    assert drawer["moreSays"], f"left {deep} lines down, the foot said no more: {drawer}"
    # G typed with the chord, before the rest is laid after its frame, still goes to the run's
    # end rather than the end of what was laid first, which the rest laid after would leave
    # the cursor far above. Both are held to the whole run's count, since what's laid first
    # this deep is itself past whole_least. G lands each time on the end of what's laid, the
    # run's end as far as it's laid, so this is also the case for go()'s showing or laying more
    # and moving again, whose edges and steps are cased in Qt's engine (DrawerRun.walk).
    shell.as_account("wtype -M ctrl -k w -m ctrl -k l G")
    shell.launcher_until(
        lambda shown: (
            (shown["drawer"]["cursor"] or {}).get("lines", 0) >= whole_lines
            and shown["drawer"]["cursor"]["line"] >= whole_lines - 1
        )
    )
    searched_mid_laying(shell, str(entered["bodyDrawn"]), whole_least)
    back_from_the_key_list(shell, whole_least, whole)
    # close_launcher can't close from inside the drawer's keys, so they're left first.
    shell.as_account("wtype -M ctrl -k w -m ctrl -k h")
    shell.launcher_until(lambda shown: not shown["drawer"]["keys"])
    shell.close_launcher()


def searched_mid_laying(shell: Shell, drawn: str, whole_least: int) -> None:
    """Search, typed with the chord from the run's top, for a name its first part doesn't hold.

    The run's rest is laid a part a turn after going in, so a search typed before the last
    part lays it all first, as it's typed, rather than search only what's laid: it counts
    what it finds before Return, which would lay the rest itself.
    """
    # The last name of two words or more, made only of letters and spaces, which the search
    # reads as typed, and found nowhere else, so the search lands on it alone: its space is
    # a no-break one in the run, which a space typed finds (Search.js).
    plain = drawn.replace("\u00a0", " ")
    names = re.split(r" \u00b7 |\n", plain)
    name = next(
        (
            name
            for name in reversed(names)
            if re.fullmatch(r"[A-Za-z]+ [A-Za-z ]+", name) and plain.count(name) == 1
        ),
        None,
    )
    assert name, f"no name of two plain words is found once in the run's {len(names)}"
    # Left at the run's top, so it's laid short at rest and the keys lay it again.
    shell.send_chars("gg")
    shell.as_account("wtype -M ctrl -k w -m ctrl -k h")
    shell.launcher_until(lambda shown: not shown["drawer"]["keys"])
    going_in = shell.mark()
    shell.as_account(f"wtype -M ctrl -k w -m ctrl -k l {shlex.quote('/' + name)}")
    # The first the launcher shows of the whole name typed, which has its count already, and
    # whether the run's rest was still being laid as the search was first shown: the search
    # lays it as it's typed, so it's done by then, and a part a turn after the frame would
    # still be laying it, some 46 turns for Emoji's, which then finds the name without it.
    # Its reports are read from the mark before the name went in, since wtype has typed it
    # all by the time it returns, and a wait marked then would miss the first.
    typed: list[object] = []
    laying: list[bool] = []

    def whole_name_typed(shown: Json) -> bool:
        search = shown["drawer"]["search"]
        if search["typing"] and not laying:
            laying.append(shown["drawer"]["laying"])
        if search["typing"] != name:
            return False
        typed.append(search["found"])
        return True

    shell.reported("showing ", lambda said: whole_name_typed(json.loads(said)), going_in)
    assert laying[0] is False, f"/{name} typed mid-laying left the run's rest to be laid"
    assert typed[0] is not None, f"/{name} typed mid-laying found nothing before Return"
    laid_going_in(shell, going_in, "Emoji")
    shell.as_account("wtype -k Return")
    # It lands on the name, far past the first part, rather than say it's found nowhere.
    shell.launcher_until(
        lambda shown: (shown["drawer"]["cursor"] or {}).get("line", 0) > whole_least
    )


def back_from_the_key_list(shell: Shell, whole_least: int, whole: int) -> None:
    """Go back to Emoji's drawer from its key list, and check its frame lays only what shows.

    The drawer shown anew lays its run as going into it does, a part a turn after its first,
    rather than all 1,833 lines before it.
    """
    to_keys = shell.mark()
    shell.as_account("wtype -k F1")
    shell.launcher_until(lambda shown: shown["drawer"]["title"] == "The Drawer's Keys")
    # The key list's own going-in is said once niri presents it, after its title shows, and
    # names its page, so going back reads only Emoji's.
    laid_going_in(shell, to_keys, "The Drawer's Keys")
    going_back = shell.mark()
    shell.as_account("wtype -k F1")
    laid = laid_going_in(shell, going_back, "Emoji")
    assert laid < whole_least, f"going back laid {laid} lines before its frame, not its view's"
    shell.launcher_until(
        lambda shown: (
            shown["drawer"]["title"] == "Emoji"
            and shown["drawer"]["laying"] is False
            and shown["drawer"]["bodyLines"] == whole
        )
    )


def test_a_drawer_stepped_off_a_long_run_measures_none_of_it(shell: Shell) -> None:
    """The row either side of Emoji in Desktop shows its drawer measuring none of Emoji's names.

    A drawer going from Emoji's run to a short one read its words and whether they were a run
    apart, so for a moment it held a run no longer, with Emoji's 1,914 names still its words,
    and laid and measured them all as plain words: 7,596 widths and 134 to 228 ms on the row
    after Emoji whichever way the drawer lab walked Desktop (2026-11-19), where other drawers
    took 9 to 16.
    """
    # Far more widths than a short drawer asks, and far fewer than Emoji's names take.
    most = 1000
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    shell.type_in("desktop", selecting="Desktop")
    shell.step_in()
    rows = cast("list[str]", shell.showing()["rows"])
    at = rows.index("Emoji")
    assert 0 < at < len(rows) - 1, f"Emoji needs a row either side of it in Desktop: {rows}"
    asked: dict[str, int] = {}
    for beside in (rows[at + 1], rows[at - 1]):
        shell.down_to("Emoji")
        shell.launcher_until(
            lambda shown: drawer_on_selected(shown) and shown["drawer"]["title"] == "Emoji"
        )
        before = int(laid_out(shell)["widthsAsked"])
        shell.down_to(beside)
        shell.launcher_until(functools.partial(drawer_moved, "Emoji"))
        asked[beside] = int(laid_out(shell)["widthsAsked"]) - before
    shell.close_launcher()
    assert all(count < most for count in asked.values()), (
        f"widths measured stepping off Emoji to each row beside it: {asked}"
    )


def openings_kept() -> list[Json]:
    """Give every opening the launcher's trials have kept, in no order."""
    state = Path(os.environ.get("XDG_STATE_HOME") or Path.home() / ".local/state")
    lines = [
        line
        for path in state.glob("quickshell/by-shell/*/launcher-trials.jsonl")
        for line in path.read_text().splitlines()
        if line.strip()
    ]
    return [cast("Json", json.loads(line)) for line in lines]


def now_ms() -> int:
    """Give the wall clock in milliseconds, as an opening's start is kept."""
    return time.time_ns() // 1_000_000


def opening_kept_since(since: int) -> Json:
    """Give the first opening begun at or after `since` (ms), once the shell has kept it.

    An opening is told by when it began rather than by how many lines there are, since the
    test before's line may still be on its way through the pipe.
    """
    deadline = time.monotonic() + launcher_on_niri.WAITING.total_seconds()
    # The shell adds the opening's line through a pipe once it closes, so it's read again until
    # it's there, a poll's length apart.
    while True:
        begun = [kept for kept in openings_kept() if cast("int", kept.get("at", 0)) >= since]
        if begun:
            return min(begun, key=lambda kept: cast("int", kept.get("at", 0)))
        assert time.monotonic() < deadline, "the opening was never kept"
        time.sleep(launcher_on_niri.POLL)


def test_return_on_an_answer_is_kept_as_a_pick_of_its_kind(shell: Shell) -> None:
    """An opening ended by Return on a sum keeps it as a pick of a sum, not a miss.

    An answer's key is empty, so every Return on Ask, a sum or Memory was kept as a miss, and
    what the trials said of them was wrong (1JQPPC, 2026-11-11). Each key typed is kept with
    it too, as its milliseconds from its press to the screen, for health to pool a day's keys
    for their 99th (1JQPPC, 2026-11-12). An opening that misses keeps each search given up on,
    so what wasn't found can be read: two of Joel's gave up after 51 and 62 s holding only counts
    (1JQPPC, 2026-11-12, 77f84121); a pick keeps none.
    """
    since = now_ms()
    copied(shell, "6*7", "42")
    last = opening_kept_since(since)
    assert last.get("picked") == "answer:sum", last
    assert last.get("missed") is False, last
    presented = cast("list[int]", last.get("presented", []))
    assert len(presented) + int(last.get("unpresented", 0)) >= len("6*7"), last
    assert presented, last
    assert all(isinstance(ms, int) and ms >= 0 for ms in presented), last
    assert last.get("sought") == [], last
    since = now_ms()
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    shell.type_in("zqxjv")
    for _ in "zqxjv":
        shell.send_key("backspace")
    shell.launcher_until(lambda shown: shown["typed"] == "")
    shell.type_in("qqwy")
    shell.close_launcher()
    missed = opening_kept_since(since)
    assert missed.get("missed") is True, missed
    assert missed.get("sought") == ["zqxjv", "qqwy"], missed


def test_a_place_stepped_into_from_whats_typed_is_kept_as_what_it_meant(shell: Shell) -> None:
    """Return into For You from "for you" keeps For You as a pick of those words, once each.

    Only doing something learned what was meant, so a place stepped into and left never came
    first for its words, however often (VKCF3Z, 2026-11-12); modules/quickshell/tests/ranking.qml
    holds the rest. The pick is learned once the step's frame is drawn, so the letters typed inside
    before it are never what it's learned for, and one closed before it's drawn still learns it.
    """
    state = Path(os.environ.get("XDG_STATE_HOME") or Path.home() / ".local/state")

    def picked() -> dict[str, Json]:
        said: dict[str, Json] = {}
        for path in state.glob("quickshell/by-shell/*/launcher-picks.json"):
            if path.read_text().strip():
                said.update(cast("dict[str, Json]", json.loads(path.read_text())))
        return said

    def times() -> int:
        answers = cast("dict[str, dict[str, int]]", picked().get("for you", {}))
        return sum(one["c"] for one in answers.values())

    def kept(at_least: int) -> int:
        deadline = time.monotonic() + 5
        while times() < at_least and time.monotonic() < deadline:
            time.sleep(launcher_on_niri.POLL)
        return times()

    before = times()
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    # Letters typed inside in the step's own turn, before its frame.
    typed_at_once(shell, "'for you' -k Return 'zq'")
    shell.launcher_until(lambda shown: shown["path"][-1:] == ["For You"])
    assert kept(before + 1) == before + 1, f"stepping in kept {times() - before}: {picked()}"
    assert "zq" not in picked(), f"the letters typed inside were learned: {picked()}"
    shell.close_launcher()
    # Closed by :q in the step's own turn, before its frame.
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    typed_at_once(shell, "'for you' -k Return -k Escape ':q' -k Return")
    shell.launcher_until(lambda shown: not shown["open"])
    assert kept(before + 2) == before + 2, f"closed at once, kept {times() - before}: {picked()}"


def test_the_date_is_copied_and_the_power_acts_arm_before_they_act(shell: Shell) -> None:
    """The date is copied, the power acts arm first, and a bind's words find its action."""
    # The date in full, as the clock has it; the day may turn while it's
    # asked, so either will do.
    before = now("+%A %-d %B %Y")
    assert yanked(shell, "date") in [before, now("+%A %-d %B %Y")]
    # The power actions answer to their names, and only those; a first
    # Return arms each in place, its row and foot saying what a second
    # does (VKCF3Z, 2026-11-12), and it's closed armed, never done.
    for typed, action, arms in [
        ("power off", "Power Off", "powers off"),
        ("reboot", "Reboot", "reboots"),
        ("log out", "Log Out", "logs out"),
    ]:
        shell.answer(typed)
        # Each wait ends before the loop moves on, so it reads this pass's action.
        shell.launcher_until(
            lambda shown: (
                (shown["selected"] or {}).get("name") == action  # noqa: B023
                and shown["kind"] == "session"
            )
        )
        said = f"again {arms}"
        shell.send_key("ret")
        shell.launcher_until(
            lambda shown: (
                shown["returned"].get("act") == "armed"
                and shown["held"] == []
                and shown["armed"] == said  # noqa: B023
                and f"\u21b5 {said}" in (shown["selected"] or {}).get("told", [])  # noqa: B023
            )
        )
        shell.close_launcher()
    # Any other key lets it go, so a Return after it arms again rather
    # than doing it; a second Return with no key between does it,
    # Reboot's systemctl finding no machine to restart here.
    shell.answer("reboot")
    shell.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Reboot")
    shell.send_key("ret")
    shell.launcher_until(lambda shown: shown["armed"] == "again reboots")
    shell.send_key("left")
    shell.launcher_until(lambda shown: shown["armed"] == "")
    # Past the key language's wait, so this Return isn't the last one doubled.
    time.sleep(PAUSE)
    shell.send_key("ret")
    shell.launcher_until(
        lambda shown: shown["returned"].get("act") == "armed" and shown["armed"] != ""
    )
    time.sleep(PAUSE)
    shell.send_key("ret")
    shell.launcher_until(lambda shown: shown["returned"].get("act") == "ran" and not shown["open"])
    # The : line arms the row it stands on, as its Return does, rather
    # than asking y/n (VKCF3Z, 2026-11-12), and the next Return does it.
    shell.answer("reboot")
    shell.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Reboot")
    shell.normal()
    shell.send_chars(":reboot")
    shell.launcher_until(lambda shown: (shown["command"] or {}).get("line") == "reboot")
    shell.send_key("ret")
    shell.launcher_until(
        lambda shown: (
            shown["command"] is None
            and shown["question"] == ""
            and shown["armed"] == "again reboots"
            and "\u21b5 again reboots" in (shown["selected"] or {}).get("told", [])
        )
    )
    time.sleep(PAUSE)
    shell.send_key("ret")
    shell.launcher_until(lambda shown: shown["returned"].get("act") == "ran" and not shown["open"])
    # A line written and left goes with the launcher, so the next opening
    # is on the find, not on a line still held.
    shell.answer("log out")
    shell.normal()
    shell.send_chars(":logout")
    shell.launcher_until(lambda shown: (shown["command"] or {}).get("line") == "logout")
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_until(lambda shown: not shown["open"])
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    shell.launcher_until(lambda shown: shown["command"] is None and shown["mode"] == "find")
    shell.close_launcher()
    # A key's action, by the other words its bind lists.
    shell.answer("fullscreen")
    shell.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name")
            == "Fill the screen with the window, or put it back"
            and shown["kind"] == "shortcut"
        )
    )
    # Escape at rest lets the key list go, as it lets go what's begun, and closes nothing.
    shell.normal()
    shell.send_chars(" ?")
    shell.launcher_until(lambda shown: shown["listing"] is True)
    shell.send_key("esc")
    shell.launcher_until(lambda shown: shown["open"] and shown["listing"] is False)
    space_waits_for_its_next_key(shell)
    shell.close_launcher()


# Twice the key language's timeoutlen (modules/keys/language.toml's wait), as long a pause as
# Joel's between Space and the Shift its ? takes.
PAUSE = 0.6


def space_waits_for_its_next_key(shell: Shell) -> None:
    """Press Space, pause past timeoutlen, then ?, from normal and from the find: the key list.

    His Neovim's which-key holds a begun Space for its next key however long he pauses, so ?
    after it lists the keys rather than opening the find upward (Joel, 2026-11-05: "space ?
    does not work, it goes into reverse search mode").
    """
    for start in (shell.normal, shell.on_find):
        start()
        # The find types a Space, so normal is Escape away, as from Vim's / line.
        shell.normal()
        shell.send_chars(" ")
        shell.launcher_until(lambda shown: shown["begun"] == "Space")
        # The pause is what's tested, so it's slept, not waited on.
        time.sleep(PAUSE)
        shell.send_chars("?")
        shell.launcher_until(lambda shown: shown["listing"] is True)
        shown = shell.showing()
        assert (shown["mode"], shown["begun"]) == ("normal", ""), shown
        shell.send_key("esc")
        shell.launcher_until(lambda shown: shown["open"] and shown["listing"] is False)


def test_the_power_acts_are_found_by_their_names_and_by_the_names_they_had(shell: Shell) -> None:
    """The power acts are found by their names, and by the names they had."""
    # Short names, each found by the start Joel types; closed by :q, never
    # Return, since Return would do them.
    for typed, name, verb in [
        ("sle", "Sleep", "sleep"),
        ("keep", "Keep Awake", "on"),
        ("reb", "Reboot", "reboot"),
        ("do not", "Do Not Disturb", "start"),
    ]:
        shell.answer(typed)
        # The wait ends before the loop moves on, so it reads this pass's name.
        shell.launcher_until(
            lambda shown: (
                (shown["selected"] or {}).get("name") == name  # noqa: B023
                and shown["selected"]["verb"] == verb  # noqa: B023
            )
        )
        shell.close_launcher()
    # A rename loses nothing: the old name still finds Sleep, never shown,
    # and says once what it's called now.
    shell.answer("suspend")
    shell.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name") == "Sleep"
            and shown["footer"] == "Once called Suspend."
            and "Suspend" not in shown["rows"]
        )
    )
    shell.close_launcher()


def test_keep_awake_is_one_row_beside_power_found_by_awake_and_sleep(shell: Shell) -> None:
    """Keep Awake is one row of one name at Machine's own level, beside Power, not in it.

    Joel looked for it under Power and gave up, as Power called it Let It Sleep while Now
    called it Keep Awake (VKCF3Z, 99c7bc98): it keeps its name whatever its state, its side
    says off or on, and "awake" and "sleep" each find it.
    """
    machine = shown_to_terminal(shell, "machine")
    names = [row["name"] for row in machine["rows"]]
    assert names[names.index("Power") + 1 :][:1] == ["Keep Awake"], names
    awake = next(row for row in machine["rows"] if row["name"] == "Keep Awake")
    assert awake["side"] == "off", awake
    power = shown_to_terminal(shell, "machine", "power")
    assert "Keep Awake" not in [row["name"] for row in power["rows"]], power["rows"]
    for typed in ("awake", "sleep"):
        shell.answer(typed)
        shell.launcher_until(
            # The wait ends before the loop moves on, so it reads this pass's words.
            lambda shown: shown["typed"] == typed and "Keep Awake" in shown["rows"]  # noqa: B023
        )
        shell.close_launcher()


def test_power_profiles_drawer_still_unread_has_a_terminal_ask_again(shell: Shell) -> None:
    """Power Profile's drawer, not yet read, says so, so a terminal asks again for it.

    Its drawer reads sysfs a turn after it's first asked for and is none until then
    (Session.qml's profileDrawer), and the launcher forgets that reading as it shuts. A
    terminal's first ask after that met no drawer and no word that one was coming, so a
    walk of the launcher took it as a row with no drawer (Y2YBTS, 301f106d). The sandbox has
    no profile files to read, so the drawer itself never comes here; on a laptop it's what
    the next ask is shown.
    """
    steps = ("machine", "power", "power profile")
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    # Asked while open, so it's read and then forgotten as the launcher shuts.
    shown_to_terminal(shell, *steps)
    shell.close_launcher()
    first = shown_to_terminal(shell, *steps)
    drawn = "drawer" in (first["row"] or {})
    assert drawn or first["reading"], f"an unread drawer was said as none: {first}"


def test_do_not_disturb_says_until_when_in_its_drawer(shell: Shell) -> None:
    """Do Not Disturb, off, has a drawer saying until when Return holds notices.

    Joel wants every row to say a little more beside it (2026-11-11), and Desktop's Do Not
    Disturb had none; what each fact says is modules/quickshell/tests/piece-drawers.qml's, and this
    is its one path through the launcher.
    """
    shell.answer("do not")
    shell.launcher_until(
        lambda shown: (
            drawer_on_selected(shown)
            and shown["drawer"]["title"] == "Do Not Disturb"
            and shown["drawer"]["facts"] == ["For an Hour", "Sharing the Screen"]
        )
    )
    shell.close_launcher()


def test_do_not_disturbs_other_act_is_done_by_its_own_key(shell: Shell) -> None:
    """Do Not Disturb holds notices for an hour on Return, and until they're ended on e.

    Its acts lie on its row, with nothing under them, so it opens no step: Return does the
    first and each other is pressed by its own key, which the foot names (VKCF3Z,
    2026-11-12: "Return does For an Hour and Until It's Ended takes its key").
    """

    def selected(shown: Json) -> Json:
        return cast("Json", shown["selected"] or {})

    shell.answer("do not disturb")
    shell.launcher_until(lambda shown: selected(shown).get("name") == "Do Not Disturb")
    shell.normal()
    told = selected(shell.showing())["told"]
    assert "e until it's ended" in told, told
    shell.send_chars("e")
    shell.launcher_until(lambda shown: selected(shown).get("side") == "until it's ended")
    held = shell.showing()["held"]
    assert held == [], f"its key stepped into {held}"
    # On, its Return ends it, so the next test finds notices shown.
    shell.send_key("ret")
    shell.launcher_until(lambda shown: selected(shown).get("side") == "for an hour")
    shell.close_launcher()


def test_ctrl_bracketright_takes_do_not_disturb_where_it_lives_and_never_in(
    shell: Shell,
) -> None:
    """Ctrl+] on Do Not Disturb, found from the top, goes to where it lives, never into it.

    Its acts lie on its row with nothing under them (Joel, 2026-11-11: "if they have nothing
    under them, we shouldn't open a submenu for them"), so Ctrl+] goes to its place with it
    selected, as with any row holding nothing, and there it says it goes nowhere more
    (RCTRF8's walk, 68c21332).
    """

    def selected(shown: Json) -> Json:
        return cast("Json", shown["selected"] or {})

    shell.answer("do not disturb")
    shell.launcher_until(lambda shown: selected(shown).get("name") == "Do Not Disturb")
    shell.normal()
    shell.step_in("ctrl-bracketright")
    shown = shell.showing()
    assert "Do Not Disturb" not in shown["held"], f"Ctrl+] went into it: {shown['held']}"
    shell.launcher_until(lambda shown: selected(shown).get("name") == "Do Not Disturb")
    held = shell.showing()["held"]
    shell.send_key("ctrl-bracketright")
    shell.launcher_until(lambda shown: "not here" in str(shown["saying"]))
    assert shell.showing()["held"] == held, "Ctrl+] in its place went somewhere"
    shell.close_launcher()


def test_an_answer_asked_for_holds_its_row_from_the_keystroke(shell: Shell) -> None:
    """An answer asked for holds its row from the keystroke."""
    typed = "weather lima"
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    since = shell.mark()
    shell.send_chars(typed)
    seen: list[Json] = []

    def settled(said: str) -> bool:
        shown = json.loads(said)
        if shown["typed"] != typed:
            return False
        seen.append(shown)
        return bool((shown["selected"] or {}).get("settled"))

    shell.reported("showing ", settled, since)
    # Each time it's shown whole, still asking or answered, the answer is
    # the first row and the rows below it are the same.
    firsts = [shown["things"][0]["is"] if shown["things"] else "" for shown in seen]
    assert set(firsts) == {"answer"}, f"the answer's row came and went: {firsts}"
    below = {tuple(shown["rows"][1:]) for shown in seen}
    assert len(below) == 1, f"the rows below the answer moved: {below}"
    shell.close_launcher()


# How long the closed launcher is watched across a minute's turn, from 45 s past it, and how
# long the clock may take to reach that.
QUIET = timedelta(seconds=30)
FROM_SECOND = 45
# The seconds past a minute the case closes the launcher between, so the closing's own making
# is done before FROM_SECOND and its freshening a minute on falls after the span.
CLOSING = range(30, 40)
TURNING = timedelta(seconds=130)


def until_second(seconds: range, turning: float) -> None:
    """Wait until the clock is within these seconds past a minute."""
    while int(now("+%S")) not in seconds:
        assert time.monotonic() < turning, f"the clock never reached {seconds} s past a minute"
        time.sleep(0.5)


def test_the_closed_launcher_draws_nothing_as_the_time_it_would_show_turns(shell: Shell) -> None:
    """The closed launcher draws nothing as a minute turns, though its foot says the time.

    Qt draws a window for any change inside it, clear or hidden alike, so the closed
    launcher drew a frame nobody saw each time its foot's facts changed: the time each
    minute, the lag, the volume (2026-11-01). The sandbox has no volume to change, so the
    minute and the lag are what's watched here.

    A closing draws the closed launcher too: its places' spots made half a second after it
    (Launcher.qml's spotsLater) and their figures a minute after (spotsFreshened). The span
    began whenever the last case left it, so it caught one or the other about half the time
    (2026-11-10: 1 and 5 frames, failing two gates). Committing nothing while shut, as the
    kept shut surface does, ends those; until then this case is the minute's turn alone, so
    it makes its own closing first, at a second it chooses, and counts once that settles.
    """
    frames = f"qs -p {shell.folder} ipc call frames"
    turning = time.monotonic() + TURNING.total_seconds()
    until_second(CLOSING, turning)
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    closing = shell.mark()
    shell.close_launcher()
    shell.reported("spots made while closed", lambda _: True, closing)
    until_second(range(FROM_SECOND, 60), turning)
    shell.as_account(f"{frames} start")
    # The span measured, across the turn of the minute, not a wait for something to happen.
    time.sleep(QUIET.total_seconds())
    drawn = json.loads(shell.as_account(f"{frames} said"))
    assert "launcher" not in drawn, f"the closed launcher drew as the minute turned: {drawn}"
    # And it says the time it's now as it opens, in the opening's own report, which is made
    # before anything else follows the opening. The minute has 45 s left to turn in.
    turned = now("+%H:%M")
    since = shell.mark()
    shell.as_account(shell.ipc("toggle"))
    shell.reported("stands ", lambda rest: turned in rest, since)
    shell.launcher_ready()
    shell.close_launcher()


def test_the_launcher_left_still_draws_nothing(shell: Shell) -> None:
    """The launcher left open and still draws nothing, its caret steady rather than blinking."""
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    shell.send_chars("sou")
    shell.launcher_until(lambda shown: shown["typed"] == "sou")
    # A blinking caret would draw six times in the three seconds.
    drawn = shell.drawn_held_still()
    still = shell.frames()
    # A letter draws, so the count sees frames at all.
    shell.send_chars("n")
    shell.launcher_until(lambda shown: shown["typed"] == "soun")
    counting = time.monotonic() + 10
    while shell.frames() <= still:
        assert time.monotonic() < counting, "a letter drew no frame, so none are counted"
        time.sleep(0.05)
    shell.close_launcher()
    assert drawn <= 1, f"the launcher drew {drawn} frames in three seconds left still"


# How long a window opening and closing is given to reach the shut shell, which made its rows
# within a few milliseconds of each event when it made them shut.
SETTLING = timedelta(seconds=2)


def test_the_shut_launcher_makes_no_window_rows_as_windows_change(shell: Shell) -> None:
    """The shut launcher makes no window's rows as windows open and close, and finds them open.

    With the launcher shut, every event niri sent made Windows' and Capture's rows again over
    all of Joel's windows: 657 and 287 rows in four minutes, for nobody (2026-11-10). Opened,
    the window that came meanwhile is found by its title, so the rows are made as it opens.
    Shut, a pick still knows that window, since its rows as last made don't hold it, and a
    pick forgets what it knows nothing of. A workspace switched to is the one exception, made
    shut in one batch, as the next case says.
    """

    def window_rows() -> dict[str, int]:
        rows = json.loads(shell.as_account(shell.ipc("made")))["rows"]
        return {name: rows.get(name, 0) for name in ("windows", "capture")}

    before = window_rows()
    window = shell.opened("Godwit Notes")
    gone = shell.opened("Whimbrel Notes")
    shell.close(gone)
    # The span measured, for the events to reach the shell, not a wait for something to happen.
    time.sleep(SETTLING.total_seconds())
    windows = json.loads(shell.as_account(shell.ipc("known", "window")))
    taking = json.loads(shell.as_account(shell.ipc("known", "capture")))
    after = window_rows()
    try:
        shell.as_account(shell.ipc("toggle"))
        shell.launcher_ready()
        shell.send_chars("godwit")
        shell.launcher_until(lambda shown: "Godwit Notes" in shown["rows"])
        shell.close_launcher()
    finally:
        shell.close(window)
    made = {name: after[name] - before[name] for name in after}
    assert not any(made.values()), f"the shut launcher made window rows: {made}"
    assert any(key.endswith(":Godwit Notes") for key in windows), (
        f"a pick made shut knows no window come since: {windows}"
    )
    assert "capture:window" in taking, f"a pick made shut knows no window to take: {taking}"


# How long after the last of a run of workspace switches the shut shell remakes the rows whose
# words they turned, its Windows' rewordingLater.
REWORDING = timedelta(seconds=1)


def test_the_shut_launcher_rewords_the_windows_a_switch_moved(shell: Shell) -> None:
    """The shut launcher remakes the rows a workspace switch reworded, so an opening makes none.

    Joel's openings remade 29 to 76 Windows rows each (2026-11-11), most of them windows whose
    workspace he had switched away from while it was shut, so each opening's first frame said
    where they had been. Shut, the rows already made whose words turned are made again in one
    batch a second after the switches settle (Y2YBTS, 2026-11-11 [c4acd8f5]); an opening then
    makes none, and says where each window is now.
    """

    def windows_made() -> int:
        return int(json.loads(shell.as_account(shell.ipc("made")))["rows"].get("windows", 0))

    def side_of(title: str) -> str:
        path = shlex.quote("\u203adesktop\u203awindows")
        asked = f"qs -p {shell.folder} ipc call -- launcher show {path}"
        rows = json.loads(shell.as_account(asked)).get("rows", [])
        return str(next(row.get("side", "") for row in rows if row["name"] == title))

    # The window Joel's in is never listed, so the one read is opened first.
    window = shell.opened("Curlew Notes")
    over = shell.opened("Dunlin Notes")
    try:
        shell.as_account(shell.ipc("toggle"))
        shell.launcher_ready()
        here = side_of("Curlew Notes")
        shell.close_launcher()
        before = windows_made()
        shell.as_account("niri msg action focus-workspace 2")
        # The span measured, for the switch to reach the shell and its batch to follow, not a
        # wait for something to happen.
        time.sleep((SETTLING + REWORDING).total_seconds())
        shut = windows_made()
        shell.as_account(shell.ipc("toggle"))
        shell.launcher_ready()
        opened = windows_made()
        there = side_of("Curlew Notes")
        shell.close_launcher()
    finally:
        shell.as_account("niri msg action focus-workspace 1")
        shell.close(window)
        shell.close(over)
    assert shut > before, f"the shut launcher remade no window a switch moved: {before}"
    assert opened == shut, f"the opening made {opened - shut} window rows after the switch"
    assert "workspace 1" in there, f"a window left behind said {there!r}, here {here!r}"


RESIZES = 20


def sits_on(window: Json, said: str) -> bool:
    """Say whether the margin cat's report has it sitting still over this window."""
    at = launcher_on_niri.placed(said)
    return at is not None and at.get("window") == window["id"] and not at.get("walk")


def test_the_shut_shell_sleeps_as_a_window_is_resized(shell: Shell) -> None:
    """With the launcher shut, a window beside the keys' resized over and over is heard of not once.

    The shell's niri heard every event niri sent, each tile resized or scrolled among them, and
    Joel's shut shell woke for each (2026-11-10), though only a notice drawn beside its window
    reads a tile. It now hears the windows daemon, told only the fields it reads, and asks for
    every tile only while a notice is drawn. The margin's cat sits over the window with the
    keys, so that one tile is told, and a resize of it is heard; it watched every tile, and the
    shell heard each of 20 resizes again (2026-11-12). So the window resized is one beside it.
    On its way over to that window it wants every tile, the gap it leaps among them, so the
    shell at rest is measured once the cat sits there.
    What the shell reads of niri's windows is counted line by line, the windows' opening
    proving the count runs; a wake counted from /proc took in whatever else the shell did then,
    such as the work a closing leaves for a moment after.
    """

    def heard() -> dict[str, int]:
        return cast("dict[str, int]", json.loads(shell.as_account(shell.ipc("linesHeard"))))

    before = heard()
    window = shell.opened("Dunlin Notes")
    # Opened right of the one with the keys, so resizing it moves nothing left of it.
    beside = shell.opened("Godwit Notes")
    try:
        since = shell.mark()
        shell.as_account(f"niri msg action focus-window --id {window['id']}")
        shell.reported("sits cat ", functools.partial(sits_on, window), since)
        # The span measured, for the windows' opening to reach the shell, not a wait for it.
        time.sleep(SETTLING.total_seconds())
        opening = heard()
        for one in range(RESIZES):
            change = f"{'+' if one % 2 else '-'}10%"
            shell.as_account(f"niri msg action set-window-width --id {beside['id']} {change}")
        # The span measured, for the resizes' events to reach the shell, not a wait for them.
        time.sleep(SETTLING.total_seconds())
        resized = heard()
    finally:
        shell.close(beside)
        shell.close(window)
    assert opening["niri"] > before["niri"], (
        f"the shell heard nothing of a window opened: {before}, then {opening}"
    )
    moved = {name: resized[name] - opening.get(name, 0) for name in resized}
    assert not any(moved.values()), (
        f"the shut shell heard of {RESIZES} resizes, by lines read: {moved}"
    )


RETITLES = 10


def test_a_retitled_window_is_heard_of_alone_with_the_launcher_open(shell: Shell) -> None:
    """With the launcher open, a window retitled is the one window its rows look at again.

    Every event niri sent walked all of Joel's windows in Windows, whichever window it was of
    (2026-11-10). The windows daemon tells of only the windows a change moved, so ten retitles
    of one window among a dozen look at about ten records, where walking would look at well over
    a hundred.
    """
    retitling = Path(os.environ["XDG_RUNTIME_DIR"]) / "retitling"
    go = retitling.with_suffix(".go")
    # foot takes a title from what runs in it, once the test says to begin; a third of a second
    # apart, so each retitle reaches niri, and the shell, as an event of its own rather than
    # several folded into one frame's title.
    retitling.write_text(
        f"while [ ! -e {go} ]; do sleep 0.1; done\n"
        f"for i in $(seq {RETITLES}); do printf '\\033]2;Tern %s\\007' \"$i\"; sleep 0.3; done\n"
        "sleep 3600\n"
    )
    opened: list[Json] = []

    # A launcher that counts no records looked at says so, rather than stopping on a missing key:
    # one that walks every window at each event keeps no such count.
    def looked() -> int:
        made = json.loads(shell.as_account(shell.ipc("made")))
        counted = made.get("looked", {}).get("windows")
        assert counted is not None, "the launcher counts no records its windows looked at"
        return int(counted)

    try:
        opened.extend(shell.opened(f"Plover {n}") for n in range(1, 11))
        opened.append(shell.opened("Tern 0", running=f"sh {retitling}"))
        shell.as_account(shell.ipc("toggle"))
        shell.launcher_ready()
        shell.send_chars("tern")
        shell.launcher_until(lambda shown: "Tern 0" in shown["rows"])
        before = looked()
        go.touch()
        shell.launcher_until(lambda shown: f"Tern {RETITLES}" in shown["rows"])
        heard = looked() - before
        shell.close_launcher()
    finally:
        for window in opened:
            shell.close(window)
    assert heard <= 2 * RETITLES, f"{RETITLES} retitles of one window looked at {heard} records"


def typed_at_once(shell: Shell, typing: str) -> None:
    """Type with wtype while the shell is frozen, so every key reaches it in one turn of its loop.

    A program's keys come together, but a busy sandbox can space them out a search apart, which
    would leave nothing to show; held in the shell's socket until it goes on, they come at once.
    """
    with shell.frozen():
        shell.as_account(f"wtype {typing}")


def test_text_typed_at_once_is_searched_once(shell: Shell) -> None:
    """Text a program types at once is searched for its newest letters, not once a letter."""
    # Its first letter alone selects Windows, and the word a workspace, so a report said partway
    # through its search selects the wrong row.
    word = "workspace"
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    before = launcher_on_niri.searched(shell)
    since = shell.mark()
    typed_at_once(shell, word)
    seen: list[str] = []

    def whole(said: str) -> bool:
        shown = json.loads(said)
        if shown["typed"] != word:
            return False
        seen.append((shown["selected"] or {}).get("name", ""))
        return True

    shell.reported("showing ", whole, since)
    searches = launcher_on_niri.searched(shell) - before
    assert searches < len(word), f"{len(word)} letters typed at once took {searches} searches"
    # Said as a frame can draw it, the text comes with what it found, never partway through
    # the search, the letters there and the rows still those before them.
    settled = (shell.showing()["selected"] or {}).get("name", "")
    assert settled, f"{word!r} found nothing, so it can't tell a report said partway"
    shell.close_launcher()
    assert seen == [settled], f"the launcher said {word!r} selecting {seen}, not {settled!r}"


def test_return_typed_with_the_text_acts_on_what_it_found(shell: Shell) -> None:
    """Return typed at once with the text acts on the row the text found, never one before it."""
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    # Found before its letters, Return would step into the first row at home, Now.
    typed_at_once(shell, "'for you' -k Return")
    shell.launcher_until(lambda shown: shown["path"][-1:] == ["For You"])
    shell.close_launcher()


# How many letters a search gathers each place's rows from, so a notice is found at home.
GATHERED_FROM = 2


def test_a_key_lists_the_notices_it_finds_with_the_rest_of_its_rows(shell: Shell) -> None:
    """Each key's rows land at once, the notices the daemon finds among them.

    The notices daemon answers a search a moment after the key, so the key's list waits for it a
    little (Launcher.qml's answersWaitedFor): listed before it came, the notices' row would land
    a frame later and move the rows under it. A key the daemon answers past the wait says so.
    """
    # A word nothing else here is named by, so its row is among the first twenty `showing` says.
    with shell.told("shows card Zygodactyl"):
        shell.as_account("notify-send --expire-time=1000 'Zygodactyl' 'a notice to find'")
    shell.notices_gone()
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    since = shell.mark()
    shell.type_in("zygodactyl", selecting="zygodactyl")
    reports, _ = shell.said_since(since)
    excused = False
    listed: list[str] = []
    for said in reports:
        if said == "listed without answers":
            excused = True
        elif said.startswith("showing "):
            shown = json.loads(said[len("showing ") :])
            typed = shown["typed"]
            if len(typed) < GATHERED_FROM or typed in listed:
                continue
            listed.append(typed)
            found = excused or "Zygodactyl" in shown["rows"]
            assert found, f"{typed!r} first listed {shown['rows']}"
            excused = False
    shell.close_launcher()
    assert listed, f"no key was listed: {reports}"


def test_notices_answered_late_come_in_with_the_selection_on_its_row(shell: Shell) -> None:
    """A key the notices daemon answers past its wait lists without them, then takes them in.

    The journal says the key was listed without them and how late they came, and they come in
    with the selection on the row it was on, not at the place it was in.
    """
    with shell.told("shows card Windows of the hide"):
        shell.as_account("notify-send --expire-time=1000 'Windows of the hide' 'a late notice'")
    shell.notices_gone()
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    since = shell.mark()
    with launcher_on_niri.notices_stopped():
        shell.type_in("windows", selecting="windows")
        shell.reported("listed without answers", lambda rest: rest == "", since)
        shown = shell.showing()
        assert "Windows of the hide" not in shown["rows"], shown["rows"]
        selected = shown["selected"]["name"]
    shell.reported("answered late notices", lambda rest: rest == "", since)
    shell.launcher_until(lambda shown: "Windows of the hide" in shown["rows"])
    now = shell.showing()["selected"]["name"]
    shell.close_launcher()
    assert now == selected, f"the notices came in and the selection went from {selected} to {now}"


# How long a case holds the notices daemon after the launcher closed, in seconds, so a late
# answer to a key typed before the closing says at least this long after it.
HELD_PAST_CLOSING = 1.5
# A notices' answer come after its key was listed without it, with how late (Presented.js's
# answeredLateSaid).
NOTICES_LATE = re.compile(r"launcher: notices answered .+? (\d+) ms after its key")


def test_a_closing_ends_the_keys_its_answers_came_late_for(shell: Shell) -> None:
    """A key listed without the notices' answer is let go when the launcher closes.

    Its answer, come after the closing, is no key's being late: nobody waits on it. Kept, the
    mark is said late by the first unheld notices' answer. Here that is the stopped daemon's,
    as it resumes with the launcher shut, which is where this case goes red. The reopening
    types the words again, so its opening is read too. A notice brought in with the launcher
    open, which would say a kept mark however many hours on, isn't driven: no case here brings
    one in open, and waiting for its row timed out (2026-11-25).
    """
    with shell.told("shows card Windows of the past"):
        shell.as_account("notify-send --expire-time=1000 'Windows of the past' 'a late notice'")
    shell.notices_gone()
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    since = shell.mark()
    with launcher_on_niri.notices_stopped():
        shell.type_in("windows", selecting="windows")
        shell.reported("listed without answers", lambda rest: rest == "", since)
        closed = shell.mark()
        shell.close_launcher()
        time.sleep(HELD_PAST_CLOSING)
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    shell.type_in("windows", selecting="windows")
    shell.close_launcher()
    logged = launcher_on_niri.logged_since(shell, closed)
    lates = [int(late) for late in NOTICES_LATE.findall(logged)]
    held = [late for late in lates if late >= HELD_PAST_CLOSING * 1000]
    assert not held, f"the notices came late for a key typed before the closing: {held} ms"


def test_for_yous_filter_steps_into_a_kind_and_minus_comes_back_out(shell: Shell) -> None:
    """In For You, :filter look steps into its looks, the head naming it, and - comes back out.

    VKCF3Z ruled a filter a step in, never a state (2026-11-08), so what it keeps and the
    words it takes are tests/for-you-filter.qml's; this is the one path through the launcher.
    """
    state = Path(os.environ.get("XDG_STATE_HOME") or Path.home() / ".local/state")
    looks = state / "pi/agentic-harness.pi/looks"
    looks.mkdir(parents=True, exist_ok=True)
    look = looks / "filtered.json"
    # It came just now, since one three days old goes quiet (launcher/Looks.js).
    said = {"v": 1, "id": "filtered", "title": "Filtered Look", "target": "/etc/hostname",
            "kind": "file", "state": "waiting", "at": now("-u", "+%FT%TZ")}  # fmt: skip
    launcher_on_niri.laid_whole(look, json.dumps(said))
    try:
        shell.as_account(shell.ipc("toggle"))
        shell.launcher_ready()
        typed_at_once(shell, "'for you' -k Return")
        # The look blocks nothing and isn't due, so For You holds it in To Look At.
        shell.launcher_until(
            lambda shown: shown["path"][-1:] == ["For You"] and "To Look At" in shown["rows"]
        )
        shell.normal()
        shell.send_chars(":filter look")
        shell.launcher_until(lambda shown: (shown["command"] or {}).get("line") == "filter look")
        shell.send_key("ret")
        shell.launcher_until(
            lambda shown: (
                shown["path"][-2:] == ["For You", "To Look At"]
                and shown["rows"] == ["Filtered Look"]
            )
        )
        shell.send_chars("-")
        shell.launcher_until(lambda shown: shown["path"][-1:] == ["For You"])
        shell.close_launcher()
    finally:
        look.unlink()


def test_a_look_written_while_shut_is_shown_to_a_terminal(shell: Shell) -> None:
    """A look written while the launcher is shut is in For You's To Look At for `launcher show`.

    The looks were listed only while it was open, so `launcher show now "for you"` said no
    To Look At with 27 looks waiting (2RCBOJ, 2026-11-12), and each opening read every one.
    """
    state = Path(os.environ.get("XDG_STATE_HOME") or Path.home() / ".local/state")
    looks = state / "pi/agentic-harness.pi/looks"
    looks.mkdir(parents=True, exist_ok=True)
    look = looks / "written-shut.json"
    # It came just now, since one three days old goes quiet (launcher/Looks.js).
    said = {"v": 1, "id": "written-shut", "title": "Written Shut", "target": "/etc/hostname",
            "kind": "file", "state": "waiting", "at": now("-u", "+%FT%TZ")}  # fmt: skip
    assert not shell.showing()["open"], "the launcher was left open"
    launcher_on_niri.laid_whole(look, json.dumps(said))
    try:
        names = looks_listed(shell, "Written Shut")
        assert "Written Shut" in names, f"a look written while shut was never listed: {names}"
        assert not shell.showing()["open"], "showing To Look At opened the launcher"
    finally:
        look.unlink()


def looks_listed(shell: Shell, *titles: str) -> list[str]:
    """Give the names in To Look At once they hold these titles, or after 10 s, as they stand."""
    deadline = time.monotonic() + 10
    names: list[str] = []
    while not set(titles) <= set(names) and time.monotonic() < deadline:
        time.sleep(launcher_on_niri.POLL)
        # A place missed is answered with what it was missing among, rows left out.
        shown = shown_to_terminal(shell, "now", "for you", "to look at")
        names = [row["name"] for row in shown.get("rows", [])]
    return names


def watchers_held(shell: Shell) -> int:
    """Give how many inotify watchers the shell holds open, each a file of its own."""
    assert shell.process is not None, "the shell isn't started"
    held = 0
    for fd in Path(f"/proc/{shell.process.pid}/fd").iterdir():
        # One closed while they're counted holds nothing.
        with contextlib.suppress(FileNotFoundError):
            held += str(fd.readlink()) == "anon_inode:inotify"
    return held


LOOKS_HELD = 30


def test_the_shell_holds_no_file_for_each_look(shell: Shell) -> None:
    """The looks cost the shell no open file each, and a look written again is still heard.

    Each look's file had a watcher of its own: 406 on Joel's machine took the shell to 617 of
    the 1,024 files it may hold open, and a restart's burst went past them, so niri's next
    file was dropped and the shell lost its display (2026-11-12).
    """
    state = Path(os.environ.get("XDG_STATE_HOME") or Path.home() / ".local/state")
    looks = state / "pi/agentic-harness.pi/looks"
    looks.mkdir(parents=True, exist_ok=True)
    written = [looks / f"held{at:02d}.json" for at in range(LOOKS_HELD)]
    came = now("-u", "+%FT%TZ")

    def said(at: int, title: str) -> str:
        # It came just now, since one three days old goes quiet (launcher/Looks.js).
        return json.dumps({"v": 1, "id": f"held{at:02d}", "title": title, "kind": "file",
                           "target": "/etc/hostname", "state": "waiting", "at": came})  # fmt: skip

    before = watchers_held(shell)
    for at, look in enumerate(written):
        launcher_on_niri.laid_whole(look, said(at, f"Held Look {at}"))
    try:
        titles = [f"Held Look {at}" for at in range(LOOKS_HELD)]
        names = looks_listed(shell, *titles)
        assert set(titles) <= set(names), f"the looks were never all listed: {names}"
        grown = watchers_held(shell) - before
        assert grown < LOOKS_HELD // 2, f"the shell held a watcher for each look: {grown} more"
        launcher_on_niri.laid_whole(written[0], said(0, "Held Look Again"))
        names = looks_listed(shell, "Held Look Again")
        assert "Held Look Again" in names, f"a look written again was never heard: {names}"
    finally:
        for look in written:
            look.unlink()


def test_a_look_cleared_in_for_you_is_written_cleared(shell: Shell) -> None:
    """Clearing a look in For You's To Look At writes its file again, saying it was cleared.

    The to-do daemon writes it, asked to settle it (2026-11-13), so this is the one path
    showing the ask lands and the file is written whole.
    """
    state = Path(os.environ.get("XDG_STATE_HOME") or Path.home() / ".local/state")
    looks = state / "pi/agentic-harness.pi/looks"
    looks.mkdir(parents=True, exist_ok=True)
    look = looks / "cleared.json"
    # It came just now, since one three days old goes quiet (launcher/Looks.js).
    said = {"v": 1, "id": "cleared", "title": "Cleared Look", "target": "/etc/hostname",
            "kind": "file", "state": "waiting", "at": now("-u", "+%FT%TZ")}  # fmt: skip
    launcher_on_niri.laid_whole(look, json.dumps(said))
    try:
        shell.as_account(shell.ipc("toggle"))
        shell.launcher_ready()
        typed_at_once(shell, "'for you' -k Return")
        shell.launcher_until(
            lambda shown: shown["path"][-1:] == ["For You"] and "To Look At" in shown["rows"]
        )
        shell.down_to("To Look At")
        shell.step_in()
        shell.launcher_until(lambda shown: shown["path"][-1:] == ["To Look At"])
        shell.normal()
        shell.down_to("Cleared Look")
        # Clearing a look costs little, so dd does it at once, asking nothing (Joel,
        # 2026-11-13).
        shell.send_chars("dd")
        shell.launcher_until(lambda _shown: json.loads(look.read_text())["state"] == "cleared")
        written = json.loads(look.read_text())
        assert (written["id"], written["title"]) == ("cleared", "Cleared Look"), written
        shell.close_launcher()
    finally:
        look.unlink()


# How many looks wait in To Look At, enough that weighing them all again shows.
LOOKS_WAITING = 12
# How many rows an act may weigh again: the one it changed, and one more for the foot's.
WEIGHED_BY_AN_ACT = 2


def test_an_act_on_one_look_weighs_the_keys_of_no_other(shell: Shell) -> None:
    """Clearing one look in To Look At weighs its own row's keys again, and no other's.

    One act in For You's 115 rows weighed every row's keys for the foot again, 7 to 46 ms of
    the 11 to 52 its rows took to be shown, where one row had changed (2026-11-10).
    """
    state = Path(os.environ.get("XDG_STATE_HOME") or Path.home() / ".local/state")
    looks = state / "pi/agentic-harness.pi/looks"
    looks.mkdir(parents=True, exist_ok=True)
    written = [looks / f"weighed{at:02d}.json" for at in range(LOOKS_WAITING)]
    # Each came yesterday at noon, a minute apart: within three days, past which one goes
    # quiet (launcher/Looks.js), and saying "yesterday" until midnight. Under an hour old,
    # each said its minutes, so a minute turning between the count and the act remade every
    # such row and weighed it again: 5 to 11 on loaded builders, each early in an hour
    # (H6EDLC, 2026-11-13).
    hour = now("-u", "-d", "yesterday", "+%FT12")
    for at, look in enumerate(written):
        said = {"v": 1, "id": look.stem, "title": f"Weighed Look {at}", "kind": "file",
                "target": "/etc/hostname", "state": "waiting",
                "at": f"{hour}:{at:02d}:00Z"}  # fmt: skip
        launcher_on_niri.laid_whole(look, json.dumps(said))
    try:
        shell.as_account(shell.ipc("toggle"))
        shell.launcher_ready()
        typed_at_once(shell, "'for you' -k Return")
        shell.launcher_until(
            lambda shown: shown["path"][-1:] == ["For You"] and "To Look At" in shown["rows"]
        )
        shell.down_to("To Look At")
        shell.step_in()
        shell.launcher_until(
            lambda shown: (
                shown["path"][-1:] == ["To Look At"] and len(shown["rows"]) >= LOOKS_WAITING
            )
        )
        shell.normal()
        # Counted only once the weighing holds still, since on a loaded builder what came
        # before the act, as the rows shown again or the keys' face measured, can land
        # after it and count as its (one act weighed 9, 85PKB2, 2026-11-12). Clearing a
        # look asks nothing, so dd is the act.
        before = launcher_on_niri.weighed_held_still(shell)
        since = shell.mark()
        shell.send_chars("dd")
        shell.reported("act shown in ", lambda _rest: True, since)
        weighed = shell.showing()["weighed"] - before
        assert weighed <= WEIGHED_BY_AN_ACT, f"one act weighed {weighed} rows' keys again"
        shell.close_launcher()
    finally:
        for look in written:
            look.unlink(missing_ok=True)


def test_the_head_holds_the_finds_mark_alone_and_no_bare_question(shell: Shell) -> None:
    """The head's / stands only on the find, it asks no bare which?, and a find says its count."""
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    shown = shell.showing()
    assert (shown["mode"], shown["sigil"], shown["hint"]) == ("find", "/", ""), shown
    # Normal's keys are on the rows, so the mark's column stands empty (Joel, 2026-11-06:
    # "when in normal mode, it shouldn't have a / at the front").
    shell.normal()
    shown = shell.showing()
    assert (shown["sigil"], shown["hint"]) == ("", ""), shown
    # Home is a place, so its glyph leads its Title, where a bare band said nothing.
    assert shown["lead"] != "", shown
    # Now asks only "which?", which the mark and the caret already say.
    shell.step_in()
    shell.launcher_until(lambda shown: shown["path"][-1:] == ["Now"])
    assert shell.showing()["hint"] == "", shell.showing()
    shell.close_launcher()
    # A find left for normal says how many it found, never where the cursor is among them.
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    shell.type_in("power")
    # Home is a place, so its Title stands on the rows' names' column, as the : line's typing
    # does, so nothing moves between the two, and what's typed follows it.
    laid = laid_out(shell)
    titled = [word["x"] for word in laid["field"] if word["text"] == "Home"]
    typed = [word["x"] for word in laid["field"] if word["text"] == "power"]
    named = [
        word["x"]
        for row in laid["rows"]
        if row["selected"]
        for word in row["words"]
        if word["text"] == shell.showing()["selected"]["name"]
    ]
    assert titled, laid["field"]
    assert typed, laid["field"]
    assert named, laid["rows"]
    assert abs(titled[0] - named[0]) <= 1, (titled, named)
    assert typed[0] > titled[0], (typed, titled)
    shell.normal()
    shell.launcher_until(lambda shown: re.fullmatch(r"[1-9]\d* found", shown["found"]) is not None)
    shell.close_launcher()


def test_a_place_stands_its_head_and_rows_where_home_does(shell: Shell) -> None:
    """Stepping in from home moves no column: a place's glyphs and names stand where home's do.

    A place numbering only its own rows once held a narrower gutter than home's, so its head's
    glyph and Title and its rows' glyphs and names stood a digit left of home's for each digit
    fewer (Joel, 2026-11-06: "padding is different when on the home launcher screen and a
    secondary screen"). Machine holds about ten rows and Now a few, so each once had its own.

    The numbers start on the inset, where the mode word does, as wide as the place needs, so
    home's eight take one figure and Machine's two, the spare room after them (Joel,
    2026-11-06: "why leave constant room for three when we often have only 1?"). On the find
    its / takes the place glyph's cell, so only the Title and the rows are held to home's there.
    """
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    opening = shell.showing()["count"]
    figure = shell.showing()["gutter"]["figure"]
    # A find at home reaches every thing, and stands where home does too.
    shell.type_in("machine", selecting="Machine")
    found = columns(shell, "Home")
    numbers = {"a find": numbered(shell)}
    shell.normal()
    stood = {"Home": columns(shell, "Home")}
    numbers["Home"] = numbered(shell)
    shell.close_launcher()
    for place in ("Machine", "Now"):
        stood[place], numbers[place] = stepped_into(shell, place)
    assert len(set(stood.values())) == 1, f"glyphs and names stand apart: {stood}"
    assert found[1:] == stood["Home"][1:], f"a find stands apart: {found}, {stood['Home']}"
    starts = {place: at for place, (at, _, _, _) in numbers.items()}
    modes = {place: mode for place, (_, _, mode, _) in numbers.items()}
    assert starts == modes, f"numbers and the mode word stand apart: {starts}, {modes}"
    # A place's numbers take its own rows' digits; a find, the most it listed as it was typed,
    # home's at its opening at least, which home in normal after it keeps.
    # As wide as its figures, give or take the pixel a block rounds up by.
    off = {
        place: (numbers[place][1], len(str(numbers[place][3])) * figure)
        for place in ("Machine", "Now")
        if abs(numbers[place][1] - len(str(numbers[place][3])) * figure) > 1
    }
    assert not off, f"numbers not as wide as their figures ({figure} each): {off}, {numbers}"
    # A find holds the most it listed as it was typed, home's at its opening at least, and
    # normal after it keeps them: whole figures, never fewer.
    held = numbers["a find"][1]
    assert held == numbers["Home"][1] >= len(str(opening)) * figure - 1, numbers
    assert abs(held / figure - round(held / figure)) * figure <= 1, numbers


def test_the_heads_title_holds_still_and_its_mark_takes_the_glyphs_cell(shell: Shell) -> None:
    """The head's Title starts at one x in normal, on the find and in visual, as the : line's text.

    The head's first column held the find's / and nothing in normal, so normal showed a bare
    cell the find filled (Joel, 2026-11-06: "an extra space in front of the launcher heading
    when in command mode that we don't have in find"). Now / and : take the place glyph's
    cell, and nothing stands in the gutter's column in any mode.
    """
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    shell.type_in("k")
    titled = {"find": head_title(shell)}
    shell.normal()
    titled["normal"] = head_title(shell)
    shell.send_chars("v")
    shell.launcher_until(lambda shown: shown["mode"] == "visual")
    titled["visual"] = head_title(shell)
    shell.send_key("esc")
    shell.launcher_until(lambda shown: shown["mode"] == "normal")
    shell.send_chars(":lo")
    shell.launcher_until(lambda shown: (shown["command"] or {}).get("line") == "lo")
    line = laid_out(shell)["line"]
    typed = next(word for word in line if word["text"] == "lo")
    colon = next(word for word in line if word["text"] == ":")
    shell.send_key("esc")
    shell.close_launcher()
    marks = {mode: mark for mode, (_, mark) in titled.items()}
    titles = {mode: title for mode, (title, _) in titled.items()}
    assert len(set(titles.values())) == 1, f"the Title moves between modes: {titles}"
    assert typed["x"] == titles["normal"], f"the : line's text at {typed['x']}: {titles}"
    # The / and the : stand in the cell the place's glyph does, never before it.
    cell = marks["normal"]
    for mark in (marks["find"], colon["x"]):
        assert cell <= mark < titles["normal"], f"a mark off the glyph's cell: {marks}, {colon}"


def test_ctrl_r_puts_the_clipboard_on_the_command_line(shell: Shell) -> None:
    """Ctrl+R then + puts the clipboard's first line at the : line's caret, as Vim's does.

    The side notices' : line took it and the launcher's didn't, so the two lines kept two
    sets of keys; they were made one line (Joel, 2026-11-06: "they need to share more code").
    """
    shell.as_account("printf 'lockscreen\\nand more' | wl-copy > /dev/null 2>&1")
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    shell.normal()
    shell.send_chars(":")
    shell.launcher_until(lambda shown: shown["command"] is not None)
    shell.send_key("ctrl-r")
    shell.send_chars("+")
    shell.launcher_until(lambda shown: (shown["command"] or {}).get("line") == "lockscreen")
    shell.send_key("esc")
    shell.close_launcher()


def test_the_command_line_draws_no_line_chosen_until_tab_chooses_one(shell: Shell) -> None:
    """The : line's menu stands with no line on the selection's ground until Tab chooses one.

    Its rows read their chosen line from the list's own, which a list sets to its first row as
    a model comes, so the first line stood chosen before any key chose it.
    """
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    shell.normal()
    shell.send_chars(":")
    shell.launcher_until(lambda shown: bool((shown["command"] or {}).get("menu")))
    command = shell.showing()["command"]
    assert command["chosen"] == -1, command
    assert command["drawnChosen"] == -1, f"a line was drawn chosen before Tab: {command}"
    shell.send_key("tab")
    shell.launcher_until(
        lambda shown: (
            (shown["command"] or {}).get("chosen") == 0 and shown["command"].get("drawnChosen") == 0
        )
    )
    # Up steps back past the first line to none, and a letter typed then narrows the menu,
    # a new model, which a list given one sets its own current row on.
    shell.send_key("up")
    shell.launcher_until(lambda shown: (shown["command"] or {}).get("chosen") == -1)
    shell.send_chars("l")
    shell.launcher_until(lambda shown: (shown["command"] or {}).get("line") == "l")
    command = shell.showing()["command"]
    assert command["drawnChosen"] == -1, f"a line was drawn chosen once typed: {command}"
    shell.send_key("esc")
    shell.close_launcher()


# How many rows the launcher's room holds (Launcher.qml's tall), which a menu scrolls past.
ROOM_ROWS = 8


def test_the_command_lines_menu_makes_its_rows_once_and_keeps_them(shell: Shell) -> None:
    """A second : rebinds the menu's rows rather than making them again, even a launch later.

    Each row is a Choice, about 3 ms to make, so a menu remade on every : spent most of Joel's
    48 to 72 ms on the key (his journal, 15 November) making rows it had made before.
    """

    def menu_open() -> Json:
        shell.normal()
        shell.send_chars(":")
        shell.launcher_until(lambda shown: bool((shown["command"] or {}).get("menu")))
        command: Json = shell.showing()["command"]
        return command

    def chosen(at: int) -> Callable[[Json], bool]:
        def drawn(shown: Json) -> bool:
            command = shown["command"] or {}
            return bool(command.get("chosen") == at and command.get("drawnChosen") == at)

        return drawn

    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    made = menu_open()["rowsMade"]
    assert made > 0, "the menu's rows are not counted"
    broke = json.loads(shell.as_account(shell.ipc("contained")))
    assert broke == [], f"the menu's words broke out of their room: {broke}"
    shell.send_key("esc")
    shell.launcher_until(lambda shown: shown["command"] is None)
    again = menu_open()
    assert again["rowsMade"] == made, f"a second : made rows again: {made} then {again}"
    shell.send_key("esc")
    shell.close_launcher()
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    later = menu_open()
    assert later["rowsMade"] == made, f"a : a launch later made rows again: {made} then {later}"
    # A menu longer than the room scrolls its kept rows to the line Tab chooses, drawn chosen.
    shell.send_chars("set ")
    shell.launcher_until(lambda shown: len((shown["command"] or {}).get("menu") or []) > ROOM_ROWS)
    lines = len(shell.showing()["command"]["menu"])
    for at in range(lines):
        shell.send_key("tab")
        shell.launcher_until(chosen(at))
    broke = json.loads(shell.as_account(shell.ipc("contained")))
    assert broke == [], f"the scrolled menu's words broke out of their room: {broke}"
    shell.send_key("esc")
    shell.close_launcher()


def head_title(shell: Shell) -> tuple[int, int]:
    """Give where the head's Title starts and where the word before it, its cell's, starts."""
    field = laid_out(shell)["field"]
    at = before_and_at(field, "Home")
    assert at, field
    return at[1], at[0]


def stepped_into(
    shell: Shell, place: str
) -> tuple[tuple[int, int, int, int], tuple[int, int, int, int]]:
    """Open the launcher, step into a place from home, and give where its columns stand."""
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    shell.type_in(place.lower(), selecting=place)
    shell.step_in()
    shell.launcher_until(lambda shown: shown["path"][-1:] == [place])
    shell.normal()
    stood = columns(shell, place), numbered(shell)
    shell.close_launcher()
    return stood


def numbered(shell: Shell) -> tuple[int, int, int, int]:
    """Give where the gutter's numbers start, their block's width, the mode word's x, the rows."""
    laid = laid_out(shell)
    numbers = laid["numbers"]
    assert numbers, laid
    mode = next(word["x"] for word in laid["foot"] if re.fullmatch("[A-Z]{3}", word["text"]))
    count = shell.showing()["count"]
    # Where the numbers' block starts, which right-aligns them within it as Vim's column does.
    block = min(word["x"] for word in numbers)
    return block, max(word["width"] for word in numbers), mode, count


def columns(shell: Shell, title: str) -> tuple[int, int, int, int]:
    """Give where the head's glyph and Title and the cursor row's glyph and name stand."""
    shell.launcher_until(lambda shown: shown["gutter"]["shown"] and shown["selected"])
    name = shell.showing()["selected"]["name"]
    laid = laid_out(shell)
    head = before_and_at(laid["field"], title)
    row = next(row for row in laid["rows"] if row["selected"])
    named = before_and_at(row["words"], name)
    assert head, (title, laid["field"])
    assert named, (name, row["words"])
    return (*head, *named)


def before_and_at(words: list[Json], text: str) -> tuple[int, int] | None:
    """Give where the glyph before a word and the word itself start, or None if it's not there."""
    at = next((at for at, word in enumerate(words) if word["text"] == text), 0)
    if at == 0:
        return None
    return (cast("int", words[at - 1]["x"]), cast("int", words[at]["x"]))


def laid_out(shell: Shell) -> Json:
    """Give where the launcher draws each word, once two reads in a row agree."""
    was = None
    for _ in range(20):
        now = json.loads(shell.as_account(shell.ipc("laidOut")))
        if now == was:
            return cast("Json", now)
        was = now
    return cast("Json", was)


# The tie rung, in logical pixels: the room between the rows' bands and each hairline.
TIE = 4


def test_no_row_inks_its_marks_in_the_frames_colour(shell: Shell) -> None:
    """No word or mark on Home's rows is drawn in the hairlines' ink or the cursor's ground.

    A row holding more drew its glyph in the frame's edge, about 1.3:1 on the cursor's ground,
    so Now's mark vanished just where the cursor stood on it (VKCF3Z, 2026-11-06).
    """
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    shell.normal()
    shell.down_to("Now")
    laid = laid_out(shell)
    # Its colour alone, since the ground is the selection's colour at a wash.
    faint = {laid["hairlines"][0]["ink"][-6:], laid["selection"]["ink"][-6:]}
    lost = [
        (word["text"], word["ink"])
        for row in laid["rows"]
        for word in row["words"]
        if word["ink"][-6:] in faint
    ]
    assert not lost, f"rows draw in the frame's ink {faint}: {lost}"
    shell.close_launcher()


def test_rows_tile_and_the_cursor_row_is_filled_edge_to_edge(shell: Shell) -> None:
    """Home's rows meet with no gap, and the cursor row's ground fills its band, gutter and all.

    As Vim's cursorline fills its line (Joel, 2026-11-06: "rows in the launcher should have
    padding instead of margins"): the room a row keeps is inside its band, the ground runs
    from the frame's inside edge to its other, and a tie stands between the bands and each
    hairline, so a ground never touches a rule. Now's chord leaves its row for the foot.
    """
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    shell.normal()
    shell.down_to("Now")
    laid = laid_out(shell)
    bands = [row["band"] for row in laid["rows"]]
    tall = laid["rowTall"]
    apart = [(top, below) for top, below in itertools.pairwise(bands) if below != top + tall]
    assert not apart, f"bands {tall} tall stand apart: {apart} of {bands}"
    ground = laid["selection"]
    across = (ground["x"], ground["width"], ground["height"])
    assert across == (laid["inside"]["x"], laid["inside"]["width"], tall), (ground, laid)
    selected = next(row for row in laid["rows"] if row["selected"])
    assert ground["y"] == selected["band"], (ground, selected)
    head, foot = laid["hairlines"]
    rows = laid["inside"]
    assert bands[0] == rows["y"], f"the first band starts below the list: {bands}, {rows}"
    room = (rows["y"] - head["y"] - head["height"], foot["y"] - rows["y"] - rows["height"])
    assert room == (TIE, TIE), f"the rows stand {room} from the hairlines: {laid['hairlines']}"
    shown = shell.showing()
    assert shown["selected"]["chord"] == "", f"Now's row draws its chord: {shown['selected']}"
    # Furthest from Return, it's the first to give way, so it's asked for, if not kept.
    taught = " ".join(shown["wanted"])
    assert "next time" in taught, f"the foot doesn't teach Now's chord: {shown['wanted']}"
    # At the line's size, the heading outranks the cursor row's name by its ink at least.
    title = next(word for word in laid["field"] if word["text"] == "Home")
    name = next(word for word in selected["words"] if word["text"] == "Now")
    assert title["ink"] == name["ink"], f"the Title is fainter than its rows: {title}, {name}"
    shell.close_launcher()


def test_an_arrow_lights_its_row_with_the_selection_in_its_first_frame(shell: Shell) -> None:
    """The selection stands on the row an arrow lit in the arrow's first frame, never gliding.

    It glided to its row over up to 30 ms while the row's text lit at once, so the bar and the
    lit text sat on different rows for a frame or two (Joel, 2026-11-13 23:38: "the
    selection can drag behind what is actually currently active"). A list moved by keys lands
    on the frame after the key, as Vim's cursorline does (VKCF3Z, 9d5caf24).
    """
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    shell.type_in("o")
    shell.launcher_until(lambda shown: len(shown["rows"]) > 1)
    # The letter's frame is presented first, so the arrow is the key after it.
    deadline = time.monotonic() + launcher_on_niri.WAITING.total_seconds()
    typed = 0
    while typed == 0 and time.monotonic() < deadline:
        typed = len(json.loads(shell.as_account(shell.ipc("keysPresented"))))
        time.sleep(launcher_on_niri.POLL)
    shell.send_key("down")
    presented: list[Json] = []
    while len(presented) <= typed and time.monotonic() < deadline:
        presented = json.loads(shell.as_account(shell.ipc("keysPresented")))
        time.sleep(launcher_on_niri.POLL)
    shell.close_launcher()
    assert len(presented) > typed, f"the arrow was never presented: {presented}"
    arrow = presented[typed]
    assert arrow["onRow"], f"the selection was off its lit row in the arrow's first frame: {arrow}"


def test_the_selection_is_drawn_on_its_row_as_rows_above_it_change(shell: Shell) -> None:
    """Letters typed in a list scrolled down leave the selection drawn on its selected row.

    Rows coming or going above a scrolled list move where the list's rows start, and the
    selection drawn from the top stayed where its row had been, under another.
    """
    windows = [shell.opened(f"Fill {at:02}") for at in range(1, 13)]
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    shell.type_in("fill")
    shell.launcher_until(
        lambda shown: {f"Fill {at:02}" for at in range(1, 13)} <= set(shown["rows"])
    )
    drift = []
    typed = "fill"
    # Each a letter added or taken away, so rows above the selection go or come back.
    for change in [" 1", "", " 0", " ", " 1"]:
        # Down to the last row, so the list is scrolled.
        shown = shell.showing()
        below = len(shown["rows"]) - 1 - shown["rows"].index(shown["selected"]["name"])
        for _ in range(below):
            shell.send_key("down")
        shell.launcher_until(lambda shown: shown["selected"]["name"] == shown["rows"][-1])
        wanted = f"fill{change}"
        while not wanted.startswith(typed):
            shell.send_key("backspace")
            typed = typed[:-1]
        if wanted[len(typed) :]:
            shell.send_chars(wanted[len(typed) :])
        typed = wanted
        shell.launcher_until(
            lambda shown: (
                shown["typed"] == typed  # noqa: B023
                and (shown["selected"] or {"settled": True})["settled"]
            )
        )
        selection = laid_out(shell)["selection"]
        if selection and selection["y"] != selection["row"]:
            drift.append((typed, selection))
    shell.close_launcher()
    for window in windows:
        shell.close(window)
    assert not drift, f"the selection was drawn off its row: {drift}"


def test_a_letter_narrowing_the_rows_draws_none_it_kept(shell: Shell) -> None:
    """A letter that leaves rows listed draws none of them again, only rows new to the list.

    Kept only where the start and the end were the same, a letter narrowing a search drew
    every row it kept afresh, about 3 ms each in the sandbox and most of what its showing
    spent after its search (2026-11-09). Here the letter takes away rows at both ends.
    """
    titles = [f"Wren {name}" for name in ("Ash", "Birch", "Cedar", "Dogwood")]
    windows = [shell.opened(title) for title in titles]
    try:
        shell.as_account(shell.ipc("toggle"))
        shell.launcher_ready()
        shell.type_in("wren")
        shell.launcher_until(lambda shown: set(titles) <= set(shown["rows"]))
        before = laid_out(shell)
        was = shell.showing()
        shell.send_chars(" c")
        shell.launcher_until(lambda shown: shown["typed"] == "wren c")
        after = laid_out(shell)
        now = shell.showing()
        shell.close_launcher()
    finally:
        for window in windows:
            shell.close(window)
    # Else a row held before stood past what's reported, and this can't say it was kept.
    assert was["count"] <= len(was["things"]), was["count"]
    held = {thing["is"] for thing in was["things"]}
    new = [thing["is"] for thing in now["things"] if thing["is"] not in held]
    kept = [thing["is"] for thing in now["things"] if thing["is"] in held]
    # Else nothing was kept, and this proves nothing.
    assert kept, (was["rows"], now["rows"])
    drawn = int(after["drawn"]) - int(before["drawn"])
    assert drawn <= len(new), (
        f"{drawn} rows drawn afresh where {len(new)} are new ({new}); kept {kept}"
    )


def test_a_first_letter_draws_only_the_rows_in_view(shell: Shell) -> None:
    """A first letter draws the rows in view and a row past them, not a cache's height more.

    Qt's ListView builds rows a cache's height past its view as well as those shown, and a
    first letter or a step into a place lists rows wholly new, so each drew 15 rows where 8
    are shown, about half of what its showing spent after its search (2026-11-10).
    """
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    before = laid_out(shell)
    shell.type_in("s")
    shell.launcher_until(lambda shown: shown["typed"] == "s")
    after = laid_out(shell)
    shell.close_launcher()
    shown = math.ceil(after["inside"]["height"] / after["rowTall"])
    # Else the rows end within the old cache too, and this proves nothing.
    assert int(after["listed"]) > 2 * shown, (after["listed"], shown)
    drawn = int(after["drawn"]) - int(before["drawn"])
    # The rows shown, one cut by the view's foot among them, and one past it: the list stands
    # at its top, so no row is built above it.
    assert drawn <= shown + 1, f"{drawn} rows drawn for a first letter where {shown} are shown"


def test_a_launcher_closed_inside_a_place_is_home_before_it_opens(shell: Shell) -> None:
    """Close the launcher inside Programs, and it lets the step go once the card is gone.

    Joel's opening after one closed two steps deep spent 224 ms forgetting them, against 109 to
    126 after one closed at home (2026-11-10), so the next opening finds home already there.
    """
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    shell.type_in("programs", selecting="Programs")
    shell.step_in()
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_closed()
    shell.launcher_until(lambda shown: not shown["open"] and shown["held"] == [])
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    shell.launcher_until(lambda shown: shown["held"] == [] and "Programs" in shown["rows"])


def kept_out(shell: Shell) -> Json:
    """Give the list's last keeping and its last step back out's, with their tallies."""
    return cast("Json", json.loads(shell.as_account(shell.ipc("keptOut"))))


def laid_and_kept(shell: Shell) -> tuple[Json, Json]:
    """Give laidOut and keptOut read together, once two reads in a row of both agree.

    Read apart, a keeping landing between the two would give one keeping's rows beside
    another's tally.
    """
    was: tuple[Json, Json] | None = None
    for _ in range(20):
        now = (cast("Json", json.loads(shell.as_account(shell.ipc("laidOut")))), kept_out(shell))
        if now == was:
            return now
        was = now
    msg = f"laidOut and keptOut never read the same twice: {was}"
    raise AssertionError(msg)


def tallied_rows(kept: Json, which: str = "kept") -> tuple[int, int, int] | None:
    """Give the rows a laying tallied, built and took back, once it says it's done.

    Every row a laying draws is tallied under its kind, each with its own time, and none once
    it's done. Their times together are bounded by the laying's, which catches a row counted
    twice or a time read late; where each row's time starts is rows-kept.qml's, as a warm
    shell's reused rows always start after the laying's own mark. The laying is
    the last keeping's, or the last step back out's (`which` "stepped"). None where it laid
    nothing.
    """
    assert not kept["tallying"], "a laying still listens for rows once it's done"
    last = kept[which] or {}
    if "laidBy" not in last:
        return None
    by = last["laidBy"]
    unnamed = [kind for kind in by if kind in ("", "undefined", "null")]
    assert not unnamed, f"rows tallied under no kind: {by}"
    took = [float(each["ms"]) for each in by.values()]
    assert min(took, default=0) >= 0, f"a kind's rows took less than nothing: {by}"
    laying = float(last["laying"])
    # The journal says it in whole milliseconds, as its readers parse it.
    assert laying.is_integer(), f"a laying of {laying} ms isn't said whole"
    split = float((last.get("split") or {}).get("took", 0))
    assert split.is_integer(), f"a split of {split} ms isn't kept whole"
    # Rounded, the laying may say up to half a millisecond less than its rows took.
    assert sum(took) <= laying + 0.5 + 1e-6, f"rows took {took} of a {laying} ms laying"
    rows = sum(int(each["rows"]) for each in by.values())
    # Rows drawn take some time, on a clock that reads finer than a millisecond.
    assert rows == 0 or sum(took) > 0, f"{rows} rows tallied as taking no time: {by}"
    return rows, int(last["built"]), int(last["reused"])


def test_a_letter_keeping_some_rows_builds_none_but_takes_those_let_go(shell: Shell) -> None:
    """A letter that keeps some rows and lists others builds no row, once warm.

    The rows a letter let go were given on only when it let every row go, so a letter keeping
    a few, as Joel's "m" from home or "ca" after "c", built six to eight rows from nothing
    each time, up to 43 ms of laying them out (2026-11-13 and 14). Now the rows it lets go
    are given to those it lists.
    """
    built, kept, new, tallied, kinds, handed = [], [], [], [], set(), []
    for _ in range(3):
        shell.as_account(shell.ipc("toggle"))
        shell.launcher_ready()
        was = set(shell.showing()["rows"])
        before = int(laid_out(shell)["built"])
        shell.type_in("m")
        laid, kept_now = laid_and_kept(shell)
        built.append(int(laid["built"]) - before)
        tallied.append(tallied_rows(kept_now))
        kinds |= set((kept_now["kept"] or {}).get("laidBy") or {})
        handed.append((kept_now["composed"] or {}).get("laidBy"))
        now = set(shell.showing()["rows"])
        kept.append(len(now & was))
        new.append(len(now - was))
        shell.type_in("eet")
        shell.close_launcher()
    # "m" both kept rows of home and listed rows it didn't, or this proves nothing.
    assert min(kept) > 0, f"rows of home kept by each 'm': {kept}"
    assert min(new) > 0, f"rows new to each 'm': {new}"
    assert built[1:] == [0, 0], f"rows built by each 'm' from home: {built}"
    # The rows each 'm' took back are tallied, as many as it drew.
    laid_rows = [each for each in tallied if each is not None]
    assert any(back > 0 for _, _, back in laid_rows), f"no 'm' took a row back: {tallied}"
    drew = [(rows, made + back) for rows, made, back in laid_rows]
    assert all(rows == drawn for rows, drawn in drew), f"rows tallied, drawn: {drew}"
    # A row is tallied as what its provider says it is, such as a shortcut, where it says one,
    # not only as the sort every row has.
    own = kinds - {"act", "holds", "figure"}
    assert own, f"no 'm' tallied a row as what it is, only {sorted(kinds)}"
    # The trials file is handed each kind's time whole, as it keeps every time.
    by_kind = [by for by in handed if by]
    assert by_kind, f"no 'm' handed the trials file its rows by kind: {handed}"
    fine = [each["ms"] for by in by_kind for each in by.values() if not isinstance(each["ms"], int)]
    assert not fine, f"the trials file was handed kinds' times of {fine} ms"


def test_a_step_into_a_place_builds_no_row_but_takes_those_let_go(shell: Shell) -> None:
    """A step into Desktop or Machine and back builds no row, once one round has built them.

    Each step into a place built every row it showed from nothing, 2.5 to 3 ms each, about two
    thirds of the step's own work under load, as Joel stepped into Desktop in up to half a
    second (2026-11-10: "this needs to be as fast as thought no matter how loaded my machine
    is"). The rows a step lets go are given to those it shows, so after the first round,
    which may build a row the opening hadn't, a step builds none.

    Every row a place shows is drawn as its own: its name among its words, and no word only
    the other place says, with nothing reading on but the selection, so a row given on takes
    nothing of the one it was. And each row drawn again reads the same, word for word, in
    the same places and inks, as the first time, whichever row it was given.
    """
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    built, rounds, tallied = [], [], []
    for place in ("Desktop", "Machine") * 2:
        shell.type_in(place.lower(), selecting=place)
        before = int(laid_out(shell)["built"])
        shell.step_in()
        shell.launcher_until(functools.partial(stands_in, place))
        shell.normal()
        laid, kept_now = laid_and_kept(shell)
        built.append(int(laid["built"]) - before)
        tallied.append(tallied_rows(kept_now))
        rounds.append((place, laid))
        stepped_out(shell)
    shell.close_launcher()
    # The second round's steps built nothing.
    assert built[2:] == [0, 0], f"rows built by each step, Desktop then Machine twice: {built}"
    # The rows each step took back are tallied too, as many as it drew.
    laid_rows = [each for each in tallied if each is not None]
    assert laid_rows, f"no step laid a row: {tallied}"
    drew = [(rows, made + back) for rows, made, back in laid_rows]
    # Else every step drew nothing, and the tally's matching proves nothing.
    assert any(drawn > 0 for _, drawn in drew), f"no step drew a row: {drew}"
    assert all(rows == drawn for rows, drawn in drew), f"rows tallied, drawn: {drew}"
    names = {place: {row["reads"].split(" \u00b7 ")[0] for row in laid["rows"]}
             for place, laid in rounds}  # fmt: skip
    for place, laid in rounds:
        others = set().union(*(names[other] for other in names if other != place))
        only = others - names[place]
        # Else no row is shown, and this proves nothing.
        assert laid["rows"], (place, laid)
        for row in laid["rows"]:
            texts = {word["text"] for word in row["words"]}
            name = row["reads"].split(" \u00b7 ")[0]
            assert name in texts, f"{place}'s {row['is']} doesn't say {name}: {sorted(texts)}"
            left = texts & only
            assert not left, f"{place}'s {row['is']} says what it said elsewhere: {left}"
            assert row["selected"] or row["readsDone"] == 0, (place, row["is"], row)
    for place in ("Desktop", "Machine"):
        first, again = (dict(drawn_rows(laid)) for shown, laid in rounds if shown == place)
        # A row whose figures moved between the visits reads otherwise, and isn't compared.
        both = first.keys() & again.keys()
        # Else every row read differently between the visits, and this proves nothing.
        assert both, (place, first, again)
        differ = {key: (first[key], again[key]) for key in both if first[key] != again[key]}
        assert not differ, f"{place}'s rows drawn otherwise the second time: {differ}"


def test_a_find_finding_nothing_keeps_the_rows_it_let_go(shell: Shell) -> None:
    """Taking back a find that found nothing builds no row, those it let go taken back.

    Qt's list drops every row it has laid aside whenever it moves, and it moved down a row while
    the line saying nothing was found showed, so the rows a miss let go were gone: Joel's
    Backspace after one built 3 to 9 rows, 14 to 21 ms laying them out, and his - out of To Try
    after one built 6 of 7, 20 ms (his journal, 16 and 17 November). Each miss here is typed in
    Desktop, the second time asserted, as the first may build what the launcher never showed.
    """
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    built = []
    for _ in range(2):
        shell.type_in("desktop", selecting="Desktop")
        shell.step_in()
        shell.launcher_until(functools.partial(stands_in, "Desktop"))
        missed = "zqxj"
        shell.type_in(missed)
        before = int(laid_out(shell)["built"])
        shell.as_account("wtype " + " ".join(["-k BackSpace"] * len(missed)))
        shell.launcher_until(lambda shown: shown["typed"] == "" and shown["rows"] != [])
        built.append(int(laid_out(shell)["built"]) - before)
        shell.close_launcher()
        shell.as_account(shell.ipc("toggle"))
        shell.launcher_ready()
    shell.close_launcher()
    assert built[1:] == [0], f"rows built taking back each miss in Desktop: {built}"


# How many notices the far step out is taken among, and how far down them it stands.
NOTICES_MANY = 30
NOTICES_DOWN = 16


def test_a_step_out_lays_out_only_the_rows_it_comes_back_to(shell: Shell) -> None:
    """- back to a row far down a list draws no more rows than stood around it, where it stood.

    The rows came back laid out from the list's top, then the selection's row was scrolled to
    and its rows laid out again, so Joel's - out of a notice twenty rows down Desktop took 68
    ms, 41 of it after the rows were shown (his journal, 2026-11-17 16:39:52). Sixteen down is
    twice the rows a card shows, as far as the twenty rows the shell says of its list. The rows it
    draws are counted the second time, as the first may build what the notices never showed, and
    held to those drawn while the row was selected before, not to those it drew itself. The
    selection moves down the drawer first, as a search coming back selects its first row and
    showing that one scrolled to the list's top.
    """
    # Each rests at once, so the line has none left to show by the last.
    with shell.told(f"rests Heron {NOTICES_MANY - 1:02}", timedelta(seconds=90)):
        for at in range(NOTICES_MANY):
            shell.as_account(f"notify-send --expire-time=1 'Heron {at:02}' 'one of many'")
    shell.notices_gone()
    drawn, tallied = [], []
    for _ in range(2):
        # Now's Desktop, which holds the notices rested off the screen, since none is critical;
        # home's own row named Desktop is another place.
        shell.as_account(shell.ipc("toggle"))
        shell.launcher_ready()
        shell.normal()
        shell.down_to("Now")
        shell.step_in()
        shell.launcher_until(lambda shown: shown["held"] == ["now"] and "Desktop" in shown["rows"])
        shell.normal()
        shell.down_to("Desktop")
        shell.step_in()
        shell.launcher_until(lambda shown: len(shown["rows"]) > NOTICES_DOWN)
        shell.normal()
        far = shell.showing()["rows"][NOTICES_DOWN]
        shell.down_to(far)
        standing = laid_out(shell)
        # Else the list's top is in reach, and the case can't tell a split from none.
        assert standing["firstAt"] is None, f"the first row was drawn with {far}: {standing}"
        # Into its drawer, nothing that sent it being there to go to (PastNotices.qml).
        shell.step_in()
        inside = shell.showing()["rows"]
        # Else the selection can't move off the drawer's first row.
        assert len(inside) > 1, f"{far}'s drawer holds {inside}"
        shell.down_to(inside[-1])
        laid = laid_out(shell)
        before = int(laid["drawn"])
        depth = len(shell.showing()["held"])
        shell.send_chars("-")
        shell.launcher_until(functools.partial(stepped_back_to, depth=depth, far=far))
        laid, kept_now = laid_and_kept(shell)
        drawn.append(int(laid["drawn"]) - before)
        # Fails loudly if the split is ever passed by, as the bound could hold without it.
        assert (laid["stepped"] or {}).get("from", 0) > 0, f"- didn't split: {laid['stepped']}"
        # Its rows are tallied too, those the split drew among them, in no more than its laying
        # took, read from its own keeping, which those after it leave: the split's own.
        read = (kept_now["stepped"] or {}).get("split")
        assert read == laid["stepped"], f"the tally read {read}, not the split {laid['stepped']}"
        tallied.append(tallied_rows(kept_now, "stepped"))
        was, now = band_of(standing, far), band_of(laid, far)
        assert was is not None, f"{far} wasn't drawn before stepping in"
        assert now == was, f"- back put {far} at {now}, not at {was} where it stood"
        assert (laid["selection"] or {}).get("row") == now, f"{far} isn't lit: {laid['selection']}"
        for row in laid["rows"]:
            name = row["reads"].split(" \u00b7 ")[0]
            texts = {word["text"] for word in row["words"]}
            assert name in texts, f"{row['is']} doesn't say {name}: {sorted(texts)}"
        shell.close_launcher()
    # The rows drawn around the row before, the one built ahead each way among them.
    reach = len(standing["rows"])
    assert drawn[1] <= reach, f"rows drawn by each - back to {far}: {drawn}, {reach} in reach"
    laid_rows = [each for each in tallied if each is not None]
    assert laid_rows, f"no - back's split tallied a row: {tallied}"
    drew = [(rows, made + back) for rows, made, back in laid_rows]
    # Else every split drew nothing, and the tally's matching proves nothing.
    assert any(count > 0 for _, count in drew), f"no - back's split drew a row: {drew}"
    assert all(rows == count for rows, count in drew), f"rows each - back tallied, drew: {drew}"


def band_of(laid: Json, name: str) -> int | None:
    """Give where the row by this name is drawn on the card, its name drawn, or None."""
    bands = [
        row["band"]
        for row in laid["rows"]
        if row["reads"].split(" \u00b7 ")[0] == name
        and any(word["text"] == name for word in row["words"])
    ]
    return cast("int", bands[0]) if bands else None


def stepped_back_to(shown: Json, *, depth: int, far: str) -> bool:
    """Say whether - has come back a step from this deep, to this row selected."""
    return len(shown["held"]) == depth - 1 and (shown["selected"] or {}).get("name") == far


def drawn_rows(laid: Json) -> Iterator[tuple[tuple[str, str], list[tuple[object, ...]]]]:
    """Give each row shown by what it is and reads, with its words as drawn, each in order.

    The selected row is left out, since its name may be reading on. A graph is given by its ink,
    height and width alone: what it draws is the figure's history, which its reading leaves out,
    and where it stands is the list's, beside the widest of every graphed row's figures
    (Choice.qml's `figures`), so another row's figure growing by a character moves it.
    """
    for row in laid["rows"]:
        if row["selected"]:
            continue
        words: list[tuple[object, ...]] = [
            ("graph", None, word["y"] - row["band"], word["ink"], word["width"])
            if is_graph(word["text"])
            else (word["text"], word["x"], word["y"] - row["band"], word["ink"], word["width"])
            for word in row["words"]
        ]
        yield (str(row["is"]), str(row["reads"])), words


def is_graph(text: object) -> bool:
    """Say whether a word is a graph's, set wholly in braille as Graph.qml sets one."""
    return isinstance(text, str) and text != "" and all("\u2800" <= c <= "\u28ff" for c in text)


def test_the_drawers_fact_rows_are_kept_between_programs_and_steps(shell: Shell) -> None:
    """An arrow between programs, or a step back into Programs, makes no fact row it has.

    The drawer made a row afresh for each fact every time its facts changed: the first
    program's three took 5 ms of a 19 ms step into Programs at rest, made again on every step
    since home's drawer holds none, and again on every arrow (2026-11-10). Its rows are kept
    once made, each reading the fact at its place, so only a thing with more facts than any
    before makes rows.
    """
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    held = [stepped_into_programs(shell)]
    made = int(laid_out(shell)["factsMade"])
    lacked = 0
    # Down to the last and back: the test's account has three programs, the Foot terminal's.
    for key in "jjkk":
        title = shell.showing()["drawer"]["title"]
        shell.send_chars(key)
        shell.launcher_until(functools.partial(drawer_moved, title))
        held.append(int(laid_out(shell)["factsHeld"]))
        lacked += max(0, held[-1] - max(held[:-1]))
    stepped_out(shell)
    held.append(stepped_into_programs(shell))
    lacked += max(0, held[-1] - max(held[:-1]))
    made = int(laid_out(shell)["factsMade"]) - made
    stepped_out(shell)
    shell.close_launcher()
    # Else no program after the first has a fact, and this proves nothing.
    assert sum(held[1:]) > 0, f"facts held by each program's drawer: {held}"
    assert made <= lacked, f"{made} fact rows made where {lacked} were missing, holding {held}"


def test_a_drawer_shown_again_measures_little_anew(shell: Shell) -> None:
    """Programs' drawers shown a second time measure few of the widths their facts ask.

    A program's drawer measured its facts' words in their faces about 160 times each time it
    was given, though under one in a hundred were words it hadn't measured: 2 of the 5 ms a
    Chrome drawer took to show offscreen, where Joel's took up to 14 (2026-11-14). So each
    face keeps the widths it has measured, and the same drawers shown again ask it, not Qt.
    """
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    rounds = []
    for _ in range(2):
        before = laid_out(shell)
        stepped_into_programs(shell)
        # Down to the last and back: the test's account has three programs, the Foot terminal's.
        for key in "jjkk":
            title = shell.showing()["drawer"]["title"]
            shell.send_chars(key)
            shell.launcher_until(functools.partial(drawer_moved, title))
        after = laid_out(shell)
        rounds.append({count: int(after[count]) - int(before[count])
                       for count in ("widthsAsked", "widthsMade")})  # fmt: skip
        stepped_out(shell)
    shell.close_launcher()
    again = rounds[1]
    # Else the drawers asked no widths, and this proves nothing.
    assert again["widthsAsked"] > 0, f"widths asked and measured by each round: {rounds}"
    assert again["widthsMade"] * 4 < again["widthsAsked"], (
        f"widths asked and measured by each round: {rounds}"
    )


# Emoji's first row: the first of Unicode's groups, which it lists with nothing typed.
FIRST_GROUP = "Smileys & Emotion"
# How many of Emoji's rows its start makes and Super+E lists at most: its groups, not 1,914.
EMOJI_MADE_MOST = 20
# How many emoji Desktop's Emoji says it holds at least, though its own rows are its groups.
EMOJI_LISTED_LEAST = 1000
# How each of Emoji's group rows' keys begins (clipboard/src/emoji.rs), and no emoji's does.
EMOJI_GROUP_KEY = "row:emoji:group:"

DRAWER_SAID = re.compile(
    r'launcher: drawer for "((?:[^"\\]|\\.)*)" presented (\d+) ms from the key: a median (\d+) ms '
    r"before it followed, (\d+) until its frame began and (\d+) from its frame's start"
)
# Where a drawer's time from its frame's start went, where the render thread timed its swap.
DRAWER_SWAPPED = re.compile(r"from its frame's start \((\d+) to its swap and (\d+) after it\)")
DRAWERS_SAID = re.compile(r"launcher: \d+ drawers? in \S+ presented a median \d+ ms from the key")


def came_in_its_keys_frame(line: str) -> bool:
    """Say whether a drawer's line says it came in its key's frame, before the screen it names."""
    return line.rsplit("; on ", 1)[0].endswith("in the key's frame")


def test_a_drawer_is_said_presented_from_the_key_that_brought_it(shell: Shell) -> None:
    """A drawer an arrow brings says how long it took from that key to niri's screen.

    Joel's 235 drawers on 2026-11-11 were timed only from their own work to their frame,
    though each followed its key by 50 ms and more and reached the screen 25 to 46 ms after
    that frame (Joel, 20:28: "the drawer in the launcher and its time to appear needs to be
    kept to the same performance requirements as everything else"). So each is said from the
    key, where its time went adding up to it, and the close says the drawers by place.
    """
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    stepped_into_programs(shell)
    title = shell.showing()["drawer"]["title"]
    since = shell.mark()
    shell.send_chars("j")
    shell.launcher_until(functools.partial(drawer_moved, title))
    said = said_once(shell, since, DRAWER_SAID)
    drawn = DRAWER_SAID.search(said)
    lines = [line for line in said.splitlines() if "drawer for" in line]
    assert drawn, f"the drawer the key brought was never said presented: {lines}"
    took, following, framing, showing = (int(part) for part in drawn.groups()[1:])
    assert abs(following + framing + showing - took) <= ROUNDED, (
        f"{took} ms from the key, said as {following}, {framing} and {showing}"
    )
    swapped = DRAWER_SWAPPED.search(said)
    assert swapped, f"the drawer's frame never said its time to its swap and after: {lines}"
    to_swap, after_swap = (int(part) for part in swapped.groups())
    assert abs(to_swap + after_swap - showing) <= ROUNDED, (
        f"{showing} ms from its frame's start, said as {to_swap} to its swap and {after_swap}"
    )
    stepped_out(shell)
    shell.close_launcher()
    closed = said_once(shell, since, DRAWERS_SAID)
    assert DRAWERS_SAID.search(closed), "the close never said the drawers presented by place"


def test_an_emoji_shows_the_drawer_the_shell_makes_of_it(shell: Shell) -> None:
    """An emoji's row shows the drawer the shell makes of it, worded, said from its key.

    The history serves no emoji's drawer, its row saying only that it has one, since the row
    holds all it says (launcher/EmojiDrawer.js): its group, keywords and code points, named
    in the launcher's words. An emoji once held no drawer, only a list of Paste and Copy,
    which Return and y already do (Joel, 2026-11-11 22:00: "if they have nothing under them,
    we shouldn't open a submenu for them"). That it opens no list is
    modules/quickshell/tests/ready-rows.qml's, over what the history serves. Emoji lists
    Unicode's groups, so the emoji are a step into the first.
    """
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    shell.type_in("emoji", selecting="Emoji")
    shell.step_in()
    shell.launcher_until(functools.partial(stands_in, "Emoji"))
    shell.launcher_until(lambda shown: (shown.get("selected") or {}).get("name") == FIRST_GROUP)
    shell.step_in()
    shell.launcher_until(functools.partial(stands_in, FIRST_GROUP))
    shell.normal()
    shell.launcher_until(drawer_on_selected)
    drawer = shell.showing()["drawer"]
    title = drawer["title"]
    assert drawer["eyebrow"] == FIRST_GROUP, f"the emoji's drawer named no group: {drawer}"
    assert drawer["nerd"] == ["Code Points"], f"its code points went unnamed: {drawer}"
    # Unicode's first emoji, the grinning face, has keywords, so a drawer without them, or
    # with other words beside their name, lost them between the served row and the made
    # drawer. Its title is held first, so a list that leads with another fails as itself. The
    # words are read beside the drawn facts' names, the list they come from, since the drawn
    # card drops a fact that repeats its title; and a value still being asked is said as such.
    assert title == "Grinning face", f"Unicode's first emoji isn't the grinning face: {drawer}"
    assert drawer["facts"] == ["Keywords"], f"its keywords went unnamed or missing: {drawer}"
    assert drawer["drawnFacts"] == ["Keywords"], f"its drawer didn't draw its keywords: {drawer}"
    keywords = drawer["values"][0]
    assert keywords is not None, f"its keywords were still being asked: {drawer}"
    assert "grin" in str(keywords).split(", "), f"its keywords were lost: {drawer}"
    since = shell.mark()
    shell.send_chars("j")
    shell.launcher_until(functools.partial(drawer_moved, title))
    said = said_once(shell, since, DRAWER_SAID)
    lines = [line for line in said.splitlines() if "drawer for" in line]
    assert DRAWER_SAID.search(said), f"the emoji's drawer was never said presented: {lines}"
    stepped_out(shell)
    shell.close_launcher()


def test_emoji_makes_a_row_only_for_an_emoji_it_shows(shell: Shell) -> None:
    """Super+E lists Unicode's groups, and makes a row only for an emoji a step shows.

    Every one of Emoji's 1,914 rows was made as the shell started, kept for Super+E and matched
    in the shell, about 30 MB of what its heap held (Y2YBTS's lab, 2026-11-19). The history
    finds what's typed now (clipboard/src/search.rs, the whole-word case below), so the start
    makes only its groups' rows, a group's row moved over makes none of its emoji, and closing
    lets go of those a step into a group made, which were otherwise held for the shell's life.
    The tally counts every row since the shell started, and a case before this one in its part
    steps into a group, so each count here is from this case's start; the start itself is
    judged by having made fewer than Emoji holds, which a step into one group never reaches.
    """
    launcher_on_niri.warmed(shell)
    before = shell.rows_made("emoji")
    assert before < EMOJI_LISTED_LEAST, (
        f"Emoji made {before} rows before this case, so its start made every emoji"
    )
    shell.as_account(shell.ipc("collection", "emoji"))
    shell.launcher_ready()
    shell.launcher_until(lambda shown: shown["rows"][:1] == [FIRST_GROUP])
    # Once its groups are listed the history's records have come, so an opening that made
    # every emoji has made them by now.
    made = shell.rows_made("emoji") - before
    assert made < EMOJI_MADE_MOST, f"Emoji made {made} rows before a group was stepped into"
    listed = shell.showing()
    assert listed["count"] < EMOJI_MADE_MOST, (
        f"Super+E listed {listed['count']} rows, {listed['rows'][:12]}, not Unicode's groups"
    )
    shell.normal()
    shell.send_chars("j")
    shell.launcher_until(lambda shown: (shown.get("selected") or {}).get("name") != FIRST_GROUP)
    # The selection's actions are read a moment after it moves (Launcher.qml's
    # readSelectedActions, called later), which would make every emoji of a group they read,
    # so what moving made is judged once Emoji has stopped making rows, and by each row's key:
    # its groups' rows may be made again, but none of their emoji.
    shell.made_settled("emoji")
    moved = shell.rows_made("emoji") - before
    assert moved - made <= launcher_on_niri.LATELY_KEPT, (
        f"moving to the next group made {moved - made} rows: {shell.made_lately('emoji')}"
    )
    made_moving = shell.made_since("emoji", before + made)
    emoji = [key for key in made_moving if not key.startswith(EMOJI_GROUP_KEY)]
    assert not emoji, f"moving to the next group made {len(emoji)} of its emoji: {emoji}"
    shell.send_chars("k")
    shell.launcher_until(lambda shown: (shown.get("selected") or {}).get("name") == FIRST_GROUP)
    shell.step_in()
    shell.launcher_until(functools.partial(stands_in, FIRST_GROUP))
    stepped = shell.rows_made("emoji") - before
    assert stepped > moved, (
        "a step into the first group made none of its emoji, so they were made before this"
        " case and kept: a close before it let none go, or the start made them"
    )
    shell.close_launcher()
    shell.as_account(shell.ipc("collection", "emoji"))
    shell.launcher_ready()
    shell.launcher_until(lambda shown: shown["rows"][:1] == [FIRST_GROUP])
    shell.step_in()
    shell.launcher_until(functools.partial(stands_in, FIRST_GROUP))
    again = shell.rows_made("emoji") - before - stepped
    assert again > 0, "closing kept every emoji the first step made, so the second made none"
    shell.close_launcher()


def test_emoji_finds_a_whole_word_typed_in_it(shell: Shell) -> None:
    """A word typed into Super+E is found by the clipboard's history, its emoji first.

    Each letter asks the history again (launcher/Emoji.qml's search), so the rows a word's first
    letter found must give way to the word's own. It stands apart from the rows case above so
    a quarantine of it excuses only this search, not the bounds on what Emoji makes.
    """
    shell.as_account(shell.ipc("collection", "emoji"))
    shell.launcher_ready()
    # Its groups listed, Emoji has heard from the history, so a case run alone types into a
    # search that can ask it, as one run after the rows case does.
    shell.launcher_until(lambda shown: shown["rows"][:1] == [FIRST_GROUP])
    # Twice a failure here showed the rows held from "t" and never tada's, with nothing in the
    # journal to say why (2026-11-23), so a failure says how Emoji's search of the history stood.
    try:
        shell.type_in("tada", selecting="Party popper")
    except AssertionError as failed:
        said = f"{failed}; Emoji's search: {shell.searching('emoji')}"
        raise AssertionError(said) from failed
    shell.close_launcher()


# How the head says a find the history answered only in part, in normal: those shown, of all it
# found (Launcher.qml's narrowed, which showing() gives as `found`).
FOUND_OF = re.compile(r"(\d+) of (\d+) found")


def test_emoji_says_how_many_it_found_past_those_it_answers(shell: Shell) -> None:
    """A find in Emoji the history answers with its first hundred says how many it found.

    The history answers a find with its best hundred emoji and how many it found
    (clipboard/src/search.rs), and Emoji passes that on (launcher/Emoji.qml's search), so the
    head says "100 of 412 found" rather than "100 found" as though that were all. An f finds
    every flag and face, past a hundred.
    """
    shell.as_account(shell.ipc("collection", "emoji"))
    shell.launcher_ready()
    shell.launcher_until(lambda shown: shown["rows"][:1] == [FIRST_GROUP])
    shell.type_in("f")
    # Emoji lists nothing for an f until the history answers, which type_in takes as settled.
    shell.launcher_until(lambda shown: shown["count"] > 0)
    shell.normal()
    # The head judged is the one the wait passed on, as a later showing() could come after the
    # answer was let go and say nothing.
    heads: list[Json] = []

    def says_found(shown: Json) -> bool:
        heads.append(shown)
        return cast("str", shown["found"]) != ""

    shell.launcher_until(says_found)
    said, listed = cast("str", heads[-1]["found"]), int(heads[-1]["count"])
    shell.close_launcher()
    counted = FOUND_OF.fullmatch(said)
    assert counted, f"Emoji's head never said how many it found past those it shows: {said}"
    shown, found = (int(part) for part in counted.groups())
    # The head says "N of M" only where M is more, so what pins it is that N is the list it
    # stood over, its groups found with the history's hundred.
    assert shown == listed, f"the head said {shown} of {found} found over {listed} rows: {said}"
    assert shown < found, f"the head said {shown} of {found} found: {said}"


def test_a_cheap_drawer_comes_in_its_keys_own_frame(shell: Shell) -> None:
    """A drawer out whose kind makes cheaply reaches the screen in the frame its key does.

    Each drawer came the frame after its key, waiting a whole refresh for its swap: 16 ms of
    Joel's median of 69 from the key, against his 50 (2026-11-12), while making one took a
    median of 2. So one whose kind made in under 13 ms of late comes in the key's own frame,
    and its line says so (VKCF3Z, 2026-11-12, d4c660a6; 8 rather than 3 from 2026-11-16, for
    For You's, made in 4 to 7; 13 from 2026-11-26, for on's, collections' and notices', made
    in 7 to 8).
    Home's first places make theirs in 0 to 2 ms here, so the walk is over home's.
    """
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    shell.normal()
    # Down to the third place and back, three times, so each of the first three has made its
    # drawer thrice before the key that's measured, which brings the third's again. Twice
    # wasn't enough: the median of two makings is their mean, so one dear making on a loaded
    # builder could hold it at 3 and send a 1 ms drawer the frame after (KP0L6A, 2026-11-13).
    for key in "jjjkkjjkkjjk":
        title = (shell.showing()["drawer"] or {}).get("title", "")
        shell.send_chars(key)
        shell.launcher_until(functools.partial(drawer_moved, title))
    since = shell.mark()
    title = shell.showing()["drawer"]["title"]
    shell.send_chars("j")
    shell.launcher_until(functools.partial(drawer_moved, title))
    said = said_once(shell, since, DRAWER_SAID)
    lines = [line for line in said.splitlines() if "drawer for" in line]
    assert any(came_in_its_keys_frame(line) for line in lines), lines
    keys = json.loads(shell.as_account(shell.ipc("keysPresented")))
    drawers = json.loads(shell.as_account(shell.ipc("drawersPresented")))
    drawer = drawers[-1]
    brought = next((one for one in keys if one["from"] == drawer["from"]), None)
    assert brought is not None, f"no key pressed when the drawer's was: {drawer}, {keys[-3:]}"
    assert drawer["at"] == brought["at"], (
        f"the drawer came after its key's frame: {drawer}, {brought}"
    )
    # The line ends with the screen it was shown on, so a docked day's drawers read as the
    # screen's. The nested niri's one screen is winit's, which niri times no refresh for, so
    # the line names it with no rate; pacing.qml's cases say a rate where niri gives one.
    assert drawer["screen"] == "winit", f"the drawer wasn't on niri's one screen: {drawer}"
    assert any(line.endswith("; on winit") for line in lines), lines
    shell.close_launcher()


def test_a_drawer_inside_a_place_is_said_by_its_row_and_place(shell: Shell) -> None:
    """Each drawer walked inside Network is said by its row, and tallied in Network at the close.

    Network's own answers carry no key and its commands no source, so the lab's walk through
    it said 15 of its drawers as `drawer for ""` and all of them as drawers in home
    (2026-11-12), and neither could be told from home's in Joel's journal. The nested shell
    runs with reports on, so each is said by its name here; with them off, as on Joel's
    machines, an answer is said as one, which pacing.qml's rowShown cases hold.
    """
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    shell.type_in("network", selecting="Network")
    shell.step_in()
    shell.launcher_until(functools.partial(stands_in, "Network"))
    shell.normal()
    since = shell.mark()
    rows = shell.showing()["rows"]
    for row in rows[1:]:
        shell.send_chars("j")
        shell.launcher_until(functools.partial(drawer_on, row))
    stepped_out(shell)
    shell.close_launcher()
    said = said_once(shell, since, DRAWERS_SAID)
    lines = [line for line in said.splitlines() if "drawer" in line and "presented" in line]
    assert lines, f"no drawer walked in Network was said presented, of {rows}"
    # A drawer that loses its row is said as `row` through rowShown, so a walk whose rows all
    # have names must say it of none; and nothing, only by a line that skipped rowShown.
    unnamed = [line for line in lines if any(f"drawer for {no}" in line for no in ('"row"', '""'))]
    assert not unnamed, f"drawers said with no row: {unnamed}"
    in_home = [line for line in lines if " in home " in line]
    assert not in_home, f"Network's drawers were tallied in home: {in_home}"


def drawer_on(row: str, shown: Json) -> bool:
    """Say whether the row named is selected with its drawer out, or makes none.

    A row whose source can't answer here makes no drawer, as Work Network's does with no
    warp-cli to read (2026-11-12), so the walk passes it rather than waiting on it. An
    answer's drawer is titled by what it answers rather than its row's name, as Address's
    "no address" is, so the drawer out is the one the row makes.
    """
    selected = cast("Json", shown.get("selected") or {})
    if selected.get("name") != row:
        return False
    made = cast("Json | None", selected.get("drawer"))
    drawer = cast("Json", shown["drawer"])
    return made is None or (bool(drawer["shown"]) and drawer["title"] == made.get("title"))


def said_once(shell: Shell, since: int, line: re.Pattern[str]) -> str:
    """Give the shell's log since `since` once it holds `line`, else as the wait ends."""
    said = ""
    deadline = time.monotonic() + launcher_on_niri.WAITING.total_seconds()
    while not line.search(said) and time.monotonic() < deadline:
        said = shell.log.read_bytes()[since:].decode(errors="replace")
        time.sleep(launcher_on_niri.POLL)
    return said


def says_its_clock(said: str, key: Json) -> bool:
    """Whether one key's part of the close's clock line says its record's press, swap and screen."""
    one = CLOCK_KEY.fullmatch(said)
    if not one:
        return False
    pressed, swapped, shown, later = one.groups()

    def near(tenth: str, ms: float) -> bool:
        return tenth != "-" and abs(float(tenth) - float(ms)) <= TENTH

    return (
        (pressed == "-" if key["stamped"] is False else int(pressed) == math.floor(key["from"]))
        and near(swapped, key["swapped"])
        and near(shown, key["at"])
        and int(later or 0) == key["later"]
    )


def stepped_into_programs(shell: Shell) -> int:
    """Step into Programs from home and give how many facts its first program's drawer holds."""
    shell.type_in("programs", selecting="Programs")
    shell.step_in()
    shell.launcher_until(functools.partial(stands_in, "Programs"))
    shell.normal()
    shell.launcher_until(drawer_on_selected)
    return int(laid_out(shell)["factsHeld"])


def test_a_closed_program_holding_only_new_windows_holds_nothing_to_go_into(
    shell: Shell,
) -> None:
    """Tab on a closed program holding only its New Windows says not here, and Return starts it.

    Foot Server in Programs, closed, went into a list of New Window and New Window Below, a
    submenu of acts alone (Y2YBTS's sweep, 2026-11-12 [39f55fff]: a row holding only acts is
    never stepped into). Foot's server opens no window, so its start is seen as the launcher
    closing; the server is stopped after.
    """
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    stepped_into_programs(shell)
    shell.down_to("Foot Server")
    held = len(shell.showing()["held"])
    shell.send_key("tab")
    shell.launcher_until(lambda shown: "not here" in str(shown["saying"]))
    shown = shell.showing()
    assert len(shown["held"]) == held, f"Tab went into Foot Server: {shown['path']}"
    try:
        shell.send_key("ret")
        shell.launcher_closed()
    finally:
        shell.as_account("pkill -f 'foot --server' || true")


def test_a_step_into_programs_asks_no_rows_acts_twice_as_it_weighs_their_keys(
    shell: Shell,
) -> None:
    """Weighing the keys of Programs' rows asks each row whether it holds acts alone once.

    The foot weighs every listed row's keys, and asked each program three times whether it
    holds acts alone, each time asking its whole step (Applications.qml's startsOnly): 83 ms
    of a 116 ms step into Programs under load (vm-desktop-keeps-pace, 2026-11-14).
    """
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    shown = shell.showing()
    read, rows = cast("int", shown["actsRead"]), cast("int", shown["actsRowsAsked"])
    shell.type_in("programs", selecting="Programs")
    shell.step_in()
    shell.launcher_until(functools.partial(stands_in, "Programs"))
    launcher_on_niri.weighed_held_still(shell)
    shown = shell.showing()
    read, rows = cast("int", shown["actsRead"]) - read, cast("int", shown["actsRowsAsked"]) - rows
    shell.close_launcher()
    assert rows > 0, "weighing the rows' keys asked no row whether it holds acts alone"
    assert read == rows, f"weighing the rows' keys read {read} rows' acts for {rows} rows"


def test_a_search_names_each_rows_command_once_as_it_weighs_their_keys(shell: Shell) -> None:
    """Weighing the keys of a search's rows names each row's command once.

    The foot weighs every listed row's keys, and named each row's command about four times
    over, reading its words each time: Return's act, whether it lands and whether it ends
    something each asked, 204 times for 55 rows on "c" (2026-11-17).
    """
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    launcher_on_niri.weighed_held_still(shell)
    shown = shell.showing()
    read, rows = cast("int", shown["namedRead"]), cast("int", shown["namedRowsAsked"])
    shell.type_in("c")
    launcher_on_niri.weighed_held_still(shell)
    shown = shell.showing()
    read = cast("int", shown["namedRead"]) - read
    rows = cast("int", shown["namedRowsAsked"]) - rows
    shell.close_launcher()
    assert rows > 0, "weighing the rows' keys named no row's command"
    assert read == rows, f"weighing the rows' keys named {read} commands for {rows} rows"


def test_a_program_selected_in_programs_is_asked_its_acts_alone_once(shell: Shell) -> None:
    """Moving onto a program in Programs asks it once whether its acts lie on its row.

    The foot's keys, whether Return steps in and whether it leads elsewhere each asked the
    selection, and a program answers by making its whole step (Applications.qml's
    startsOnly): 90 times over 11 steps into Programs in the keys lab, 154 ms of their
    showing under Qt's profiler (2026-11-15).
    """
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    stepped_into_programs(shell)
    launcher_on_niri.weighed_held_still(shell)
    shown = shell.showing()
    asked, selected = cast("int", shown["actsAsked"]), (shown["selected"] or {}).get("name")
    shell.send_key("down")
    shell.launcher_until(lambda shown: (shown["selected"] or {}).get("name") != selected)
    shell.held_still()
    asked = cast("int", shell.showing()["actsAsked"]) - asked
    shell.close_launcher()
    assert asked <= 1, f"moving onto the next program asked rows' acts alone {asked} times"


# Eight makings of a kind well past the 13 ms a drawer joins its key's frame under, and eight
# well within it. Four more makings leave DEAR's median at 20 or more, and five take CHEAP's to
# the line only if each takes 13 ms, where a program's makes in 4 to 14 ms, so either stays.
DEAR = [40] * 8
CHEAP = [1] * 8


def kept_makings_file(shell: Shell) -> Path:
    """Give the file the shell keeps its drawers' makings in, making one drawer if none is."""
    state = Path(os.environ.get("XDG_STATE_HOME") or Path.home() / ".local/state")

    def found() -> list[Path]:
        return sorted(state.glob("quickshell/by-shell/*/launcher-drawers.json"))

    if not found():
        # Home's first row's drawer follows as it opens, and the launcher keeps its making as
        # it shuts.
        shell.as_account(shell.ipc("toggle"))
        shell.launcher_ready()
        shell.launcher_until(lambda shown: bool(shown["drawer"]["out"]))
        shell.close_launcher()
        deadline = time.monotonic() + 5
        while not found() and time.monotonic() < deadline:
            time.sleep(0.05)
    files = found()
    assert len(files) == 1, f"the shell keeps its drawers' makings in {files}"
    return files[0]


@contextlib.contextmanager
def taught(shell: Shell, makings: dict[str, list[int]]) -> Iterator[None]:
    """Restart the shell having made each kind's drawers as `makings` says, and as it was after.

    Whether a drawer joins its key's frame or rests for the letters is decided by its kind's
    makings, and a program's make in 4 to 14 ms here, cheap under the line or past it by the
    builder's load alone: at 13 a case about the dear way almost never saw it (#6737's
    council, 2026-11-26). So a case about one way teaches the shell which, through the file
    it reads as it starts. What it kept comes back after, so the cases after read their own.
    """
    path = kept_makings_file(shell)
    before: list[str] = []

    def teach() -> None:
        before.append(path.read_text())
        # The shell writes the file off its thread as the launcher shuts, so a stop can land
        # mid-write, and it reads what it couldn't finish as nothing (Makings.js); so does this.
        try:
            kept = cast("dict[str, Json]", json.loads(before[0] or "{}"))
        except json.JSONDecodeError:
            kept = {}
        path.write_text(json.dumps({**kept, **makings}))

    def put_back() -> None:
        if before:
            path.write_text(before[0])

    # Put back however the teaching ended, so a failed start leaves no kind taught for the
    # cases after, and the shell is started again for them.
    try:
        shell.restart(teach)
        yield
    finally:
        shell.restart(put_back)


def test_a_key_is_timed_on_its_swap_before_the_drawer_it_brings(shell: Shell) -> None:
    """A key's frame is timed as it's swapped, before the drawer waiting on that frame is made.

    What waits for a frame runs on its swap, and it ran first when it began waiting before the
    key did, so a program's drawer was made before the key was timed and its making was said as
    the key's: Joel's down onto Slack took 28 ms before the shell heard its frame swapped,
    beside Slack's drawer made in 28, and onto mpv 39 beside 39 (2026-11-13, 20:05).

    A drawer whose kind made in under 13 ms of late is made in the key's own handler instead,
    to come in the key's frame, and a program's makes in 4 to 14 ms here. So programs' kind,
    home's wherever they're listed, is taught dear first, as Slack's is on Joel's machine, and
    its drawer comes the frame after.
    """
    with taught(shell, {"home program": DEAR}):
        shell.as_account(shell.ipc("toggle"))
        shell.launcher_ready()
        stepped_into_programs(shell)
        shell.held_still()
        since = shell.mark()
        shell.send_key("down")
        shell.reported("drawer follows ", lambda _: True, since)
        shell.held_still()
        said, _ = shell.said_since(since)
        drawn = [
            line
            for line in said_once(shell, since, DRAWER_SAID).splitlines()
            if DRAWER_SAID.search(line)
        ]
        shell.close_launcher()
    assert drawn, "the drawer the key brought was never said presented"
    assert not came_in_its_keys_frame(drawn[-1]), (
        f"a program's drawer taught dear came in its key's frame, so the kind taught isn't "
        f"its: {drawn[-1]}"
    )
    order = [one for one in said if one.startswith(("drawer follows ", "key framed "))]
    assert order[:1] == ["key framed ↓"], f"the key was timed after its drawer: {order}"


def drawer_moved(was: str, shown: Json) -> bool:
    """Say whether the drawer is out on the selected row, titled otherwise than it was."""
    return drawer_on_selected(shown) and cast("Json", shown["drawer"])["title"] != was


def drawer_on_selected(shown: Json) -> bool:
    """Say whether the drawer is out, titled as the selected row is named."""
    drawer = cast("Json", shown["drawer"])
    selected = cast("Json", shown.get("selected") or {})
    return bool(drawer["shown"]) and drawer["title"] == selected.get("name")


def stands_in(place: str, shown: Json) -> bool:
    """Say whether the launcher shows the place, as the last of its path."""
    return cast("list[str]", shown["path"])[-1:] == [place]


def stepped_out(shell: Shell) -> None:
    """Step back out to home by -, and take back what was typed to step in, as Joel would."""
    before = len(shell.showing()["held"])
    shell.send_chars("-")
    shell.launcher_until(lambda shown: len(shown["held"]) < before)
    shell.on_find()
    typed = len(shell.showing()["typed"])
    if typed:
        shell.as_account("wtype " + " ".join(["-k BackSpace"] * typed))
    shell.launcher_until(lambda shown: shown["typed"] == "")


def test_a_step_home_asks_the_windows_daemon_nothing_again(shell: Shell) -> None:
    """Stepping home from a place listing no window takes no windows answer after the key.

    Windows' rows were watched only while wanted, so a step home asked the daemon for every
    window again, and its answer was refreshed a quarter second after the key, by the pace:
    253 ms from Joel's key to its frame (ZINAWZ, 2026-11-11).
    """
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    shell.type_in("emoji", selecting="Emoji")
    shell.step_in()
    assert shell.held_still(), "Emoji never held still"
    shell.normal()
    mark = shell.mark()
    stepped_out(shell)
    assert shell.held_still(), "home never held still"
    # The span a paced refresh would come in, four times over, not a wait for an end.
    time.sleep(1)
    reports, _ = shell.said_since(mark)
    shell.close_launcher()
    late = [report for report in reports if report.startswith("refresh of windows")]
    assert not late, f"the step home refreshed Windows from a new answer: {late}"


def test_a_refresh_at_home_moves_neither_the_selection_nor_the_list(shell: Shell) -> None:
    """A collection changing while home is shown leaves the selection and the list still.

    Each refresh at home was composed afresh: the selection went to the first row, the list
    to the top, and both were put back, so the highlight went its own way and the list
    stopped short of its first row (Joel, 2026-11-08).
    """
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    shown = shell.showing()
    for _ in range(len(shown["rows"]) - 1 - shown["rows"].index(shown["selected"]["name"])):
        shell.send_key("down")
    shell.launcher_until(
        lambda shown: (
            shown["selected"]["name"] == shown["rows"][-1] and shown["selected"]["settled"]
        )
    )
    before = laid_out(shell)
    selected = shell.showing()["selected"]["name"]
    since = shell.mark()
    window = shell.opened("Refresh Under Home")
    shell.reported("refreshed ", lambda _: True, since)
    after = laid_out(shell)
    now = shell.showing()["selected"]["name"]
    shell.close_launcher()
    shell.close(window)
    assert now == selected, f"the selection moved from {selected!r} to {now!r}"
    assert after["selection"]["y"] == after["selection"]["row"], (
        f"the selection was drawn off its row: {after['selection']}"
    )
    assert after["firstAt"] == before["firstAt"], (
        f"the list moved from {before['firstAt']} to {after['firstAt']}"
    )


def test_a_name_too_long_for_its_row_reads_on_while_selected(shell: Shell) -> None:
    """A name too long for its row slides to show its end while selected, and only it moves."""
    title = (
        "Edgewise notes, and a title that runs on well past the edge of the launcher row"
        " as the title of a web page often does"
    )
    long = shell.opened(title)
    short = shell.opened("Edgewise")
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    shell.type_in("edgewise")
    shell.launcher_until(lambda shown: {title, "Edgewise"} <= set(shown["rows"]))
    shell.down_to(title)

    def selected_words() -> list[Json]:
        """Give the words the selected row draws, with where each is."""
        laid = json.loads(shell.as_account(shell.ipc("laidOut")))
        return cast("list[Json]", next(row["words"] for row in laid["rows"] if row["selected"]))

    def named(words: list[Json], name: str = title) -> Json:
        # A row not selected draws its name with the letters typed brought forward, as styled
        # text, so its tags are set aside.
        return next(word for word in words if re.sub("<[^>]*>", "", word["text"]) == name)

    # Where every name starts, read from the short one, which never moves: the long one may
    # already be on its way, selected as the rows came.
    laid = json.loads(shell.as_account(shell.ipc("laidOut")))
    start = next(
        named(row["words"], "Edgewise")["x"]
        for row in laid["rows"]
        if row["reads"].startswith("Edgewise \u00b7")
    )
    rest = sorted((word["text"], word["x"]) for word in selected_words() if word["text"] != title)
    sliding = time.monotonic() + 30
    while True:
        words = selected_words()
        # Whatever else the row says holds still while its name moves.
        others = sorted((word["text"], word["x"]) for word in words if word["text"] != title)
        assert others == rest, words
        if int(named(words)["x"]) < int(start):
            break
        assert time.monotonic() < sliding, f"the long name never slid: {words}"
        time.sleep(0.1)
    # Given up by the selection, it's back at its start at once.
    away, back = ("up", "down") if shell.showing()["rows"].index(title) > 0 else ("down", "up")
    shell.send_key(away)
    shell.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Edgewise")
    laid = json.loads(shell.as_account(shell.ipc("laidOut")))
    row = next(row for row in laid["rows"] if row["reads"].startswith(title))
    assert named(row["words"])["x"] == start, row
    # Selected again, it reads on twice, then rests at its start until the selection moves
    # (VKCF3Z, 2026-11-09).
    shell.send_key(back)
    shell.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == title)

    def selected_row() -> Json:
        laid = json.loads(shell.as_account(shell.ipc("laidOut")))
        return cast("Json", next(row for row in laid["rows"] if row["selected"]))

    reading = time.monotonic() + 60
    while cast("int", selected_row()["readsDone"]) < READINGS:
        assert time.monotonic() < reading, f"the long name never read on twice: {selected_row()}"
        time.sleep(0.1)
    # Three times the half second a read rests at its start before it slides, and it's still.
    resting = time.monotonic() + 1.5
    while time.monotonic() < resting:
        row = selected_row()
        assert row["readsDone"] == READINGS, f"the long name read on again: {row}"
        assert named(row["words"])["x"] == start, f"the long name moved after two: {row}"
        time.sleep(0.1)
    shell.close_launcher()
    shell.close(long)
    shell.close(short)


# How many times a selected long name reads on before it rests at its start (Choice.qml).
READINGS = 2


def opening_said(shell: Shell, since: int) -> str:
    """Give what the shell has said since the mark, once it's said an opening's time.

    A key's time is said once the same frame is presented, in no order beside it, so what's said
    is read again the time a late key is given to land.
    """
    deadline = time.monotonic() + launcher_on_niri.WAITING.total_seconds()
    said = ""
    while time.monotonic() < deadline:
        said = shell.log.read_bytes()[since:].decode(errors="replace")
        if "launcher: opening took " in said:
            time.sleep(LATE_LANDS_S)
            return shell.log.read_bytes()[since:].decode(errors="replace")
        time.sleep(launcher_on_niri.POLL)
    unsaid = f"no opening was timed: {said[-2000:]}"
    raise AssertionError(unsaid)


def test_super_n_lists_the_notices_on_screen_on_the_one_in_front_and_return_acts(
    shell: Shell,
) -> None:
    """Super+N lists the notices on screen, on the one in front; j moves and Return acts on one.

    Notices are the launcher's rows, so its keys, acts and drawer are theirs. The one in front
    is the oldest, never the row first in Desktop, so the cursor landing on it shows the
    landing chose it. One rested off the screen is Desktop's alone, never among those on
    screen, which come in the order the line takes them. It searches only there, and - climbs
    back onto Now, where Desktop stands while no notice is critical. Again, Super+N closes.
    """
    with shell.told("rests Plover"):
        shell.as_account("notify-send --expire-time=1000 'Plover' 'rested off the screen'")
    for summary in ["Kestrel", "Lapwing"]:
        with shell.told(f"shows card {summary}"):
            shell.as_account(f"notify-send --expire-time=60000 '{summary}' 'one of three'")
    # Merlin offers to open, and says so once it's asked, as a program would open its window.
    with shell.told("shows card Merlin"):
        merlin = subprocess.Popen(
            ["notify-send", "--expire-time=60000", "--action=default=Open", "Merlin", "one of 3"],
            stdout=subprocess.PIPE,
            stdin=subprocess.DEVNULL,
            text=True,
        )
    # Home kept first, as it is by the time Joel presses it, so opening finds no home anew, and
    # warming done: each place it keeps is a search, and two kept within the moment before
    # Super+N counted 3 for its opening (a gate, 2026-11-15).
    launcher_on_niri.warmed(shell)
    shell.launcher_until(lambda shown: not shown["open"] and shown.get("kept"))
    searches = launcher_on_niri.searched(shell)
    since = shell.mark()
    shell.take_notices()
    shell.launcher_ready()
    on_screen = ["Kestrel", "Lapwing", "Merlin"]
    shell.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name") == "Kestrel"
            and shown["rows"] == on_screen
            and shown["mode"] == "normal"
        )
    )
    # It passes through where Desktop stands on the way, as a walk does, searching only where it
    # lands: searching home and For You first held all of Joel's Super+N openings past 50 ms.
    searched = launcher_on_niri.searched(shell) - searches
    assert searched == 1, f"Super+N searched {searched} times to open on the notices"
    # The steps it takes are the opening's, timed in its line, never again as steps of their own:
    # each of Joel's "\u2192 On Screen" lines timed its opening's frame a second time, the
    # slowest place in his journal (2026-11-14, 16:00 to 22:30).
    said = opening_said(shell, since)
    stepped = [line for line in said.splitlines() if '"\u2192 ' in line]
    assert stepped == [], f"Super+N's opening was timed again as a step: {stepped}"
    # Passed through, Now is listed as - climbs back onto it, Desktop among its rows.
    shell.send_chars("-")
    shell.launcher_until(lambda shown: shown["held"] == ["now"] and "Desktop" in shown["rows"])
    shell.close_launcher()
    shell.take_notices()
    shell.launcher_ready()
    shell.take_notices()
    shell.launcher_closed()
    shell.take_notices()
    shell.launcher_ready()
    shell.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Kestrel")
    for key in ["j", "j"]:
        shell.send_key(key)
    shell.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Merlin")
    shell.send_key("ret")
    try:
        asked, _ = merlin.communicate(timeout=launcher_on_niri.WAITING.total_seconds())
    except subprocess.TimeoutExpired:
        merlin.kill()
        raise
    assert asked.strip() == "default", f"Return on Merlin asked it {asked!r}, not to open"
    shell.as_account("qs ipc call notices putAway")
    shell.notices_gone()


def test_what_comes_after_the_words_are_typed_is_found_and_comes_last(shell: Shell) -> None:
    """A thing that comes after the words are typed is found, after the row selected.

    A document Drive sent once its daemon answered, after the words were typed, never showed
    until they changed: a refresh searched again only for a source whose row was shown, and
    then only among what was found before.
    """
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    shell.type_in("quokka")
    first = shell.opened("Quokka Notes")
    second = None
    try:
        shell.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Quokka Notes")
        second = shell.opened("Quokka Sketches")
        # The selection stays on its row, and what came since comes after it.
        shell.launcher_until(
            lambda shown: (
                [row for row in shown["rows"] if row.startswith("Quokka")]
                == ["Quokka Notes", "Quokka Sketches"]
                and (shown["selected"] or {}).get("name") == "Quokka Notes"
            )
        )
    finally:
        shell.close(first)
        if second is not None:
            shell.close(second)


def test_returns_place_is_weighed_for_the_rows_whats_typed_lists_now(shell: Shell) -> None:
    """Return's place on the find is as wide as the rows what's typed lists now need.

    A row no longer listed can't be named, so it holds no room: held from a letter passed,
    Sleep's ":suspend" on "s" left a gap before "\u21b5 more" once "swayimg" was typed
    (VKCF3Z, 2026-11-07). Typed on from "s", the foot leaves what typed at once leaves.
    """

    def spare(shown: Json) -> int:
        """Give how much wider Return's place is than what it says."""
        room = shown["keysRoom"]
        return cast("int", room["place"]) - cast("int", room["placeSays"])

    def on_it(shown: Json) -> bool:
        """Say whether the window is selected."""
        return (shown["selected"] or {}).get("name") == "Sandpiper Notes"

    window = shell.opened("Sandpiper Notes")
    try:
        shell.answer("sandpiper")
        shell.launcher_until(on_it)
        at_once = shell.showing()
        shell.close_launcher()
        shell.answer("s")
        passed = shell.showing()["keysRoom"]["place"]
        shell.type_in("andpiper")
        shell.launcher_until(on_it)
        typed_on = shell.showing()
        # Else no row "s" lists says a wider Return, and this proves nothing.
        assert passed > at_once["keysRoom"]["place"], (passed, at_once["keysRoom"])
        assert spare(typed_on) == spare(at_once), (at_once["keysRoom"], typed_on["keysRoom"])
    finally:
        shell.close(window)


def test_a_letter_weighs_the_keys_of_only_the_rows_it_lists_anew(shell: Shell) -> None:
    """A letter typed on weighs the foot's keys for the rows it lists that the last didn't.

    A found row is the same as long as it's found alike (launcher/Member.js's copiedAgain),
    so its keys are too. Held for what's typed, each letter weighed every listed row's keys
    again: the keys lab's "c" and "co" called keysFor 1,400 times over 20 letters, 123 ms of
    their weighing, profiled (2026-11-15).
    """
    # Windows named alike, so the letters typed on keep each of them.
    windows = [shell.opened(f"Weighed Once {name}") for name in ("Alder", "Birch", "Cedar")]
    try:
        shell.answer("weighed o")
        before = launcher_on_niri.weighed_held_still(shell)
        listed = shell.showing()["rows"]
        shell.type_in("nc")
        weighed = launcher_on_niri.weighed_held_still(shell) - before
        now = shell.showing()
        anew = [name for name in now["rows"] if name not in listed]
        kept = len(now["rows"]) - len(anew)
        # Within the twenty rows showing() names, and the windows kept, or this proves nothing.
        assert len(now["tinted"]) <= len(now["rows"]), f"more listed than named: {now['rows']}"
        assert kept >= len(windows), f"too few rows kept from {listed}: {now['rows']}"
        # One more for a row whose keys a letter may change, as the selection's are.
        assert weighed <= len(anew) + 1, (
            f"letters keeping {kept} rows weighed {weighed}, with {anew} new"
        )
    finally:
        for window in windows:
            shell.close(window)


def test_a_terminal_is_shown_a_place_by_its_names_and_a_miss_the_nearest(shell: Shell) -> None:
    """`launcher show` is answered any place by the names its steps say, and a miss the nearest.

    The shell walks from home by its own rows (Launcher.qml's shownToTerminal), each step a
    name in any case or the start of only one (launcher/Showing.js, whose cases are
    modules/quickshell/tests/showing.qml), so a terminal says what the launcher would, a thing's own
    acts apart from its rows. The launcher stays closed: a terminal's asking never opens it.
    """

    def shown(*steps: str) -> Json:
        return shown_to_terminal(shell, *steps)

    home = shown()
    names = [row["name"] for row in home["rows"]]
    assert home["in"] == [], home
    assert home["row"] is None, home
    assert home["acts"] == [], home
    assert "Machine" in names, names
    machine = shown("machine")
    assert machine["in"] == ["Machine"], machine
    assert machine["row"]["holds"], machine["row"]
    assert "Power" in [row["name"] for row in machine["rows"]], machine["rows"]
    # A step's start names the one row it starts; each row's id is its path's.
    power = shown("MACH", "pow")
    assert power["in"] == ["Machine", "Power"], power
    reboot = next(row for row in power["rows"] + power["acts"] if row["name"] == "Reboot")
    assert reboot["id"].startswith("\u203a".join(power["path"]) + "\u203a"), reboot
    assert reboot["verb"] == "reboot", reboot
    # A thing's own acts are said apart from its rows, for `launcher do` (VKCF3Z),
    # and only the row the path names says its drawer, never each row it lists.
    grinning = shown("desktop", "emoji", FIRST_GROUP, "grinning face")
    assert "Copy" in [row["name"] for row in grinning["acts"]], grinning
    assert "Copy" not in [row["name"] for row in grinning["rows"]], grinning
    # Desktop's Emoji says how many emoji it holds, counted from the history's records, though
    # it lists only Unicode's groups and has made none of the emoji inside them. A place's
    # figure is its side, as "1,914"; its count is what needs Joel.
    emoji = next(row for row in shown("desktop")["rows"] if row["name"] == "Emoji")
    figure = re.match(r"[\d,]+", str(emoji.get("side", "")))
    held = int(figure.group().replace(",", "")) if figure else 0
    assert held >= EMOJI_LISTED_LEAST, f"Desktop's Emoji said how many it holds so: {emoji}"
    assert all("drawer" not in row for row in machine["rows"]), machine["rows"]
    # A miss says where, the names it was nearest, and that nothing it was looked among is
    # still being read, so the terminal says it at once rather than asking again.
    assert shown("mahcine") == {
        "missing": "mahcine",
        "near": ["Machine"],
        "in": [],
        "reading": False,
    }
    assert shown("machine", "zebra")["in"] == ["Machine"]
    assert not shell.showing()["open"]


# What's typed for the meaning daemon's stand-in to find a part by, which no letters find.
MEANT_PART = "where the parts live"


def test_a_meaning_lands_on_a_part_inside_a_place(shell: Shell) -> None:
    """The parts a place holds are told to the meaning daemon, and one meant is found itself.

    The launcher tells the stand-in the spots its devices hold, as a screen's Resolution, as well
    as its things (tests/desktop/stand-ins/stand-in-meaning.py), and a spot it says is meant is
    added below what the letters found as typing would find it, saying where it lives and that
    it was found by its meaning. That it stands where its place would have, and is meant once by
    its name, is modules/meaning/tests/meaning.qml's to prove; this is the wiring.
    """
    runtime = Path(os.environ["XDG_RUNTIME_DIR"])
    means = runtime / "meaning-means.json"
    means.unlink(missing_ok=True)
    shell.answer(MEANT_PART)
    deadline = time.monotonic() + launcher_on_niri.WAITING.total_seconds()
    spots: list[str] = []
    while not spots:
        assert time.monotonic() < deadline, "the launcher told the meaning daemon of no spot"
        time.sleep(launcher_on_niri.POLL)
        known = runtime / "meaning-known"
        told = known.read_text().split() if known.exists() else []
        spots = [key for key in told if key.startswith("spot:")]
    typed = [row["is"] for row in shell.showing()["things"]]
    shell.close_launcher()
    spot = next((key for key in spots if key not in typed), "")
    assert spot, f"every spot told was found by its letters: {spots}"
    means.write_text(json.dumps({MEANT_PART: [spot]}))
    try:
        shell.answer(MEANT_PART)
        shell.launcher_until(lambda shown: spot in [row["is"] for row in shown["things"]])
        shown = shell.showing()
        found = [row["is"] for row in shown["things"]]
        assert found[: len(typed)] == typed, f"the meaning moved a typed row: {found}"
        for _ in range(found.index(spot) - shown["current"]):
            shell.send_key("down")
        shell.launcher_until(lambda shown: (shown["selected"] or {}).get("is") == spot)
        selected = shell.showing()["selected"]
        assert selected["note"].startswith("found by its meaning"), f"a meant part: {selected}"
        assert selected["side"] != "", f"a meant part says nothing of where it lives: {selected}"
        shell.close_launcher()
    finally:
        means.unlink(missing_ok=True)


def test_a_terminal_is_asked_before_an_ending_act_and_a_place_opens(shell: Shell) -> None:
    """`launcher do` is asked the launcher's question before an act that ends something.

    The shell walks the steps as `show` does (Launcher.qml's doneFromTerminal), and an act the
    line would ask about, or that ends something, isn't done until the terminal says it's sure:
    its answer is the launcher's own question, in the act's own words and what it closes, never
    the place's name joined to the act's. A place opens in the launcher, as the catalogue's run
    opens one.
    """

    def done(*steps: str) -> Json:
        asked = shlex.quote(json.dumps({"steps": steps, "words": "", "sure": False}))
        said = shell.as_account(f"qs -p {shell.folder} ipc call -- launcher doing {asked}")
        return cast("Json", json.loads(said))

    reboot = done("machine", "power", "reboot")
    assert reboot["in"] == ["Machine", "Power", "Reboot"], reboot
    closing = r"(, Closing (the Window|\d+ Windows))?"
    assert re.fullmatch(rf"Reboot the Machine{closing}\?", reboot["asks"]), reboot
    assert not shell.showing()["open"]
    assert done("mahcine") == {"missing": "mahcine", "near": ["Machine"], "in": []}
    machine = done("machine")
    assert machine == {"done": True, "opened": True, "in": ["Machine"]}, machine
    assert shell.showing()["open"]
    shell.close_launcher()


def test_a_terminal_does_an_act_unasked_and_an_ending_one_once_sure(shell: Shell) -> None:
    """`launcher do` does an act that ends nothing at once, and one that ends only once sure.

    Going to a window ends nothing and the : line never asks it, so it's done unasked; Float
    is asked, as :float is, since it changes the window with nothing to take it back. Close
    ends the window, so it's asked of the window first and done only by the sure ask, which
    walks the steady ids the question gave (`path`) rather than the names again, so a row
    moved since is never closed in its place (Y2YBTS's review of #2421).
    """

    def done(*steps: str, path: list[str] | None = None, sure: bool = False) -> Json:
        said = {"steps": steps, "path": path or [], "words": "", "sure": sure}
        asked = shlex.quote(json.dumps(said))
        answer = shell.as_account(f"qs -p {shell.folder} ipc call -- launcher doing {asked}")
        return cast("Json", json.loads(answer))

    def listed(title: str) -> Json | None:
        """Give the window as niri lists it, or None once it's gone."""
        titled = [one for one in shell.windows() if one["title"] == title]
        return titled[0] if titled else None

    def until(test: Callable[[], bool], what: str) -> None:
        deadline = time.monotonic() + launcher_on_niri.WAITING.total_seconds()
        while not test():
            assert time.monotonic() < deadline, f"{what} never came: {shell.windows()}"
            time.sleep(launcher_on_niri.POLL)

    def windows_listed() -> list[str]:
        """Give the names Desktop's Windows lists, as a terminal's show walks them."""
        path = shlex.quote("\u203adesktop\u203awindows")
        asked = f"qs -p {shell.folder} ipc call -- launcher show {path}"
        return [row["name"] for row in json.loads(shell.as_account(asked)).get("rows", [])]

    # Windows lists every window but the focused one, Desktop's This Window, so each acted on
    # is one the other took the focus from.
    kestrel = shell.opened("Kestrel Notes")
    plover = shell.opened("Plover Notes")
    try:
        until(lambda: "Kestrel Notes" in windows_listed(), "Kestrel Notes in Windows")
        went = done("desktop", "windows", "kestrel notes")
        assert went == {"done": True, "in": ["Desktop", "Windows", "Kestrel Notes"]}, went
        until(lambda: (listed("Kestrel Notes") or {}).get("is_focused") is True, "The focus")
        until(lambda: "Plover Notes" in windows_listed(), "Plover Notes in Windows")
        floating = done("desktop", "windows", "plover notes", "float")
        assert floating["asks"] == "Float Plover Notes?", floating
        assert (listed("Plover Notes") or {}).get("is_floating") is False, "Float was done"
        closing = done("desktop", "windows", "plover notes", "close")
        assert closing["asks"] == "Close Plover Notes?", closing
        assert listed("Plover Notes") is not None, "Close was done before it was sure"
        closed = done(path=closing["path"], sure=True)
        assert closed["done"] is True, closed
        assert closed["in"] == ["Desktop", "Windows", "Plover Notes", "Close"], closed
        until(lambda: listed("Plover Notes") is None, "Its closing")
    finally:
        for window in (kestrel, plover):
            if listed(window["title"]) is not None:
                shell.close(window)


def test_a_window_that_needs_joel_says_so_and_return_takes_him_there(shell: Shell) -> None:
    """A window niri marks urgent says it needs Joel, and going to it takes him there.

    niri marks a window urgent as it asks for the focus unfocused, as a sandboxed program's
    does opening behind on a workspace no screen shows (JMJ7J4, 2026-11-14), but no row read
    it. Its row now says "needs you" with its glyph in the alert, its drawer says since when,
    and Desktop's Windows counts it; going to it focuses it on its workspace, which clears its
    urgency and its count (VKCF3Z, 2026-11-14 [961bf73d]).
    """

    def listed(title: str) -> Json:
        return next(one for one in shell.windows() if one["title"] == title)

    def shown(path: str) -> Json:
        asked = f"qs -p {shell.folder} ipc call -- launcher show {shlex.quote(path)}"
        return cast("Json", json.loads(shell.as_account(asked)))

    def until(test: Callable[[], bool], what: str) -> None:
        deadline = time.monotonic() + launcher_on_niri.WAITING.total_seconds()
        while not test():
            assert time.monotonic() < deadline, f"{what} never came: {shell.windows()}"
            time.sleep(launcher_on_niri.POLL)

    def row() -> Json:
        return named_row(shown("\u203adesktop\u203awindows"), "Sanderling Notes") or {}

    def counted() -> str:
        return str((named_row(shown("\u203adesktop"), "Windows") or {}).get("side", ""))

    # Joel stays in the window opened last, on the first workspace, while the one asking for
    # him waits on the second.
    asking = shell.opened("Sanderling Notes")
    here = shell.opened("Turnstone Notes")
    try:
        shell.as_account(
            f"niri msg action move-window-to-workspace --window-id {asking['id']} --focus false 2"
        )
        until(lambda: "workspace 2" in row().get("side", ""), "Its row on the second")
        assert row().get("count") is None, f"a window needing nobody said so: {row()}"
        shell.as_account(f"niri msg action set-window-urgent --id {asking['id']}")
        until(lambda: row().get("count") == "needs you", "Its needing Joel")
        marked = row()
        assert marked.get("hue") == "alert", f"its glyph wasn't in the alert: {marked}"
        # Only the row a path names says its drawer.
        named = shown("\u203adesktop\u203awindows\u203asanderling notes").get("row") or {}
        facts = [one["label"] for one in (named.get("drawer") or {}).get("facts", [])]
        assert "Needs You Since" in facts, f"its drawer didn't say since when: {named}"
        assert counted() == "1 needs you", f"Desktop's Windows said {counted()!r}"
        said = {"steps": ["desktop", "windows", "sanderling notes"], "path": [], "words": ""}
        went = json.loads(
            shell.as_account(
                f"qs -p {shell.folder} ipc call -- launcher doing {shlex.quote(json.dumps(said))}"
            )
        )
        assert went.get("done") is True, f"going to it wasn't done: {went}"
        until(lambda: listed("Sanderling Notes")["is_focused"] is True, "Its focus")
        went_to = listed("Sanderling Notes")
        assert went_to["is_urgent"] is False, f"the window still needed Joel there: {went_to}"
        until(lambda: counted() != "1 needs you", "Its count letting go")
    finally:
        shell.as_account("niri msg action focus-workspace 1")
        shell.close(asking)
        shell.close(here)


def named_row(shown: Json, name: str) -> Json | None:
    """Give the row a terminal's answer shows by this name, or None."""
    return next((row for row in shown.get("rows") or [] if row["name"] == name), None)


def reading(shown: Json, start: str) -> str:
    """Give how the first row starting so reads whole, or nothing."""
    reads = [str(thing["reads"]) for thing in shown["things"]]
    return next((read for read in reads if read.startswith(start)), "")


def test_warp_says_how_its_link_stands_in_network_and_when_it_isnt_answering(
    shell: Shell,
) -> None:
    """The Work Network says how WARP's link stands after the Home Network, and a service down.

    It's Joel's word for his link to work (2026-11-11), so typing work finds it too.

    The shell runs the real warp, given the check's stand-in for warp-cli, which answers with
    the contract's files as they're written here (tests/stand-ins/warp-cli-stand-in.sh at the
    repository's root). What each figure reads as, a step in, is tests/answers.qml's, over the
    record warp prints.
    """
    said = Path(os.environ["HOME"]) / "warp-cli"
    contract = Path(os.environ["WARP_CONTRACT"])
    said.mkdir()
    for question in ("status", "tunnel-stats", "settings", "dns-stats"):
        shutil.copyfile(contract / f"{question}.json", said / f"{question}.json")
    try:
        # Network holds it after the Home Network, saying how its link stands.
        shell.as_account(shell.ipc("toggle"))
        shell.launcher_ready()
        shell.send_chars("network ")
        stands = "Work Network \u00b7 connected \u00b7 127 ms \u00b7 answer"
        shell.launcher_until(lambda shown: reading(shown, "Work Network") == stands)
        rows = shell.showing()["rows"]
        assert rows.index("Work Network") == rows.index("Home Network") + 1, rows
        shell.close_launcher()
        # Joel's word for it answers how it stands, as warp and vpn do.
        shell.answer("work")
        shell.launcher_until(
            lambda shown: (
                ((shown["selected"] or {}).get("is"), (shown["selected"] or {}).get("name"))
                == ("answer", "connected \u00b7 127 ms")
            )
        )
        shell.close_launcher()
        # Its service gone, WARP says so in its place, as "vpn" asks it too, and
        # what warp-cli's error means beside it rather than the error itself.
        (said / "status.json").unlink()
        shutil.copyfile(contract / "daemon-down.json", said / "status.error.json")
        shell.answer("vpn")
        shell.launcher_until(
            lambda shown: (
                (
                    (shown["selected"] or {}).get("name"),
                    (shown["selected"] or {}).get("note"),
                )
                == ("not answering", "its service isn't running")
            )
        )
        # Its drawer comes out, warp-cli's error as written its last fact (VKCF3Z, 2026-11-09).
        shell.launcher_until(
            lambda shown: (
                shown["drawer"]["shown"]
                and shown["drawer"]["title"] == "Work Network"
                and shown["drawer"]["facts"][-1:] == ["Error"]
            )
        )
        shell.close_launcher()
    finally:
        warp_forgotten(said)


def test_now_says_while_the_work_network_isnt_answering_and_forgets_it_once_it_is(
    shell: Shell,
) -> None:
    """Now lists the Work Network while WARP's service is down, and drops it as it answers.

    The margin holds a watch on how WARP stands always, so it's said with the launcher shut,
    gone within seconds of healing (VKCF3Z, 2026-11-11). What each state says is
    modules/quickshell/tests/work-network-ails.qml's; this is the wiring, warp to the margin to Now.
    """
    said = Path(os.environ["HOME"]) / "warp-cli"
    contract = Path(os.environ["WARP_CONTRACT"])
    said.mkdir()
    shutil.copyfile(contract / "daemon-down.json", said / "status.error.json")
    try:
        shell.as_account(shell.ipc("toggle"))
        shell.launcher_ready()
        shell.normal()
        shell.step_in()
        shell.launcher_until(
            lambda shown: (
                shown["path"][-1:] == ["Now"] and "not answering" in reading(shown, "Work Network")
            )
        )
        # Its service back, the piece stops ailing and the margin lets its mark go, as the
        # next status asked says so, its line dims in Now still held, and Now opened afresh
        # lists it no more.
        healed = shell.mark()
        shutil.copyfile(contract / "status.json", said / "status.json")
        (said / "status.error.json").unlink()
        since: list[str] = []
        deadline = time.monotonic() + launcher_on_niri.WAITING.total_seconds()
        while not healed_in(since):
            assert time.monotonic() < deadline, f"since the heal: {since}"
            time.sleep(ASKED_AGAIN)
            since = [
                report[:160]
                for report in shell.said_since(healed)[0]
                if not report.startswith("showing")
            ]
        # The held step is made again as its source refreshes, so the line dims at once;
        # it had stayed lit until something else made Now again, past 30 s in one of four
        # runs of this file (2026-11-12).
        shell.launcher_until(
            lambda shown: all(
                thing["gone"]
                for thing in shown["things"]
                if thing["reads"].startswith("Work Network")
            ),
            timeout=DIMMED_AT_ONCE,
        )
        shell.close_launcher()
        shell.as_account(shell.ipc("toggle"))
        shell.launcher_ready()
        shell.normal()
        shell.step_in()
        shell.launcher_until(
            lambda shown: shown["path"][-1:] == ["Now"] and reading(shown, "Work Network") == ""
        )
        shell.close_launcher()
    finally:
        warp_forgotten(said)


def test_a_marks_click_draws_now_from_what_was_kept_on_its_row(shell: Shell) -> None:
    """A click on a margin mark draws Now as the closed launcher kept it, landing on its row.

    It opened Now whole, as nothing but a chord's place was drawn from what was kept: Joel's
    clicks took 51 and 89 ms to the screen, 19 and 34 of it making Now again (2026-11-17, 10:07
    and 10:21). Now is kept while a look waits, and the look is cleared and the mark lit before
    the click, so its first frame lands on the mark's row, with For You's saying nothing waits,
    only if Now's rows are listed as they stand rather than as last made. The Work Network's mark,
    alone in the margin once the look's is gone, is centred along the top.
    """
    look = now_kept_while_a_look_waits(shell)
    said = Path(os.environ["HOME"]) / "warp-cli"
    contract = Path(os.environ["WARP_CONTRACT"])
    said.mkdir()
    lit = shell.mark()
    shutil.copyfile(contract / "daemon-down.json", said / "status.error.json")
    try:
        cells: list[int] = []

        def alone(rest: str) -> bool:
            drawn = json.loads(rest)
            cells[:] = [glyph["cell"] for glyph in drawn]
            return [glyph["mark"] for glyph in drawn] == ["tunnel"]

        shell.reported("sets margin ", alone, lit)
        # Said once its frame is handed to niri, so the click lands on a surface that's there.
        shell.reported("drawn margin", lambda _: True, lit)
        timed = json.loads(shell.as_account(shell.ipc("openings")))
        before = max((one["nth"] for one in timed), default=0)
        output = json.loads(shell.as_account("niri msg --json focused-output"))["logical"]
        shell.click(output["width"] // 2, cells[0] // 2)
        opened: list[Json] = []

        def came(_: Json) -> bool:
            each = json.loads(shell.as_account(shell.ipc("openings")))
            opened[:] = [one for one in each if one["nth"] > before]
            return bool(opened)

        try:
            shell.launcher_until(came)
        except AssertionError as missed:
            failed = f"the click on the mark at its cell {cells[0]} opened nothing"
            raise AssertionError(failed) from missed
        assert opened[0]["into"] == "on", f"the click didn't open Now: {opened[0]}"
        assert opened[0].get("kept"), f"the click opened Now whole: {opened[0]}"
        first = opened[0].get("shown") or {"keys": [], "at": -1}
        landed = first["keys"][first["at"]] if 0 <= first["at"] < len(first["keys"]) else None
        assert landed == "on:tunnel", f"the first frame didn't land on the mark's row: {first}"

        def filled_in(shown: Json) -> bool:
            return came(shown) and "filled" in opened[0]

        shell.launcher_until(filled_in)
        told_first, told_filled = (
            side_of(each, "on:waitingOnYou") for each in (first, opened[0]["filled"])
        )
        assert told_first == told_filled, f"For You's row said {told_first!r}, then {told_filled!r}"
        shell.launcher_until(
            lambda shown: (
                shown["path"][-1:] == ["Now"]
                and (shown["selected"] or {}).get("name") == "Work Network"
            )
        )
        shell.close_launcher()
    finally:
        look.unlink()
        warp_forgotten(said)


def now_kept_while_a_look_waits(shell: Shell) -> Path:
    """Keep Now, by its chord, while a look waits, then clear the look shut; give its file."""
    state = Path(os.environ.get("XDG_STATE_HOME") or Path.home() / ".local/state")
    looks = state / "pi/agentic-harness.pi/looks"
    looks.mkdir(parents=True, exist_ok=True)
    look = looks / "kept-waiting.json"
    # It came just now, since one three days old goes quiet (launcher/Looks.js).
    told = {"v": 1, "id": "kept-waiting", "title": "Kept Waiting", "target": "/etc/hostname",
            "kind": "file", "state": "waiting", "at": now("-u", "+%FT%TZ")}  # fmt: skip
    launcher_on_niri.laid_whole(look, json.dumps(told))
    try:
        assert "Kept Waiting" in looks_listed(shell, "Kept Waiting"), "the look was never heard"
        shell.as_account(shell.ipc("collection", "on"))
        shell.launcher_ready()
        shell.close_launcher()
        shell.launcher_until(lambda shown: not shown["open"] and shown.get("kept"))
    except BaseException:
        look.unlink()
        raise
    launcher_on_niri.laid_whole(look, json.dumps(told | {"state": "cleared"}))
    return look


def side_of(rows: Json, key: str) -> str | None:
    """Give what a row says at its right in rows said as an opening's are (rowsSaid)."""
    return dict(zip(rows["keys"], rows["sides"], strict=True)).get(key)


def healed_in(reports: list[str]) -> bool:
    """Whether the shell said the Work Network well again, and then a margin without it."""
    if "work network ails false" not in reports:
        return False
    after = reports[reports.index("work network ails false") :]
    held = [report for report in after if report.startswith("holds margin")]
    return bool(held) and "tunnel" not in held[-1].split()


def warp_said(**settings: object) -> Path:
    """Write the contract's answers where the stand-in warp-cli reads them, settings changed.

    Its disconnect answers as warp-cli's does, and the stand-in, asked it, says WARP is
    disconnected after, as WARP's service would; nothing here reaches a real WARP.
    """
    said = Path(os.environ["HOME"]) / "warp-cli"
    contract = Path(os.environ["WARP_CONTRACT"])
    said.mkdir()
    for question in ("status", "tunnel-stats", "settings", "dns-stats", "override-show"):
        shutil.copyfile(contract / f"{question}.json", said / f"{question}.json")
    written = json.loads((said / "settings.json").read_text())
    written["settings"].update(settings)
    (said / "settings.json").write_text(json.dumps(written))
    (said / "disconnect.json").write_text('"Success"\n')
    return said


def warp_forgotten(said: Path) -> None:
    """Take the stand-in's answers away, again while it writes behind the removal.

    The shell may still be having warp read as a test ends, and the stand-in writes each
    question into the folder once it sees it, so a removal can meet a file written after it
    emptied the folder (Errno 39 on 2026-11-10).
    """
    deadline = time.monotonic() + launcher_on_niri.WAITING.total_seconds()
    while said.exists():
        try:
            shutil.rmtree(said)
        except OSError as error:
            if error.errno != errno.ENOTEMPTY or time.monotonic() > deadline:
                raise


def asked(said: Path) -> list[str]:
    """Give each question the stand-in warp-cli was asked, in turn."""
    where = said / "asked"
    return where.read_text().splitlines() if where.exists() else []


def disconnect_from_its_switch(shell: Shell, *, held: bool = False) -> None:
    """Step into WARP and run :disconnect on its switch, as Joel types it.

    Held by the organisation, the switch teaches no d and writes no line, since the foot
    teaches only what works and it ends nothing, and says who holds it (VKCF3Z, 2026-11-08).
    """
    shell.answer("warp")
    shell.launcher_until(
        lambda shown: (shown["selected"] or {}).get("name") == "connected \u00b7 127 ms"
    )
    shell.step_in()
    shell.launcher_until(lambda shown: shown["rows"][:1] == ["Disconnect WARP"])
    shell.down_to("Disconnect WARP")
    shell.normal()
    if held:
        shell.launcher_until(
            lambda shown: (
                "holds the switch" in shown["footer"]
                and "d disconnect" not in shown["footer"]
                and ":disconnect" not in shown["footer"]
            )
        )
    shell.send_chars(":disconnect")
    shell.launcher_until(lambda shown: (shown["command"] or {}).get("line") == "disconnect")
    shell.send_key("ret")


def test_warps_disconnect_runs_warp_and_says_when_it_comes_back(shell: Shell) -> None:
    """WARP's :disconnect has warp disconnect it, and says when WARP, always on, comes back.

    The stand-in warp-cli writes each question it's asked, so the check sees the switch reach
    warp-cli through warp (tests/stand-ins/warp-cli-stand-in.sh).
    """
    said = warp_said()
    try:
        disconnect_from_its_switch(shell)
        shell.launcher_until(lambda _: "disconnect" in asked(said))
        shell.launcher_until(lambda shown: "comes back by itself in 2 min" in shown["footer"])
        shell.close_launcher()
    finally:
        warp_forgotten(said)


def test_a_switch_the_organisation_holds_is_left_and_says_who_holds_it(shell: Shell) -> None:
    """A WARP switch the organisation holds says who holds it, and warp-cli is never asked."""
    said = warp_said(switch_locked=True)
    try:
        with shell.told("not become warp:power: example holds the switch"):
            disconnect_from_its_switch(shell, held=True)
        assert "settings" in asked(said), asked(said)
        assert "disconnect" not in asked(said), asked(said)
        shell.close_launcher()
    finally:
        warp_forgotten(said)


# How often a person might ask a terminal again: more often than WARP is read afresh while
# shown (Warp.qml's fresh), so a read kept from outside is said before it's asked again.
ASKED_AGAIN = 0.5
# How soon a line gone from a step still held dims, once its source has said so: at once, with
# room for a loaded builder, and well short of what else makes the step again.
DIMMED_AT_ONCE = timedelta(seconds=5)
# How soon the margin says an episode of degraded WARP: the minute it must gather, the daemon's
# next judging five seconds on, and room for a loaded builder.
EPISODE_SAID = timedelta(seconds=100)


def terminal_shown_until(shell: Shell, test: Callable[[Json], object], *steps: str) -> Json:
    """Ask `launcher show` again, as a person would, until what it's told passes the test.

    A terminal's ask is answered at once, so a provider still asking says so, and what it
    hears is said to the next ask (Warp.qml's askedOutside).
    """
    deadline = time.monotonic() + launcher_on_niri.WAITING.total_seconds()
    shown = shown_to_terminal(shell, *steps)
    while not test(shown):
        assert time.monotonic() < deadline, f"a terminal was never shown what's wanted: {shown}"
        time.sleep(ASKED_AGAIN)
        shown = shown_to_terminal(shell, *steps)
    return shown


def test_now_says_a_degraded_work_network_within_its_line_and_steps_to_its_row(
    shell: Shell,
) -> None:
    """A degraded episode's line in Now holds its figures within it, and Return lands on WARP.

    The warp daemon says an episode once a minute of degraded has gathered
    (modules/warp/src/degraded.rs), so this is the one case here that waits a minute, a part
    of its own (modules/quickshell/checks/core.nix); it's the only one where the margin's
    piece carries its context, WARP's figures, the hour and what's behind it, some 170
    characters under Now's line, which the launcher's own measure of what broke out of its
    card judges (Y2YBTS, 2026-11-16). Return then steps into Network onto the Work Network's
    row, walked into from home as a machine's row walks into Computer (pieces/WorkNetwork.qml).
    """
    said = warp_said()
    contract = Path(os.environ["WARP_CONTRACT"])
    try:
        mark = shell.mark()
        shutil.copyfile(contract / "status.degraded.json", said / "status.json")
        deadline = time.monotonic() + EPISODE_SAID.total_seconds()
        while "work network ails true" not in shell.said_since(mark)[0]:
            assert time.monotonic() < deadline, "the margin never said WARP's episode"
            time.sleep(ASKED_AGAIN)
        shell.as_account(shell.ipc("toggle"))
        shell.launcher_ready()
        shell.normal()
        shell.step_in()
        shell.launcher_until(
            lambda shown: (
                shown["path"][-1:] == ["Now"] and "degraded" in reading(shown, "Work Network")
            )
        )
        line = next(name for name in shell.showing()["rows"] if name.startswith("Work Network"))
        shell.down_to(line)
        # The Date's drawer goes in as the cursor leaves it, and its facts break out of it while
        # it goes (2026-11-16), so the card is judged once it's gone, the line alone in view.
        shell.launcher_until(lambda shown: not (shown["drawer"] or {}).get("out"))
        broke = json.loads(shell.as_account(shell.ipc("contained")))
        assert broke == [], f"what broke out of Now's card: {broke}"
        # Return walks from home into Network, as deep as Now is held, so it's pressed rather
        # than stepped in, whose wait is for the launcher to hold one more.
        shell.send_key("ret")
        shell.launcher_until(
            lambda shown: (
                shown["path"][-1:] == ["Network"]
                and (shown["selected"] or {}).get("name") == "Work Network"
            )
        )
        shell.close_launcher()
    finally:
        warp_forgotten(said)


def test_a_terminal_is_shown_warps_state_health_and_round_trips_drawer(shell: Shell) -> None:
    """`launcher show network work network` says WARP's state and Health, and Round Trip's.

    A terminal names it by its name, as every step is (Showing.js's step).

    What warp prints for a terminal's ask is kept for the next while the launcher stays
    closed, and each read after sparks as it does while shown, so once two are in, Round
    Trip's drawer says whose figures they are. The launcher never opens.
    """
    said = warp_said()
    try:
        # Its rows, not its side, say the link is read: the side says connected at once, from
        # how the margin heard WARP stand.
        warp = terminal_shown_until(
            shell,
            lambda shown: any(row["name"] == "Health" for row in shown.get("rows") or []),
            "network",
            "work network",
        )
        assert warp["in"] == ["Network", "Work Network"], warp
        assert "connected" in warp["row"]["side"], warp["row"]
        health = next(row for row in warp["rows"] if row["name"] == "Health")
        assert health["side"] == "healthy", health
        trip = terminal_shown_until(
            shell,
            lambda shown: "drawer" in (shown["row"] or {}),
            "network",
            "work network",
            "round trip",
        )
        assert trip["row"]["drawer"]["eyebrow"] == "via WARP \u00b7 tunnel", trip["row"]
        assert not shell.showing()["open"]
    finally:
        warp_forgotten(said)


def test_the_work_networks_row_has_a_drawer_saying_how_work_stands(shell: Shell) -> None:
    """The Work Network's row in Network has its own drawer: whose, a lead, then why.

    Every row has a drawer (VKCF3Z, 2026-11-12). Its lead says how the link stands and its
    round trip, and for how long only where the daemon saw it begin, as it did here if an
    earlier case changed the link; warp-cli's own words for it are its nerdy bits, and its
    status, which State says, is none (WarpDrawer.js; VKCF3Z, f8984db0).
    """
    said = warp_said()
    try:
        warp = terminal_shown_until(
            shell,
            lambda shown: bool(((shown.get("row") or {}).get("drawer") or {}).get("body")),
            "network",
            "work network",
        )
        drawer = warp["row"]["drawer"]
        assert drawer["eyebrow"] == "via WARP \u00b7 tunnel", str(drawer)
        assert drawer["title"] == "Work Network", str(drawer)
        body = str(drawer["body"])
        assert body.startswith("Connected to work"), str(drawer)
        assert body.endswith("; a round trip takes 127 ms."), str(drawer)
        facts = {fact["label"]: fact["value"] for fact in drawer["facts"]}
        assert facts["State"] == "connected", str(facts)
        assert facts["Round Trip"] == "127 ms", str(facts)
        assert "Status" not in facts, str(facts)
        assert "Reason" not in facts, str(facts)
        nerd = {bit["label"]: bit["value"] for bit in drawer.get("nerd") or []}
        assert nerd.get("Reason") == "NetworkHealthy", str(drawer)
        assert "Tunnel Mode" in nerd, str(drawer)
    finally:
        warp_forgotten(said)


def test_the_work_network_says_whats_waiting_on_the_way_as_the_shaper_hears_it(
    shell: Shell,
) -> None:
    """The Work Network's Waiting and Upload say what the shaper heard, until it stops.

    warp reads the shaper's status.json with each read of the link (its waiting.rs), and the
    place says how long past the way at rest a question through the VPN took, after the
    Round Trip, then the rate the shaper holds the upload to, noted why in its words. The
    shaper's file goes as it stops: Waiting is unknown, never none, and Upload goes. A
    terminal's ask is answered from what was read last, an earlier case's maybe, so each
    reading is waited for.
    """
    said = warp_said()
    shaper = Path(os.environ["HOME"]) / "shaper"
    shaper.mkdir()
    status = shaper / "status.json"
    shutil.copyfile(Path(os.environ["WARP_CONTRACT"]) / "shaper-status.json", status)
    try:
        warp = terminal_shown_until(
            shell,
            lambda shown: (named_row(shown, "Waiting") or {}).get("side") == "75 ms",
            "network",
            "work network",
        )
        names = [row["name"] for row in warp["rows"]]
        assert names[names.index("Round Trip") + 1] == "Waiting", names
        waiting = named_row(warp, "Waiting")
        assert waiting is not None, names
        assert waiting["note"] == "103 ms through it \u00b7 28 ms at rest", str(waiting)
        assert names[names.index("Waiting") + 1] == "Upload", names
        upload = named_row(warp, "Upload")
        assert upload is not None, names
        assert upload["side"] == "7.42 Mb/s", str(upload)
        assert upload["note"].startswith("the VPN waited 75 ms inside its tunnel"), str(upload)
        status.unlink()
        stopped = terminal_shown_until(
            shell,
            lambda shown: (named_row(shown, "Waiting") or {}).get("side") == "unknown",
            "network",
            "work network",
        )
        waiting = named_row(stopped, "Waiting")
        assert waiting is not None, stopped
        assert waiting["note"] == "the shaper isn't running", str(waiting)
        assert named_row(stopped, "Upload") is None, str(stopped["rows"])
    finally:
        shutil.rmtree(shaper, ignore_errors=True)
        warp_forgotten(said)


def test_network_says_how_warp_stands_before_its_link_is_read(shell: Shell) -> None:
    """The Work Network's row says how WARP stands at once, as the margin heard it, not "...".

    The margin watches how WARP stands all the time (WarpState.qml), so the row waiting on
    the link's first read says it rather than an ellipsis: degraded here, and with no round
    trip beside it, since none is read yet. Opening and closing the launcher first forgets
    what an earlier case's terminal ask kept (Warp.qml's onOpenChanged). The margin's own
    piece says nothing of a moment's degraded, since the warp daemon says an episode only
    once it has held a minute (modules/warp/src/degraded.rs), so the case waits on what
    WarpState.qml says it heard, which reads nothing of the link either.
    """
    said = warp_said()
    try:
        shell.as_account(shell.ipc("toggle"))
        shell.launcher_ready()
        shell.close_launcher()
        mark = shell.mark()
        status = json.loads((said / "status.json").read_text())
        status["reason"] = "NetworkDegraded"
        (said / "status.json").write_text(json.dumps(status))
        deadline = time.monotonic() + launcher_on_niri.WAITING.total_seconds()
        while not any(
            report.startswith("warp stands") and "degraded" in report.lower()
            for report in shell.said_since(mark)[0]
        ):
            assert time.monotonic() < deadline, "the margin never heard WARP degraded"
            time.sleep(ASKED_AGAIN)
        network = shown_to_terminal(shell, "network")
        work = next(row for row in network["rows"] if row["name"] == "Work Network")
        assert work["side"] == "degraded", work
    finally:
        warp_forgotten(said)


def test_a_terminal_disconnects_warp_once_sure_and_connects_it_unasked(shell: Shell) -> None:
    """`launcher do` asks before WARP's Disconnect, as its card does, and connects it unasked.

    Disconnect WARP ends the link, so the terminal is asked the launcher's own question and
    warp-cli hears nothing until it's sure; Connect WARP ends nothing, so it's done at once.
    The shell walks the same rows the card draws (Launcher.qml's doneFromTerminal), so there's
    nothing of WARP's own to wire; this proves it reaches warp-cli through warp.
    """

    def done(*steps: str, path: list[str] | None = None, sure: bool = False) -> Json:
        said = {"steps": steps, "path": path or [], "words": "", "sure": sure}
        asked_for = shlex.quote(json.dumps(said))
        answer = shell.as_account(f"qs -p {shell.folder} ipc call -- launcher doing {asked_for}")
        return cast("Json", json.loads(answer))

    def rows_say(name: str) -> Callable[[Json], bool]:
        return lambda shown: name in [row["name"] for row in shown.get("rows", [])]

    said = warp_said()
    (said / "connect.json").write_text('"Success"\n')
    try:
        terminal_shown_until(shell, rows_say("Disconnect WARP"), "network", "work network")
        asking = done("network", "work network", "disconnect warp")
        # The question names the place, in Joel's word for what he'd lose.
        assert asking["asks"].startswith("Disconnect Work Network"), asking
        assert asking["asks"].endswith("?"), asking
        assert "disconnect" not in asked(said), "warp-cli was told before the terminal was sure"
        disconnected = done(path=asking["path"], sure=True)
        assert disconnected["done"] is True, disconnected
        assert disconnected["in"] == ["Network", "Work Network", "Disconnect WARP"], disconnected
        shell.launcher_until(lambda _: "disconnect" in asked(said))
        terminal_shown_until(shell, rows_say("Connect WARP"), "network", "work network")
        connected = done("network", "work network", "connect warp")
        assert connected == {
            "done": True,
            "in": ["Network", "Work Network", "Connect WARP"],
        }, connected
        shell.launcher_until(lambda _: "connect" in asked(said))
    finally:
        warp_forgotten(said)


def test_the_home_network_is_found_by_eeros_name_and_by_whats_on_it(shell: Shell) -> None:
    """Typing eero finds the Home Network, and a device's name finds the device in it.

    Joel reaches his home network by the name on its boxes as by its own (QEST-20261111-PN2VZC),
    and the place was found only by its name and its noun, so "eero" found only a node whose
    model says it. The eero daemon's stand-in holds the contract's network
    (tests/desktop/stand-ins/stand-in-eero.py), whose console is named game console 1.
    """
    for typed, name in [("eero", "Home Network"), ("game console", "game console 1")]:
        shell.answer(typed)
        shell.launcher_until(functools.partial(selecting, name))
        shell.close_launcher()


def test_the_home_network_draws_the_rows_the_eero_daemon_serves(shell: Shell) -> None:
    """The Home Network watches the eero daemon's rows in the launcher's words, and draws them.

    The place made every row itself from eero's whole record, again on each change, where the
    daemons are the launcher's data layer (QEST-20261111-PN2VZC). The stand-in serves the rows
    the daemon makes (modules/eero/tests/rows.json) and says each watch of them in its log, so
    the place is seen to ask for them in words from the store, and a node is found among them.
    """
    log = shell.log.parent / "eero.log"
    watched = "watched eero's rows in /nix/store/"
    shell.launcher_until(lambda _: watched in log.read_text(errors="replace"))
    shell.answer("study")
    shell.launcher_until(functools.partial(selecting, "Study"))
    shell.close_launcher()


def test_each_eero_heads_the_devices_on_it(shell: Shell) -> None:
    """Each eero starts a group of its own in the Home Network, its devices a step in under it.

    Joel (2026-11-13): "home network is confusing because endpoints are not obviously grouped
    under their base station". The daemon serves each device a step in, and the place drew it
    flush with its eero, with a gap above the first eero alone. The stand-in's nodes are Den,
    Study and Hall, and every row between two of them is a device on the one above.
    """
    nodes = {"Den", "Study", "Hall"}
    shell.answer("eero")
    shell.launcher_until(functools.partial(selecting, "Home Network"))
    shell.step_in()
    shell.launcher_until(lambda shown: "Study" in shown["rows"])
    rows = sorted(laid_out(shell)["rows"], key=lambda row: row["at"])
    named = [(row["reads"].split(" \u00b7 ")[0], row) for row in rows]
    under = [row for at, (name, row) in enumerate(named) if at > 0 and named[at - 1][0] in nodes]
    assert under, f"no row follows an eero: {[name for name, _ in named]}"
    assert all(row["depth"] == 1 for row in under), f"a device sits flush with its eero: {under}"
    assert all(row["depth"] == 0 for name, row in named if name in nodes), str(named)
    # A device follows a device a row's pitch down; an eero stands a beat further from the row
    # above it, whichever eero it is.
    steps = [
        (name, row["band"] - named[at - 1][1]["band"])
        for at, (name, row) in enumerate(named)
        if at > 0
    ]
    pitch = min(step for _, step in steps)
    apart = [name for name, step in steps if name in nodes and step <= pitch]
    assert not apart, f"these eeros stand no further from the rows above than a row: {apart}"
    shell.close_launcher()


def test_the_home_network_never_rests_with_a_row_cut(shell: Shell) -> None:
    """The Home Network's list, a gap above each eero, ends on a whole row where a key leaves it.

    It's drawn in home's card, which keeps one shape, so its gaps left its last row cut at the
    foot. Its view rests on a row's top, draws only the rows wholly in it, and draws the row a
    move lands on (Y2YBTS, 2026-11-20 [95c35e88]).
    """
    shell.answer("eero")
    shell.launcher_until(functools.partial(selecting, "Home Network"))
    shell.step_in()
    shell.launcher_until(lambda shown: "Study" in shown["rows"])
    shell.normal()

    # inView reads whole bands and not what's hidden, so a view that hid a row it holds whole
    # would draw fewer than it spans.
    def rested(shown: Json) -> bool:
        return bool(
            shown["cut"] == []
            and shown["restsOnARow"]
            and len(shown["gutter"]["numbers"]) == shown["drawnRows"]
            and shown["drawnRows"] >= shown["inView"]["bottom"] - shown["inView"]["top"] + 1
        )

    def parts(shown: Json) -> tuple[object, ...]:
        return (
            shown["count"],
            shown["current"],
            shown["cut"],
            shown["restsOnARow"],
            shown["gutter"]["numbers"],
            shown["drawnRows"],
            shown["inView"],
        )

    # Read again, so a state the opening only passed through doesn't count.
    shell.launcher_until(rested)
    opened = shell.showing()
    assert rested(opened), parts(opened)
    none_drawn_cut(shell, "opened")
    count = opened["count"]
    assert count > 2 * opened["drawnRows"], parts(opened)
    # Study, an eero, has a gap above it, so the selection's ground laid on it shows one put
    # on the gap rather than its band, which none of the other landings has. The opening's
    # rows name the list's first twenty, drawn or not, Study among them; that it stands a gap
    # below the row above is test_each_eero_heads_the_devices_on_it's. Its landing starts from
    # the top itself, so it holds wherever it stands in the list. The selection has two
    # writers: pickOn, as Qt lays the selected row, and pick's fillAt, as a key selects it.
    # Landing on Study from afar lays it anew, so pickOn has the last word; stepping off it
    # and back while it's drawn lays nothing, so fillAt's place is the one left standing.
    assert "Study" in opened["rows"], (opened["rows"], *parts(opened))
    study = opened["rows"].index("Study")
    assert study > 0, (opened["rows"], *parts(opened))
    landings = [
        ("7j", 7),
        ("5j", 12),
        ("G", count - 1),
        ("gg", 0),
        (f"gg{study}j", study),
        ("k", study - 1),
        ("j", study),
    ]
    for key, at in landings:
        # The cursor's number, absolute, is drawn only beside a row the view draws; a landing
        # on Study's index is held to Study itself, so a list that moved under it fails.
        def landed(shown: Json, at: int = at) -> bool:
            return bool(
                shown["current"] == at
                and shown["gutter"]["cursor"] == str(at + 1)
                and (at != study or selecting("Study", shown))
                and rested(shown)
            )

        sent = time.monotonic() * 1000
        shell.send_chars(key)
        shell.launcher_until(landed)
        shown = shell.showing()
        assert landed(shown), (key, *parts(shown))
        none_drawn_cut(shell, key)
    # The last step back onto Study lays nothing, so fillAt alone places the selection; the
    # key's first frame must have it on Study's drawn row, which a fillAt laid from the list's
    # top rather than Study's gap misses by the gap. `sent` is the last landing's, so that
    # landing has to be the step back, or a landing added after it would be read instead.
    assert landings[-1] == ("j", study), f"the last landing isn't the step back: {landings}"
    stepped = key_presented_since(shell, sent)
    assert stepped["onRow"], f"stepping back onto Study, the selection was off its row: {stepped}"
    shell.close_launcher()


def key_presented_since(shell: Shell, sent: float) -> Json:
    """Wait for the launcher to present a key it heard since `sent`, monotonic ms, and give it.

    A key is told by when the shell heard it rather than by the last presented, since the key
    before's frame may still be on its way to niri's screen.
    """
    deadline = time.monotonic() + launcher_on_niri.WAITING.total_seconds()
    while time.monotonic() < deadline:
        presented = json.loads(shell.as_account(shell.ipc("keysPresented")))
        since = [one for one in presented if one["heard"] >= sent - CLOCKS_APART_MS]
        if since:
            return cast("Json", since[0])
        time.sleep(launcher_on_niri.POLL)
    msg = f"no key heard since {sent:.0f} was presented"
    raise AssertionError(msg)


def none_drawn_cut(shell: Shell, where: str, *, selection_in_view: bool = True) -> None:
    """Fail, naming them, where a box cuts a row's words or the selection down, or it's off its row.

    Read from where each text's line is drawn and what clips it (Launcher.qml's drawnTexts),
    and the selection's ground from where it's drawn (groundCut), not from the rule's own
    extents, so a row the rule took for whole and drew cut is caught; and the selection against
    where its row's band is drawn (laidOut's selection), so one wholly in view on the wrong row
    is caught too. A key keeps the selection in view; the wheel moves the view alone, so it
    may leave the selected row out of it (`selection_in_view`), when only a drawn one is held
    to its row. Where the view rests is `restsOnARow`'s, from the rows' tops. A row's name
    reading on is clipped across, never down, so it's no cut.
    """
    cut = rows_drawn_cut(shell)
    assert cut == [], f"{where}: drawn cut: {cut}"
    shown = shell.showing()
    assert not shown["groundCut"], (
        f"{where}: the selected row's ground is cut",
        shown["current"],
        shown["scrolled"],
    )
    selection = laid_out(shell)["selection"]
    assert selection is not None or not selection_in_view, f"{where}: the selected row isn't drawn"
    if selection is not None:
        assert selection["y"] == selection["row"], (
            f"{where}: the selection is off its row: {selection}"
        )


def rows_drawn_cut(shell: Shell) -> list[str]:
    """Give the words of the card's rows that a box cuts at their top or foot.

    Qt lays a row out again only before its next frame, and a loaded builder can be slow to
    draw one, so words read between a change and that frame stand where they were: each read
    comes after two frames asked for, and two reads must agree on where each stands down the
    card. Across is left out, as a long name reads on.
    """
    deadline = time.monotonic() + 10
    last = None
    while True:
        two_frames_drawn(shell, deadline)
        card = json.loads(shell.as_account(shell.ipc("drawnTexts")))[0]
        listed = [text for text in card if "/QQuickListView" in text["from"]]
        standing = [(text["said"], text["ink"]["y"], text["cutDown"]) for text in listed]
        if standing == last:
            break
        assert time.monotonic() < deadline, f"the rows' words never stood still: {standing}"
        last = standing
    assert listed, f"no row's words were drawn: {card}"
    return [text["said"] for text in listed if text["cutDown"]]


def two_frames_drawn(shell: Shell, deadline: float) -> None:
    """Ask the launcher for a frame twice, each once the last is drawn.

    The first may have been under way before it was asked for, so only the second is sure to
    lay out what changed before the ask (as agents-on-niri-test.py's framed).
    """
    for _ in range(2):
        asked = int(shell.as_account(shell.ipc("drawAFrame")))
        while shell.frames() <= asked:
            assert time.monotonic() < deadline, "no frame drawn when asked for"
            time.sleep(launcher_on_niri.POLL)


def home_network_in_normal(shell: Shell) -> None:
    """Open the Home Network's list, a gap above each eero, rested whole, and go to normal."""
    shell.answer("eero")
    shell.launcher_until(functools.partial(selecting, "Home Network"))
    shell.step_in()
    shell.launcher_until(lambda shown: "Study" in shown["rows"] and shown["restsOnARow"])
    shell.normal()


def test_the_home_network_rests_whole_where_the_wheel_leaves_it(shell: Shell) -> None:
    """The wheel leaves the Home Network's list on a row's top, no row drawn cut at its foot.

    The wheel moves the view by pixels, so it can stop inside a row; once it stops, the list
    rests on the top of the first row at or below where it stopped (Launcher.qml's
    onMovementEnded, Headings.rested). The selection stays on the first row, which the wheel
    can take out of view, and where the turn back up rests depends on the rows' heights, so
    whether the selection is in view isn't asked here.
    """
    home_network_in_normal(shell)
    card = shell.showing()["card"]
    wlrctl = os.environ["WLRCTL"]
    shell.as_account(f"{wlrctl} pointer move -100000 -100000")
    middle = ((card["left"] + card["right"]) // 2, (card["top"] + card["foot"]) // 2)
    shell.as_account(f"{wlrctl} pointer move {middle[0]} {middle[1]}")
    # Down and back up by a turn that isn't a whole number of rows, so the view can stop
    # inside one.
    for by in [70, -70]:
        was = shell.showing()["scrolled"]
        # Said, so a wait that runs out names which way the wheel went and from where.
        print(f"the wheel by {by} from {was}")
        shell.as_account(f"{wlrctl} pointer scroll {by} 0")

        # Once the view has stopped, so one the wheel only passed through doesn't count.
        def rested(shown: Json, was: int = was, by: int = by) -> bool:
            return bool(
                not shown["moving"]
                and (shown["scrolled"] - was) * by > 0
                and shown["restsOnARow"]
                and shown["cut"] == []
            )

        shell.launcher_until(rested)
        shown = shell.showing()
        print(f"the wheel by {by} rested at {shown['scrolled']}")
        assert rested(shown), (by, shown["scrolled"], shown["restsOnARow"], shown["cut"])
        none_drawn_cut(shell, f"the wheel by {by}", selection_in_view=False)
    shell.close_launcher()


def test_the_home_network_draws_nothing_cut_once_its_find_is_gone(shell: Shell) -> None:
    """Once a find in the Home Network is taken away, G lands on its last row with nothing cut.

    A find drops headings and sets its own rows, so the list rests whole only on the place's
    own list (Headings.restsWhole); once the letters are gone, it's that list again, and G
    lands on its last row drawn whole. Where G leaves the view's top isn't asked: it was seen
    resting off a row's top, a fault of its own. Where a find leaves the view as it goes is a
    case of its own, not this one.
    """
    home_network_in_normal(shell)
    whole = shell.showing()["count"]
    shell.type_in("st")
    shell.launcher_until(lambda shown: shown["count"] < whole)
    for _ in "st":
        shell.send_key("backspace")
    shell.launcher_until(
        lambda shown: shown["typed"] == "" and shown["count"] == whole and shown["restsOnARow"]
    )
    shell.normal()
    shell.send_chars("G")

    # The cursor's number is drawn only beside a row the view draws. Where the view rests isn't
    # asked: here G was seen to land off a row's top (20 Nov), with no row drawn cut, and why
    # is the Sat 21 Nov follow-up's.
    def landed(shown: Json) -> bool:
        return bool(
            shown["current"] == whole - 1
            and shown["gutter"]["cursor"] == str(whole)
            and shown["cut"] == []
        )

    shell.launcher_until(landed)
    shown = shell.showing()
    assert landed(shown), (shown["current"], shown["gutter"], shown["cut"])
    none_drawn_cut(shell, "after the find, at the foot")
    shell.close_launcher()


def test_the_home_networks_row_has_the_drawer_the_eero_daemon_serves(shell: Shell) -> None:
    """The Home Network's row in Network has its own drawer: whose, a lead, then its figures.

    Every row has a drawer (VKCF3Z, 2026-11-12), and the eero daemon serves this one beside
    its rows (its place.rs), so the stand-in serving the daemon's records has the place say
    how the home network stands rather than the names of everything it holds.
    """
    home = terminal_shown_until(
        shell,
        lambda shown: bool(((shown.get("row") or {}).get("drawer") or {}).get("facts")),
        "network",
        "home network",
    )
    drawer = home["row"]["drawer"]
    assert drawer["eyebrow"] == "via eero \u00b7 network", str(drawer)
    assert drawer["title"] == "Home Network", str(drawer)
    expected = (
        "Connected to the internet, last tested at 370 Mb/s down, with 3 nodes online"
        " and 15 devices on now."
    )
    assert drawer["body"] == expected, str(drawer)
    facts = {fact["label"]: fact["value"] for fact in drawer["facts"]}
    assert facts["Provider"] == "Example ISP", str(facts)
    assert facts["Nodes"] == "3 nodes online", str(facts)


def test_a_profiles_drawer_is_the_one_the_eero_daemon_serves(shell: Shell) -> None:
    """A profile's drawer says what it blocks and how many devices it lists, as served.

    The daemons are the launcher's data layer (QEST-20261111-PN2VZC), so the eero daemon makes
    a profile's drawer (its drawers.rs) and serves it beside the row that names it, which the
    stand-in serves as the daemon does (modules/eero/tests/rows.json). It holds a glance, at
    most six facts, each device in it being the step in's (Y2YBTS, 2026-11-13).
    """
    profile = terminal_shown_until(
        shell,
        lambda shown: bool(((shown.get("row") or {}).get("drawer") or {}).get("facts")),
        "network",
        "home network",
        "profile 2",
    )
    drawer = profile["row"]["drawer"]
    assert drawer["title"] == "profile 2", str(drawer)
    facts = {fact.get("label"): fact.get("value") for fact in drawer["facts"]}
    assert facts["Blocks"] == "nothing", str(facts)
    assert facts["Devices"] == "3 listed", str(facts)
    assert len(drawer["facts"]) <= GLANCE, str(drawer)
    assert not any(fact.get("heading") for fact in drawer["facts"]), str(drawer)


def test_a_nodes_drawer_is_the_one_the_eero_daemon_serves(shell: Shell) -> None:
    """A node's drawer says how it stands and when it was heard, worded as it's drawn.

    The eero daemon makes a node's drawer (its drawers/node.rs) and serves it beside the node's
    row, a glance of at most six facts, each time one the shell words at the minute it draws,
    as a terminal is shown it.
    """
    den = terminal_shown_until(
        shell,
        lambda shown: bool(((shown.get("row") or {}).get("drawer") or {}).get("facts")),
        "network",
        "home network",
        "Den",
    )
    drawer = den["row"]["drawer"]
    assert drawer["title"] == "Den", str(drawer)
    facts = {fact.get("label"): fact.get("value") for fact in drawer["facts"]}
    assert facts["Backhaul"] == "Wi-Fi from Hall over 5 GHz high \u00b7 5 of 5 bars", str(facts)
    assert str(facts["Last Heard"]).endswith(" ago"), str(facts)
    assert len(drawer["facts"]) <= GLANCE, str(drawer)


def test_a_devices_drawer_is_the_one_the_eero_daemon_serves(shell: Shell) -> None:
    """A device's drawer says how well it's heard, what it is and where, as served.

    The eero daemon makes a device's drawer (its drawers/device.rs) and serves it beside the
    device's row, a glance of at most six facts, its moves and events the step in's.
    """
    tablet = terminal_shown_until(
        shell,
        lambda shown: bool(((shown.get("row") or {}).get("drawer") or {}).get("facts")),
        "network",
        "home network",
        "tablet 1",
    )
    drawer = tablet["row"]["drawer"]
    assert drawer["title"] == "tablet 1", str(drawer)
    assert drawer["eyebrow"] == "on Den \u00b7 Wi-Fi", str(drawer)
    facts = {fact.get("label"): fact.get("value") for fact in drawer["facts"]}
    assert str(facts["Signal"]).endswith(" dBm"), str(facts)
    assert facts["Node"] == "Den \u00b7 eero Pro 6", str(facts)
    assert len(drawer["facts"]) <= GLANCE, str(drawer)


def test_the_internets_drawer_is_the_one_the_eero_daemon_serves(shell: Shell) -> None:
    """The internet's drawer says how fast and near it is and the last thing that happened.

    The eero daemon makes the internet's drawer (its drawers/internet.rs) and serves it beside
    the internet's row, its times sent for the shell to word as it draws them. It holds six
    facts: its 57 took up to 77 ms making on Joel's machine (2026-11-12), past the launcher's
    50, so the rest are the step in's (Y2YBTS, 2026-11-13).
    """
    internet = terminal_shown_until(
        shell,
        lambda shown: bool(((shown.get("row") or {}).get("drawer") or {}).get("facts")),
        "network",
        "home network",
        "internet",
    )
    drawer = internet["row"]["drawer"]
    assert drawer["title"] == "Internet", str(drawer)
    assert drawer["eyebrow"] == "via eero \u00b7 internet", str(drawer)
    facts = {fact.get("label"): fact.get("value") for fact in drawer["facts"]}
    assert facts["Public Address"] == "203.0.113.1", str(facts)
    assert len(drawer["facts"]) == GLANCE, str(drawer)
    last = drawer["facts"][-1]
    assert last["value"] == "The nodes updated to v7.17.4-5. They were on v7.17.3-9.", str(last)
    assert isinstance(last["label"], str), str(last)
    assert last["label"], str(last)


def test_the_internet_holds_the_acts_the_eero_daemon_does(shell: Shell) -> None:
    """A step into the internet runs a speed test and turns its switch by asking the daemon.

    The shell ran the eero program for each, through niri, and heard nothing when eero refused
    (QEST-20261111-PN2VZC). The daemon now serves them as rows inside the internet and does
    them itself; its stand-in says each act asked of it in its log. Each row reads as it becomes
    from Return, the speed test testing and the switch off, as the daemon says it does, and the
    switch's own :set, typed inside the internet, runs there rather than stepping in again.
    """
    log = shell.log.parent / "eero.log"
    shell.answer("internet")
    shell.launcher_until(functools.partial(selecting, "Internet"))
    shell.step_in()
    acts = ["Run a Speed Test", "Optimize for Conferencing and Gaming"]
    shell.launcher_until(lambda shown: shown["rows"][:2] == acts)
    # Its step lists every figure, Provider among them, past the six its drawer holds.
    shell.launcher_until(lambda shown: "Provider" in shown["rows"])
    for row, line, acted, reads in [
        (acts[0], "speedtest", "acted speedtest", "Run a Speed Test · testing"),
        (acts[1], "set nosqm", "acted sqm off", "Optimize for Conferencing and Gaming · off"),
    ]:
        shell.down_to(row)
        shell.normal()
        shell.send_chars(f":{line}")
        shell.launcher_until(
            # Ruff's B023 fears a late binding; the wait ends within this turn of the loop.
            lambda shown: (shown["command"] or {}).get("line") == line  # noqa: B023
        )
        shell.send_key("ret")
        shell.launcher_until(
            lambda shown: any(
                # Ruff's B023 fears a late binding; the wait ends within this turn of the loop.
                one["reads"].startswith(reads)  # noqa: B023
                for one in shown["things"]
            )
        )
        # Ruff's B023 fears a late binding; the wait ends within this turn of the loop.
        shell.launcher_until(lambda _: acted in log.read_text(errors="replace"))  # noqa: B023
    held = shell.showing()["held"]
    assert held == ["Internet"], f"the switch's :set stepped into the internet again: {held}"
    shell.close_launcher()


def selecting(name: str, shown: Json) -> bool:
    """Say whether the launcher has the row so named selected."""
    return bool((shown["selected"] or {}).get("name") == name)


def test_a_held_arrow_makes_one_drawer_once_it_rests(shell: Shell) -> None:
    """Arrows pressed quicker than the drawer follows make it once, for where they rest.

    A session's or a program's drawer was made on each key's own path, 56 to 122 ms a key in a
    VM (2026-11-09), so the highlight waited on it (Joel, 2026-11-08: "the highlight doesn't
    always perfectly match my keyboard interactions").
    """
    windows = [shell.opened(f"Drawn {at}") for at in range(1, 5)]
    try:
        shell.as_account(shell.ipc("toggle"))
        shell.launcher_ready()
        typed = shell.mark()
        shell.type_in("drawn")
        shell.reported("drawer follows ", lambda _: True, typed)
        shell.held_still()
        since = shell.mark()
        # Three in one go, as a held arrow's repeat sends them, quicker than the drawer follows:
        # sent while the shell is held stopped, so a loaded builder spacing wtype's keys apart
        # can't make them three rests (two of RTOIMQ's runs, 2026-11-09).
        with shell.frozen():
            shell.as_account("wtype -k Down -k Down -k Down")
        rested = shell.showing()["selected"]
        shell.reported("drawer follows ", lambda _: True, since)
        shell.held_still()
        said, _ = shell.said_since(since)
        followed = [report for report in said if report.startswith("drawer follows ")]
        assert len(followed) == 1, f"the drawer was made for each key: {followed}"
        assert shell.showing()["selected"] == rested, rested
    finally:
        shell.close_launcher()
        for window in windows:
            shell.close(window)


def drawer_trace(said: list[str]) -> list[str]:
    """Give each drawer followed, and each showing as what's typed, selected and drawn beside."""
    trace = []
    for one in said:
        if one.startswith(("drawer follows ", CHEAP_AFTER_LETTER)):
            trace.append(one)
        elif one.startswith("showing "):
            shown = json.loads(one[len("showing ") :])
            drawer = shown["drawer"]
            trace.append(
                f"{shown['typed']!r} on {(shown['selected'] or {}).get('name')!r}: drawer"
                f" {'out' if drawer['out'] else 'in'} {drawer['title']!r} for {drawer['row']!r}"
                f" timed {drawer['timed']}"
            )
    return trace


LETTER_TYPED = re.compile(r"letter typed (\d+) ms after the one before, resting (\d+)")
# What the shell says before a drawer it judged cheap follows its letter rather than resting.
CHEAP_AFTER_LETTER = "letter's drawer is cheap: "


def drawers_under_the_rest(said: list[str], *, before_the_first: bool = True) -> list[str]:
    """Give each dear drawer followed before a letter the shell heard quicker than its rest.

    wtype's letters are 50 ms apart, but a loaded builder spaced them 175 to 395 ms apart as
    the shell heard them, past the 160 ms rest, and a drawer made between them then is right
    (Y2YBTS, 2026-11-14, ed99cdc6 and 5dc9f833; typing again three times still failed in four
    gates). So a drawer is judged by the gap before the letter after it: one followed before
    a letter under the rest didn't wait for the letters. Those before the first letter count
    too, since home's drawer was out before it, unless `before_the_first` is false: a cheap
    kind's drawer rightly joins the key's frame of a first letter, which no letters came before.
    One the shell judged cheap follows its letter rather than resting, made long before the
    next (Pacing.drawerRestsForLetters), so it's left out.
    """
    wrong: list[str] = []
    waiting: list[str] = []
    cheap: str | None = None
    first = True
    for one in said:
        if one.startswith(CHEAP_AFTER_LETTER):
            cheap = one[len(CHEAP_AFTER_LETTER) :]
        elif one.startswith("drawer follows "):
            if cheap is None or one != f"drawer follows {cheap}":
                waiting.append(one)
            cheap = None
        elif typed := LETTER_TYPED.fullmatch(one):
            # The first letter's gap is from whatever was typed before the mark.
            if (first and before_the_first) or int(typed[1]) < int(typed[2]):
                wrong.extend(waiting)
            waiting = []
            first = False
    return wrong


def typed_from_home(shell: Shell, text: str, until: Callable[[int], None]) -> list[str]:
    """Type the text from home, its first row's drawer followed, and say what came of it.

    `until` waits from the mark for what the typing should end on.
    """
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    try:
        shell.on_find()
        # Home's first row's drawer follows as it opens, before any letter.
        shell.launcher_until(lambda shown: shown["drawer"]["out"] and selecting("Now", shown))
        shell.held_still()
        since = shell.mark()
        shell.send_chars(text)
        until(since)
        said, _ = shell.said_since(since)
    finally:
        shell.close_launcher()
    return said


def test_letters_typed_quicker_than_the_rest_make_no_dear_drawer(shell: Shell) -> None:
    """Letters typed faster than they rest make a dear drawer once, for where the typing stops.

    Joel types letters 100 to 150 ms apart, and each letter that moved the selection had its
    drawer made right after its frame, so the next letter waited behind it: "ca" 198 ms behind
    Chrome's drawer, "calend" 272 ms behind Google Calendar's first (2026-11-13, 06:14).
    No drawer comes in its letter's own frame, however cheap, so typing onto a program shows
    it; each letter makes its row again, and its drawer was made again each time.
    Only a dear kind, or one not yet made, rests: a cheap one follows its letter, so home's
    programs are taught dear first, as Chrome's and Google Calendar's were.
    Only the drawers followed before the last letter are judged: once the letters rest, a
    loaded builder can list Foot's row again, and its drawer rightly follows the new row
    (75a49d9587e9, the net).
    """

    def footed(since: int) -> None:
        shell.launcher_until(lambda shown: shown["typed"] == "foot")
        shell.reported("drawer follows program:foot", lambda _: True, since)
        shell.held_still()

    with taught(shell, {"home program": DEAR}):
        said = typed_from_home(shell, "foot", footed)
    assert f"{CHEAP_AFTER_LETTER}program:foot" not in said, (
        f"Foot's drawer, taught dear, was judged cheap: {drawer_trace(said)}"
    )
    last = max(at for at, report in enumerate(said) if report.startswith("letter typed "))
    between = drawers_under_the_rest(said)
    after = [report for report in said[last:] if report.startswith("drawer follows ")]
    gaps = [one for one in said if one.startswith("letter typed ")]
    assert between == [], f"a drawer was made between letters: {drawer_trace(said)}, {gaps}"
    assert after[:1] == ["drawer follows program:foot"], (
        f"Foot's drawer never followed the letters: {drawer_trace(said)}"
    )


@pytest.mark.parametrize("cheap", [False, True], ids=["dear", "cheap"])
def test_a_drawer_resting_for_the_letters_names_the_row_selected(
    shell: Shell, *, cheap: bool
) -> None:
    """While letters rest, the drawer out names the row selected rather than the one before.

    The drawer waits for the letters before it's made, and kept the row before's facts beside
    the new highlight all the while, so typing "calend" would show Chrome's drawer by Google
    Calendar for the whole word (VKCF3Z, c842849d). Typing a program's name from home moves
    the selection from Now, whose drawer is out, to the program, whose drawer never comes in its
    letter's frame, and before that drawer follows the drawer out must say the program's name.
    One cheap enough to follow its letter rather than rest names it by being made, so the first
    showing after it is judged instead, and it never rests: in the frame after its letter, or
    in its key's frame once a loaded builder has spaced the letters past the rest. A program's
    makes in 4 to 14 ms here, either side of the line by the builder's load, so home's programs
    are taught each way in turn.
    """

    def footed(since: int) -> None:
        shell.reported("drawer follows program:foot", lambda _: True, since)
        if cheap:
            # Its timing is kept a frame after it's made and said in a showing after that, and
            # each letter can follow it again, so the whole word is waited for, held still.
            shell.launcher_until(lambda shown: shown["typed"] == "foot")
            shell.held_still()

    with taught(shell, {"home program": CHEAP if cheap else DEAR}):
        said = typed_from_home(shell, "foot", footed)
    follow = said.index("drawer follows program:foot")
    judged = said[follow:] if cheap else said[:follow]
    shown = [json.loads(one[len("showing ") :]) for one in judged if one.startswith("showing ")]
    # How Foot's drawers came (in its key's frame, the frame after its letter or after
    # resting), as the showings after it follows time them.
    timed = [one["drawer"].get("timed") or {} for one in shown]
    came = [one.get("came", "") for one in timed if one.get("key") == "program:foot"]
    # Taught dear, it's never judged cheap after its letter. Taught cheap, it's said cheap only
    # while letters still come, which a loaded builder's spacing can end first, joining its
    # key's frame unsaid, so it's judged by never resting for them, and by joining only the
    # frame of a letter heard at or past the rest (#6737's council, 2026-11-26).
    judged_cheap = f"{CHEAP_AFTER_LETTER}program:foot" in said[:follow]
    rested = [how for how in came if how.startswith("after resting")]
    joined = drawers_under_the_rest(said, before_the_first=False)
    assert (bool(came) and not rested and not joined) if cheap else not judged_cheap, (
        f"Foot's drawer, taught {'cheap' if cheap else 'dear'}, was judged otherwise: "
        f"{drawer_trace(said)}; came {came!r}; joined under the rest {joined!r}"
    )
    on_foot = [one for one in shown if one["drawer"]["out"] and selecting("Foot", one)]
    assert on_foot, f"the drawer was never out beside Foot: {drawer_trace(said)}"
    assert on_foot[0 if cheap else -1]["drawer"]["title"] == "Foot", (
        f"the drawer named another row: {drawer_trace(said)}"
    )


def test_an_opening_is_timed_from_the_chord_that_asked_for_it(shell: Shell) -> None:
    """An opening is timed from the chord that asked for it, as launcher-say says it ran.

    Joel feels the time from his press, and a shell busy with other work hears the ask late,
    which timing from the ask can't show (WCJQSG, 2026-11-10: "to its frame" isn't what he
    feels; he feels how long until it's usable). So a chord said to have run a while ago
    makes an opening at least that long from it, and the shell says it heard that chord late,
    since one heard late shows its thread was held (Joel, 2026-11-12: Super+E heard 17 s late).
    """
    since = shell.mark()
    pressed = time.time() - CHORD_AGO.total_seconds()
    with socket.socket(socket.AF_UNIX) as said:
        said.connect(str(Path(os.environ["XDG_RUNTIME_DIR"]) / "launcher.sock"))
        said.sendall(f"toggle @{pressed:.6f}\n".encode())
    shell.launcher_ready()
    deadline = time.monotonic() + launcher_on_niri.WAITING.total_seconds()
    felt = -1
    while felt < 0 and time.monotonic() < deadline:
        openings = json.loads(shell.as_account(shell.ipc("openings")))
        felt = openings[-1].get("felt", -1) if openings else -1
        time.sleep(launcher_on_niri.POLL)
    ago = CHORD_AGO.total_seconds() * 1000
    assert felt >= ago, f"the opening was timed {felt} ms from a chord {ago:.0f} ms before it"
    with shell.log.open("rb") as log:
        log.seek(since)
        logged = log.read().decode(errors="replace")
    assert "launcher: heard a chord " in logged, shell.tail()


def test_an_opening_is_usable_only_once_the_shell_is_free_after_its_chord(shell: Shell) -> None:
    """An opening is usable only once the shell's thread answers on time, however soon it drew.

    Joel's openings logged 437 ms to their first frame and showed him the launcher 7 s after his
    chord, once the shell's thread came free (Joel, 2026-11-10: "It takes 7 seconds for me to see
    the launcher window"). So a shell held stopped just after a chord says the opening wasn't
    usable until at least that long after it.
    """
    with shell.frozen():
        # Sent once the shell is stopped, so a quick shell can't answer it in the moment before
        # it's held; the socket keeps it until it goes on, and the chord carries its own time.
        chord_now(time.time())
        # How long the shell is held, as its thread was held after Joel's chords.
        time.sleep(HELD.total_seconds())
    shell.launcher_ready()
    deadline = time.monotonic() + launcher_on_niri.WAITING.total_seconds()
    usable = -1
    while usable < 0 and time.monotonic() < deadline:
        openings = json.loads(shell.as_account(shell.ipc("openings")))
        usable = openings[-1].get("usable", -1) if openings else -1
        time.sleep(launcher_on_niri.POLL)
    held = HELD.total_seconds() * 1000
    assert usable >= held, f"the opening was usable {usable} ms after a shell held {held:.0f} ms"


def chord_now(chord: float) -> None:
    """Send the launcher's chord, said at `chord`, to its socket as niri's bind does."""
    with socket.socket(socket.AF_UNIX) as said:
        said.connect(str(Path(os.environ["XDG_RUNTIME_DIR"]) / "launcher.sock"))
        said.sendall(f"toggle @{chord:.6f}\n".encode())


def screen() -> bytes:
    """Give niri's screen as it stands, through its screencopy, as raw pixels."""
    return subprocess.run(
        ["grim", "-t", "ppm", "-"],
        check=True,
        capture_output=True,
    ).stdout


def changed(before: bytes, after: bytes) -> float:
    """Give how much of the screen differs between two of its pictures, from 0 to 1.

    Compared a stretch of pixels at a time, since a byte at a time took a second a picture,
    which put niri's screen a second behind the time it's held to.
    """
    if len(before) != len(after):
        return 1.0
    stretches = range(0, len(before), STRETCH)
    differ = sum(before[at : at + STRETCH] != after[at : at + STRETCH] for at in stretches)
    return differ / len(stretches)


def still_screen() -> bytes:
    """Give niri's screen once two pictures in a row agree, so what moves next is what's asked.

    The launcher holds the keys before its opening is drawn whole, and on a loaded builder a
    picture taken then caught the card still coming in: its next frame was read as the key's, 325
    ms before the key's own frame began (198c91f70d3c, 2026-11-12). Still is judged as a change
    is, by CARD_SHARE, and given up on at WAITING with the last picture.
    """
    deadline = time.monotonic() + launcher_on_niri.WAITING.total_seconds()
    was = screen()
    while time.monotonic() < deadline:
        now = screen()
        if changed(was, now) <= CARD_SHARE:
            return now
        was = now
    return was


def first_changed(before: bytes, deadline: float) -> tuple[float, float]:
    """Give when niri's screen first differed from `before`, and when it last didn't.

    Both on the monotonic clock in ms. The first is read as the picture that differed is taken,
    before it's compared, so it's when the screen already showed the change, never later by the
    comparing; -1 when it never did. The second is when the last picture that still matched began
    to be taken, so the change can't have been presented before it; -1 when the first differed.
    """
    unchanged = -1.0
    while time.monotonic() < deadline:
        began = time.monotonic() * 1000
        now = screen()
        taken = time.monotonic() * 1000
        if changed(before, now) > CARD_SHARE:
            return taken, unchanged
        unchanged = began
    return -1.0, unchanged


def prints(picture: bytes) -> list[int]:
    """Give a picture of niri's screen as a hash a stretch, compared as `changed` compares."""
    return [hash(picture[at : at + STRETCH]) for at in range(0, len(picture), STRETCH)]


def apart(one: list[int], other: list[int]) -> float:
    """Give how much of the screen differs between two pictures' hashes, from 0 to 1."""
    if len(one) != len(other):
        return 1.0
    return sum(mine != theirs for mine, theirs in zip(one, other, strict=True)) / len(one)


def settled_shown(
    before: bytes, said: Callable[[], list[Json]], deadline: float
) -> tuple[float, float, list[Json]]:
    """Give when niri's screen first showed what a change left, and what the shell said presented.

    Also when the screen last still showed `before`. Pictures are taken until the shell says the
    change presented and two taken after agree, as `still_screen` judges, so the last is the
    screen it left. The change was shown by the first picture showing that screen, rather than
    the first differing from `before`: on a loaded builder a frame of the shell's, late to niri,
    changed the screen 123 ms after a key's press and 245 before the key's own frame began, and
    was taken for the key's (2026-11-15). The times are read as `first_changed` reads them, on
    the monotonic clock in ms, -1 when there's none. The shell is asked in a thread of its own,
    so asking doesn't space the pictures out, and each picture is kept as its hashes, so a slow
    change costs little memory.
    """
    presented: list[Json] = []

    def asked() -> None:
        while not presented and time.monotonic() < deadline:
            presented.extend(said())
            time.sleep(launcher_on_niri.POLL)

    asking = threading.Thread(target=asked, daemon=True)
    asking.start()
    was = prints(before)
    # Each picture's start, when it was taken, its hashes, and whether the change had been said
    # presented before it began, so only pictures that must show it are judged settled.
    taken: list[tuple[float, float, list[int], bool]] = []
    while time.monotonic() < deadline:
        known = bool(presented)
        began = time.monotonic() * 1000
        now = prints(screen())
        taken.append((began, time.monotonic() * 1000, now, known))
        if known and len(taken) > 1 and taken[-2][3] and apart(taken[-2][2], now) <= CARD_SHARE:
            break
    asking.join()
    left = taken[-1][2] if taken else was
    shown = next((at for _, at, now, _ in taken if apart(now, left) <= CARD_SHARE), -1.0)
    unchanged = max(
        (began for began, at, now, _ in taken if 0 <= at < shown and apart(now, was) <= CARD_SHARE),
        default=-1.0,
    )
    return shown, unchanged, presented


def presented_after_unchanged(at: float, unchanged: float, what: str) -> None:
    """Fail when `what` is said presented at `at` before a picture still without it was taken.

    niri's screencopy copies what the shell has committed, which niri presents only after, so a
    picture begun after the time said must show it. It's judged by order alone, never by how soon
    a picture came, which a loaded builder stretches past any bound (1036 ms against 1000 on
    2026-11-11).
    """
    assert unchanged < 0 or unchanged <= at + CLOCKS_APART_MS, (
        f"{what} was said presented at {at:.0f},"
        f" yet niri's screen didn't show it at {unchanged:.0f}"
    )


def test_an_opening_is_presented_when_niri_first_shows_the_card(shell: Shell) -> None:
    """An opening is timed from its chord to the frame niri showed, which niri's screen agrees on.

    A frame the shell drew isn't one niri showed (Joel, 2026-11-10: "we need 50ms launch ... that
    is time from trigger to visible on the screen"), so the launcher times each opening to niri's
    presentation feedback. With the shell held stopped just after a chord, niri can show nothing
    new, so the time is at least that long; and niri's own screen first shows the card no sooner
    than the frame before the time said (COPIED_BEFORE_PRESENTED), and no picture after it lacks
    the card.
    """
    before = still_screen()
    chord = time.time()
    chord_at = time.monotonic() * 1000
    with shell.frozen():
        # Sent once the shell is stopped, as above: sent before it, one gate's shell presented
        # the card 107 ms after the chord, before the hold began (2026-11-14).
        chord_now(chord)
        # How long the shell is held, as its thread was held after Joel's chords.
        time.sleep(HELD.total_seconds())
    deadline = time.monotonic() + launcher_on_niri.WAITING.total_seconds()
    shown, unchanged = first_changed(before, deadline)
    assert shown >= 0, "niri's screen never showed the card"
    seen = shown - chord_at
    shell.launcher_ready()
    presented = -1
    while presented < 0 and time.monotonic() < deadline:
        openings = json.loads(shell.as_account(shell.ipc("openings")))
        presented = openings[-1].get("presented", -1) if openings else -1
        time.sleep(launcher_on_niri.POLL)
    held = HELD.total_seconds() * 1000
    assert presented >= held, f"the opening was presented {presented} ms after a {held:.0f} ms hold"
    lead = COPIED_BEFORE_PRESENTED.total_seconds() * 1000
    assert presented <= seen + lead, (
        f"said presented at {presented} ms; niri's screen showed it at {seen:.0f}"
    )
    presented_after_unchanged(chord_at + presented, unchanged, "the opening")


def test_a_key_is_presented_as_niri_first_shows_what_it_found(shell: Shell) -> None:
    """A key is timed from its press to the frame niri showed with it, as niri's screen agrees.

    Joel's bar is every key's change visible within 50 ms (2026-11-10), so each key is timed to
    niri's presentation feedback for the frame that drew it: niri's screen first shows what the
    key found no sooner than the frame before the time said (COPIED_BEFORE_PRESENTED), and no
    picture after it lacks it. Its frame's sync and swap are timed on the render thread, since
    the shell hears a swap only once it's free, which made Joel's "meet" say 77 ms drawing with
    3 rows new (2026-11-11).
    """
    behind = shell.opened("Ruff Notes")
    try:
        shell.as_account(shell.ipc("toggle"))
        shell.launcher_ready()
        before = still_screen()
        since = shell.mark()

        def held() -> None:
            # A change not the key's lands first, as a frame of the shell's late to niri did
            # on a loaded builder 123 ms after the press and 245 before the key's own frame
            # began (2026-11-15): a window behind the card is widened, which foot draws while
            # the shell is held, and the hold outlasts the lead so the two can't be confused.
            with shell.frozen():
                shell.as_account("wtype w")
                shell.as_account(f"niri msg action set-window-width --id {behind['id']} 100%")
                time.sleep(2 * COPIED_BEFORE_PRESENTED.total_seconds())

        # Held in a thread of its own, so the screen is pictured while the shell is held.
        holding = threading.Thread(target=held, daemon=True)
        deadline = time.monotonic() + launcher_on_niri.WAITING.total_seconds()
        holding.start()
        seen, unchanged, presented = settled_shown(
            before,
            lambda: json.loads(shell.as_account(shell.ipc("keysPresented"))),
            deadline,
        )
        holding.join()
    finally:
        shell.close(behind)
    assert seen >= 0, "niri's screen never showed what the key found"
    assert presented, "the key was never said presented"
    key = presented[-1]
    lead = COPIED_BEFORE_PRESENTED.total_seconds() * 1000

    def keys_lines() -> list[str]:
        """Give the shell's lines of the key, read only should it fail, to say what it took."""
        said = shell.log.read_bytes()[since:].decode(errors="replace")
        return [line for line in said.splitlines() if 'launcher: "w"' in line]

    assert key["from"] <= key["at"] <= seen + lead, (
        f"said {key}; niri's screen changed at {seen:.0f}; its line: {keys_lines()}"
    )
    presented_after_unchanged(key["at"], unchanged, "the key")
    # Its frame's sync and swap are read on the render thread as they happen, never when the
    # shell hears of them, so they fall between its frame's start and niri showing it.
    stamps = [key.get("begun"), key.get("synchronized"), key.get("swapped"), key.get("at")]
    times = [float(stamp) for stamp in stamps if stamp is not None]
    assert len(times) == len(stamps), f"its frame began, synced, swapped and was shown at {stamps}"
    assert times == sorted(times), f"its frame began, synced, swapped and was shown at {stamps}"
    # And its line says its drawing so, the three adding up to it but for rounding.
    said = shell.log.read_bytes()[since:].decode(errors="replace")
    drew = DRAWING_SPLIT.search(said)
    lines = [line for line in said.splitlines() if 'launcher: "w" took' in line]
    assert drew, f"its line never said its drawing apart: {lines}"
    assert any(FRAMES_BEFORE.search(line) for line in lines), (
        f"its line never said the frames begun since it was heard: {lines}"
    )
    drawing, syncing, rendering, heard = (int(part) for part in drew.groups())
    assert abs(syncing + rendering + heard - drawing) <= ROUNDED, (
        f"{drawing} ms drawing, said as {syncing} syncing, {rendering} rendering"
        f" and {heard} before the shell heard it"
    )
    shell.close_launcher()
    # The close says the key's press, swap and screen on its clock as its record holds them: a
    # wiring check, since the record's own times are proven above and the wording in pacing.qml.
    # wtype's key carries no time (keytimes.cpp), so its record says so whatever the line reads;
    # how many frames later niri answered has no such fact here, and is read from the record.
    assert key["stamped"] is False, f"wtype's key carries no press time, yet its record is {key}"
    closed = said_once(shell, since, KEYS_CLOCK)
    clock = KEYS_CLOCK.search(closed)
    assert clock, "the close never said its keys' clock"
    assert any(says_its_clock(one, key) for one in clock.group(1).split("; ")), (
        f"its clock said {clock.group(0)}; its key was {key}"
    )


# How long niri is held just after a daemon's line, well past Joel's 50 ms bar.
NIRI_HELD = timedelta(seconds=1)
# How far the shell's time of a line may read before the test's, both whole ms of one clock: the
# shell reads its line by the realtime clock, moved onto the monotonic one (Hearing.js).
CLOCKS_APART_MS = 2
# How many rows a change adds, so it changes well over CARD_SHARE of niri's screen.
ADDED = 6


class StandIn:
    """Serve a daemon's records, as the quests or machine daemon does, and tell each a change.

    It answers the shell's daemon interface on the socket the account's systemd holds for the
    daemon, which nothing else serves while it stands in: hello, a watch with every record held
    of what it watches, and any other ask with none. One that `searches`, as the quests daemon
    does, answers a search too, finding by name what it holds and the rows it doesn't serve
    (`unserved`), which come with the answer, as an ended quest's do.
    """

    def __init__(self, path: Path, *, later: tuple[str, ...] = (), searches: bool = False) -> None:
        """Listen on `path`, holding nothing yet, answering a watch of each `later` once let."""
        self.path = path
        self.held: dict[str, Json] = {}
        self.searches = searches
        self.unserved: dict[str, Json] = {}
        # The watches answered only once let (StandIn.let), as a connection of the shell's that
        # comes back seconds after the others when its daemon's socket is made again (H6EDLC's
        # traces, 2026-11-13, e02a43bf).
        self.later = {watched: threading.Event() for watched in later}
        # Each client's watches, by the id it asked each with, and what each watches
        # (StandIn.theirs): a place's rows as "all", the keys it's about to draw as "ahead".
        self.clients: dict[socket.socket, dict[str, str]] = {}
        # Every client that came, watching or not, so each is let go when it stops: one left
        # open, as a connection that only said hello, kept the shell talking to a stand-in
        # whose socket was gone, and the next case's never heard its watch (RCTRF8, 2026-11-12).
        self.came: list[socket.socket] = []
        self.lock = threading.Lock()
        path.parent.mkdir(mode=0o700, exist_ok=True)
        self.listening = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.listening.bind(str(path))
        self.listening.listen()
        threading.Thread(target=self.accept, daemon=True).start()

    @staticmethod
    def row(order: int, name: str) -> Json:
        """Give a quest's row as the quests daemon serves it, holding only what Quests reads."""
        ident = f"QEST-20261111-KES{order:03}"
        return {
            "key": f"row:quests:{ident}",
            "kind": "row",
            "row": {
                "order": order,
                "place": "quests",
                "thing": f"quest:{ident}",
                "kind": "quest",
                "name": name,
                "glyph": "pending",
                "hue": "program",
                "side": [],
                "note": ident,
                "reads": "",
                "unfinished": True,
                "held": {
                    "id": ident,
                    "kind": "quest",
                    "status": "active",
                    "priority": "active",
                    "updated": "2026-11-11",
                    "folder": f"/quests/{ident}",
                    "open": True,
                },
            },
        }

    @staticmethod
    def watched(asked: Json) -> str:
        """Give what a watch asks for: "keys:" its keys, its one kind, "place:" a place's, "" all.

        A place's ready rows watch a list of kinds, its rows and those beside them, narrowed to
        the place (ReadyRows.qml), as Power's rows and headline are watched of the machine daemon.
        Keys named narrow it to them whatever the kind, as the machine daemon's watch does.
        """
        if "keys" in asked:
            return "keys:" + " ".join(asked["keys"])
        kind = asked.get("kind")
        if isinstance(kind, str):
            return kind
        return f"place:{asked['place']}" if "place" in asked else ""

    @staticmethod
    def theirs(watching: str, record: Json) -> bool:
        """Give whether a client watching `watching` is told of `record`."""
        key = str(record["key"])
        if watching.startswith("keys:"):
            return key in watching.removeprefix("keys:").split(" ")
        if watching.startswith("place:"):
            # A served record's key names its place second, "row:power:charge", "head:power",
            # and a drawer is told only to a watch naming it (the machine daemon's APART).
            if record.get("kind") == "drawer":
                return False
            return key.split(":")[1] == watching.removeprefix("place:")
        return watching in ("", record.get("kind"))

    def accept(self) -> None:
        """Answer each client that connects, for as long as the run lasts."""
        while True:
            try:
                client, _ = self.listening.accept()
            except OSError:
                return
            with self.lock:
                self.came.append(client)
            threading.Thread(target=self.serve, args=(client,), daemon=True).start()

    def serve(self, client: socket.socket) -> None:
        """Answer one client, a line for each ask, until it goes."""
        with client.makefile("r") as lines:
            for line in lines:
                asked = json.loads(line)
                if asked.get("ask") == "hello":
                    self.write(client, {"id": "hello", "version": 1, "state": "answering"})
                elif asked.get("ask") == "watch" and asked.get("id") in ("all", "ahead"):
                    self.write(client, {"id": asked["id"]}, watching=self.watched(asked))
                elif asked.get("ask") == "search" and self.searches:
                    self.write(client, {"id": asked.get("id"), **self.found(asked)})
                elif asked.get("ask") == "unwatch":
                    with self.lock:
                        self.clients.get(client, {}).pop(str(asked.get("id")), None)
                    self.write(client, {"id": asked.get("id")})
                else:
                    self.write(client, {"id": asked.get("id"), "records": []})

    def found(self, asked: Json) -> Json:
        """Give a search's answer: each row whose name has a word each word typed begins.

        Those held first, then those it doesn't serve, each of which comes with its row.
        """
        typed = str(asked.get("words", "")).lower().split()
        with self.lock:
            served = [
                (key, rec["row"]) for key, rec in self.held.items() if rec.get("kind") == "row"
            ]
            rows = served + list(self.unserved.items())

        def finds(row: Json) -> bool:
            said = str(row.get("name", "")).lower().split()
            return all(any(one.startswith(word) for one in said) for word in typed)

        keys = [key for key, row in rows if finds(row)]
        return {
            "keys": keys,
            "rows": {key: self.unserved[key] for key in keys if key in self.unserved},
            "found": len(keys),
            "grade": 0 if keys else 3,
            "words": asked.get("words", ""),
        }

    def write(self, client: socket.socket, said: Json, *, watching: str | None = None) -> None:
        """Send one line to a client, a watch with every record held, which it's told of after.

        A watch gets only what it watches (StandIn.theirs). Under the lock, so no change is told
        between a watch's records and its being told.
        """
        if watching in self.later:
            self.later[watching].wait(launcher_on_niri.WAITING.total_seconds())
        with self.lock:
            if watching is not None:
                records = [record for record in self.held.values() if self.theirs(watching, record)]
                said = {**said, "records": records}
                self.clients.setdefault(client, {})[str(said["id"])] = watching
            client.sendall((json.dumps(said) + "\n").encode())

    def told(self, changed: list[Json], gone: list[str]) -> float:
        """Tell each client watching a change of its kind, and give when, in monotonic ms."""
        with self.lock:
            for record in changed:
                self.held[record["key"]] = record
            for key in gone:
                self.held.pop(key, None)
            at = time.monotonic() * 1000
            for client, watches in list(self.clients.items()):
                for ident, watched in list(watches.items()):
                    theirs = [record for record in changed if self.theirs(watched, record)]
                    said = {"id": ident, "changed": theirs, "gone": gone}
                    try:
                        client.sendall((json.dumps(said) + "\n").encode())
                    except OSError:
                        # A connection the shell let go, as Daemon.qml does once it's refused.
                        del self.clients[client]
                        break
        return at

    def watching(self, watched: str, deadline: float) -> bool:
        """Give whether a client watches `watched` by `deadline`, so what it's told is heard."""
        while time.monotonic() < deadline:
            with self.lock:
                if any(watched in watches.values() for watches in self.clients.values()):
                    return True
            time.sleep(launcher_on_niri.POLL)
        return False

    def ahead(self, key: str, deadline: float, *, watched: bool = True) -> bool:
        """Give whether a client watches `key` ahead by `deadline`, or none, if not `watched`."""
        while time.monotonic() < deadline:
            with self.lock:
                asked = [
                    watching.removeprefix("keys:").split(" ")
                    for watches in self.clients.values()
                    for watching in watches.values()
                    if watching.startswith("keys:")
                ]
            if any(key in keys for keys in asked) == watched:
                return True
            time.sleep(launcher_on_niri.POLL)
        return False

    def let(self, watched: str) -> None:
        """Answer the watches of `watched` held back, as their connection comes back at last."""
        self.later[watched].set()

    def close(self) -> None:
        """Stop listening, let every client go and take the socket away, as a daemon stopped."""
        for held_back in self.later.values():
            held_back.set()
        self.listening.close()
        with self.lock:
            for client in self.came:
                # Shut first, since a client's own thread blocks reading it, and a bare close
                # leaves the shell's end open until that read ends.
                with contextlib.suppress(OSError):
                    client.shutdown(socket.SHUT_RDWR)
                client.close()
            self.came.clear()
            self.clients.clear()
        self.path.unlink(missing_ok=True)


@pytest.fixture(scope="module")
def quests() -> Iterator[StandIn]:
    """Give Quests' daemon, stood in for, holding one quest, served until the run ends."""
    served = StandIn(Path(os.environ["XDG_RUNTIME_DIR"]) / "daemons" / "quests.sock", searches=True)
    served.told([StandIn.row(0, "Kestrel 0")], [])
    yield served
    served.close()


def battery(pct: int) -> list[Json]:
    """Give Power's headline and Charge row at `pct`, as the machine daemon serves them.

    The shapes are power_rows.rs's, in the crate's wrap, as modules/quickshell/tests/power.qml
    serves them too.
    """
    graph = {"shape": "bar", "values": [pct / 100], "warn": False}
    head = {"said": f"{pct}% \u00b7 on battery", "graph": graph}
    row = {
        "key": "row:power:charge",
        "thing": "power:charge",
        "place": "power",
        "kind": "figure",
        "order": 2,
        "depth": 0,
        "name": "Charge",
        "side": [{"said": f"{pct}%"}],
        "graph": graph,
        "acts": [{"key": "verb:copy", "copy": f"{pct}%"}],
    }
    return [
        {"key": "head:power", "kind": "head", "head": head},
        {"key": "row:power:charge", "kind": "row", "row": row},
    ]


@pytest.fixture
def machine(shell: Shell) -> Iterator[StandIn]:
    """Give the machine daemon, stood in for, holding the battery, for one case alone.

    The check's own machine daemon reads the builder, which has no battery, so it's stopped for
    the case and started again after. The shell's connections hear the socket made and come to
    the stand-in at once (Daemon.qml's FileView), as they come back to the daemon after.
    """
    shell.daemon_stopped("machine")
    served = StandIn(Path(os.environ["XDG_RUNTIME_DIR"]) / "daemons" / "machine.sock")
    served.told(battery(80), [])
    try:
        yield served
    finally:
        served.close()
        shell.machine()


def updates_presented(shell: Shell, after: int, deadline: float) -> list[Json]:
    """Give Quests' changes said presented past the first `after`, once there are any."""
    while time.monotonic() < deadline:
        presented = json.loads(shell.as_account(shell.ipc("updatesPresented")))
        if len(presented) > after:
            return [one for one in presented[after:] if one["place"] == "quests"]
        time.sleep(launcher_on_niri.POLL)
    return []


def quests_shown(shell: Shell) -> int:
    """Open the launcher in Quests, and give how many changes it had said presented.

    Stepped into, since a search at the top folds a place's other finds into its first row's
    side, which changes too little of the screen to see.
    """
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    shell.type_in("quests", selecting="Quests")
    shell.step_in()
    shell.launcher_until(
        lambda shown: shown["path"][-1:] == ["Quests"] and shown["rows"] == ["Kestrel 0"]
    )
    return len(json.loads(shell.as_account(shell.ipc("updatesPresented"))))


def added() -> list[Json]:
    """Give the rows a change adds, after the one held."""
    return [StandIn.row(order, f"Kestrel {order}") for order in range(1, ADDED + 1)]


def test_a_daemons_change_is_presented_as_niri_first_shows_it(
    shell: Shell, quests: StandIn
) -> None:
    """A daemon's change is timed from its line to the frame niri showed with it.

    Joel's 50 ms bar holds for updates as for keys (2026-11-10), so the launcher times each change
    a daemon tells from the line it came in to niri's presentation feedback for the frame drawing
    it (Y2YBTS, 2026-11-11): never from before the line was written, and niri's screen first shows
    what it left no sooner than the frame before the time said (COPIED_BEFORE_PRESENTED), and no
    picture after it lacks it. Judged by the screen it left, as a key's is (settled_shown).
    """
    after = quests_shown(shell)
    before = still_screen()
    written = quests.told(added(), [])
    deadline = time.monotonic() + launcher_on_niri.WAITING.total_seconds()
    seen, unchanged, presented = settled_shown(
        before, lambda: updates_presented(shell, after, deadline), deadline
    )
    assert seen >= 0, "niri's screen never showed the change"
    shell.launcher_until(lambda shown: f"Kestrel {ADDED}" in shown["rows"])
    shell.close_launcher()
    quests.told([], [row["key"] for row in added()])
    assert presented, "the change was never said presented"
    change = presented[0]
    assert change["from"] >= written - CLOCKS_APART_MS, (
        f"said {change}, timed from before its line was written at {written:.0f}"
    )
    lead = COPIED_BEFORE_PRESENTED.total_seconds() * 1000
    assert change["from"] <= change["at"] <= seen + lead, (
        f"said {change}; niri's screen showed it at {seen:.0f}"
    )
    # Where its time went is said as a key's is, so each stamp is there and in order: the
    # launcher took it after its line, and its frame began after that and before niri showed it.
    assert change["from"] <= change.get("heard", -1) <= change.get("begun", -1) <= change["at"], (
        f"said {change}, without when the launcher took it and its frame began, in order"
    )
    presented_after_unchanged(change["at"], unchanged, "the change")


def ended_quest(name: str) -> Json:
    """Give an ended quest's row as the quests daemon sends it with a search, never served."""
    row = StandIn.row(900, name)["row"]
    held = {**row["held"], "status": "concluded", "priority": "", "open": False}
    return {
        **row,
        "glyph": "done",
        "reads": f"What {name} is about.",
        "unfinished": False,
        "held": held,
    }


def test_an_ended_quest_is_found_read_and_followed(shell: Shell, quests: StandIn) -> None:
    """An ended quest the daemon doesn't serve is found by its name, read and followed.

    Quests' daemon serves only the open quests, since the ended ones held whole were most of
    what the shell's collector walked (BXNMUC, 2026-11-13), so typing asks it for the ended
    ones, whose rows come with its answer (Y2YBTS, 2026-11-13). Found from home, one reads its
    summary in the drawer as an open one does, and Ctrl+] follows it into Quests, where it lives,
    and on into its acts; found inside Quests, it's listed there too.
    """
    ended = ended_quest("Osprey ended")
    key = f"row:quests:{ended['held']['id']}"
    quests.unserved[key] = ended
    try:
        shell.as_account(shell.ipc("toggle"))
        shell.launcher_ready()
        shell.type_in("osprey")
        shell.launcher_until(
            lambda shown: (
                (shown["selected"] or {}).get("name") == "Osprey ended"
                and shown["drawer"]["body"] == "What Osprey ended is about."
            )
        )
        shell.step_in("ctrl-bracketright")
        shell.launcher_until(
            lambda shown: (
                shown["held"] == ["quests", "Osprey ended"]
                and shown["rows"][:1] == ["Start Here"]
                and "ID" in shown["rows"]
            )
        )
        shell.close_launcher()
        quests_shown(shell)
        shell.type_in("osprey")
        shell.launcher_until(lambda shown: shown["rows"] == ["Osprey ended"])
        shell.close_launcher()
    finally:
        del quests.unserved[key]


def test_the_niri_held_is_the_one_its_socket_names() -> None:
    """The niri held is read from its socket's name, the pid that niri itself writes there.

    Looking for it among /proc's processes raced the shell's commands, one of which ended as it
    was read and failed the case holding niri; the socket's name races nothing.
    """
    pid = 1923
    socket = f"/run/user/1000/niri.wayland-2.{pid}.sock"
    assert launcher_on_niri.niri_pid(socket) == pid, f"{socket} named another niri"
    held = launcher_on_niri.niri_pid(os.environ["NIRI_SOCKET"])
    named = Path(f"/proc/{held}/comm").read_text().strip()
    assert named == "niri", f"NIRI_SOCKET named {held}, which is {named}"


def test_a_daemons_change_niri_holds_back_is_presented_late(shell: Shell, quests: StandIn) -> None:
    """A change drawn while niri is held is said presented only once niri shows it, too late.

    The shell hears the line and draws the change at once, but niri, held, presents nothing, so
    a measure taken at the shell's own frame would pass what Joel sees late.
    """
    after = quests_shown(shell)
    with launcher_on_niri.niri_held():
        written = quests.told(added(), [])
        # How long niri is held, as a compositor stalled behind a frame.
        time.sleep(NIRI_HELD.total_seconds())
    deadline = time.monotonic() + launcher_on_niri.WAITING.total_seconds()
    presented = updates_presented(shell, after, deadline)
    shell.launcher_until(lambda shown: f"Kestrel {ADDED}" in shown["rows"])
    shell.close_launcher()
    quests.told([], [row["key"] for row in added()])
    assert presented, "the change was never said presented"
    change = presented[0]
    held = NIRI_HELD.total_seconds() * 1000
    assert change["at"] >= written + held, (
        f"said {change}, presented before niri, held {held:.0f} ms from {written:.0f}, could"
    )
    assert change["at"] - change["from"] > launcher_on_niri.KEY_PACE_MS, (
        f"a change niri held {held:.0f} ms was said presented within 50 ms: {change}"
    )


# How soon after :q's Return a second one is sent, before niri hears the launcher let the keys
# go: 50 and 150 ms both reached the shut launcher on a builder, Joel's 200 ms on his laptop.
LATE_RETURN_S = 0.05

# How long the late Return is given to land, which it does within a frame of being sent.
LATE_LANDS_S = 0.5


def test_a_return_sent_as_it_shuts_takes_no_step(shell: Shell) -> None:
    """A Return niri sends just after :q's, as the launcher shuts, takes no step, unseen.

    Shut from Network, a second Return stepped into home's first row, Now, while nothing showed
    it, and was timed to whatever frame came next: "\u2192 Now" took 643 ms to its frame
    (Joel's journal, 2026-11-11 09:38:51).
    """
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    shell.type_in("network", selecting="network")
    shell.step_in()
    shell.normal()
    shell.send_chars(":q")
    since = shell.mark()
    shell.as_account(f"wtype -k Return; sleep {LATE_RETURN_S}; wtype -k Return")
    shell.launcher_closed()
    # Nothing says a key was dropped, so the late one is given its time to land.
    time.sleep(LATE_LANDS_S)
    said = shell.log.read_bytes()[since:].decode(errors="replace")
    stepped = [line for line in said.splitlines() if '"\u2192 ' in line]
    assert stepped == [], f"the shut launcher took a step: {stepped}"


def test_a_step_out_is_timed_to_its_frame_as_a_step_in_is(shell: Shell) -> None:
    """- out of a place is timed to its frame, named by the arrow and the place it left.

    Joel's - out of Agents and For You took 61 to 67 ms to their frames, 40 to 48 of it the key's
    own work, and only a source's change beside it said so: no line timed the step itself, so
    none said where its time went (his journal, 2026-11-16 18:11:04 and 19:40:19).
    """
    shell.take_notices()
    shell.launcher_ready()
    shell.launcher_until(lambda shown: bool(shown["held"]))
    # Gone into with nothing typed, so - puts no words back.
    depth = len(shell.showing()["held"])
    presented = minus_presented(shell, lambda shown: len(shown["held"]) == depth - 1)
    shell.close_launcher()
    steps = [one.get("step") for one in presented]
    # One press, timed once: a second key beside it would count the step twice.
    assert len(steps) == 1, f"- out of the notices wasn't timed once: {presented}"
    assert str(steps[0]).startswith("\u2190 "), f"- wasn't timed as a step out: {presented}"


def test_a_step_out_fits_the_foot_once(shell: Shell) -> None:
    """- out of a place a chord opened fits the foot's keys and facts once, where it lands.

    Fitted again as held changed, as the mode came back and as the rows were found, Joel's - out
    of Windows spent 20 and 19 ms of its 58 and 53 ms holding, before it searched anything (his
    journal, 2026-11-17 23:05:23 and 2026-11-18 10:43:47). Fitted for each, they were fitted three
    times. Held, the keys must still read as the row they land on fits them, moved off and back.
    The facts read nothing of the row: fitted as the hold ends, from what they read then, and
    fitted again live by any later change to it, they can't stay stale. The count says only how
    often the step itself fitted them, not what came after it.
    """
    shell.launcher_until(lambda shown: not shown["open"])
    shell.as_account(shell.ipc("collection", "windows"))
    shell.launcher_until(lambda shown: shown["open"] and len(shown["held"]) > 0)
    shell.launcher_ready()
    shell.normal()
    shell.send_chars("-")
    shell.launcher_until(lambda shown: not shown["held"])
    laid = laid_out(shell)
    # Its identity, as a row's count or drawer may change under it while windows come and go.
    landed_on = (shell.showing()["selected"] or {}).get("is")
    shell.send_chars("j")
    shell.launcher_until(lambda shown: (shown["selected"] or {}).get("is") != landed_on)
    shell.send_chars("k")
    shell.launcher_until(lambda shown: (shown["selected"] or {}).get("is") == landed_on)
    refitted = laid_out(shell)["foot"]
    shell.close_launcher()
    fits = laid["stepFits"]
    assert fits == {"keys": 1, "facts": 1}, f"- out of Windows fitted the foot {fits}"
    # The clock may turn between the two.
    held = [word["text"] for word in laid["foot"] if not re.search("[0-9]", word["text"])]
    fresh = [word["text"] for word in refitted if not re.search("[0-9]", word["text"])]
    assert held == fresh, f"the foot held {held} where its row fits {fresh}"


def test_a_step_out_counts_the_rows_made_while_it_held(shell: Shell) -> None:
    """- out of a place a chord opened counts the rows home's sources made as it was held.

    Joel's - out of Now held 14, 22 and 38 ms in one evening, where every other step out held 2
    to 4, and nothing said where (his journal, 2026-11-18). Setting held wakes home's sources,
    which a chord's place left unwanted, and each makes its rows within it. Counted on either side
    of held alone, those rows are what the step's line names; counted after it, they'd be none.
    It leans on that waking remaking rows: once it no longer does, the case needs a source that
    still makes rows as held is set.
    """
    shell.launcher_until(lambda shown: not shown["open"])
    shell.as_account(shell.ipc("collection", "windows"))
    shell.launcher_until(lambda shown: shown["open"] and len(shown["held"]) > 0)
    shell.launcher_ready()
    shell.normal()
    shell.send_chars("-")
    shell.launcher_until(lambda shown: not shown["held"])
    holding = laid_out(shell)["stepHolding"]
    shell.close_launcher()
    assert holding is not None, "- out of Windows counted nothing as it held"
    made = {name: rows for name, rows in holding["rows"].items() if rows > 0}
    assert made, f"- out of Windows counted no rows made as it held: {holding}"
    until = timedelta(milliseconds=holding["untilHeld"])
    assert timedelta(0) <= until < HELD_WITHIN, f"- out of Windows was held {until} from its key"


def test_a_step_out_putting_words_back_is_timed_as_the_step(shell: Shell) -> None:
    """- back onto what was typed is timed once, as the step out and the place it left.

    The words it puts back search as a key's do, and were timed as those words alone, from the
    field's change, naming no place and leaving out the step's own work (a council, 2026-11-17).
    """
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    shell.type_in("programs", selecting="Programs")
    shell.step_in()
    presented = minus_presented(shell, lambda shown: shown["held"] == [] and shown["typed"] != "")
    shell.close_launcher()
    steps = [one.get("step") for one in presented]
    assert steps == ["\u2190 Programs"], f"- out of Programs wasn't timed once as it: {presented}"


def test_a_step_in_from_typed_words_is_timed_once_as_the_step(shell: Shell) -> None:
    """Return into a place found by what's typed is timed once, as the step and the place.

    The step empties the field, and the field timed that as a key of its own beside the step's,
    so one Return counted twice in health and trials (a council, 2026-11-17).
    """
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    shell.type_in("programs", selecting="Programs")
    before = len(launcher_on_niri.keys_presented_settled(shell))
    shell.step_in()
    presented = launcher_on_niri.keys_presented_settled(shell)[before:]
    # A letter typed after the step is timed as a key again.
    shell.type_in("f")
    after = launcher_on_niri.keys_presented_settled(shell)[before + len(presented) :]
    shell.close_launcher()
    steps = [one.get("step") for one in presented]
    assert steps == ["\u2192 Programs"], f"Return into Programs wasn't timed once: {presented}"
    assert [one.get("step") for one in after] == [""], f"a letter after it went untimed: {after}"


def test_a_step_in_through_the_place_holding_it_is_timed_once_as_the_step(shell: Shell) -> None:
    """Return into Power, which sits in Machine, is timed once, as the step into Power.

    A place found from the top that sits in another is stepped into through it, and that step
    was timed only as the field's emptying, naming no place (a council, 2026-11-17).
    """
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    shell.type_in("power")
    shell.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Power")
    before = len(launcher_on_niri.keys_presented_settled(shell))
    shell.step_in()
    shell.launcher_until(lambda shown: shown["path"][-2:] == ["Machine", "Power"])
    presented = launcher_on_niri.keys_presented_settled(shell)[before:]
    shell.close_launcher()
    steps = [one.get("step") for one in presented]
    assert steps == ["\u2192 Power"], f"Return into Power wasn't timed once: {presented}"


def test_a_step_in_by_a_word_and_a_space_is_timed_once_as_the_step(shell: Shell) -> None:
    """Typing network and a space steps into Network, the space timed once as the step.

    The space's key was timed as the words it typed, and the field the step emptied as a key of
    its own, so one step counted twice and named no place (a council, 2026-11-17).
    """
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    shell.type_in("network")
    before = len(launcher_on_niri.keys_presented_settled(shell))
    shell.send_chars(" ")
    shell.launcher_until(lambda shown: shown["path"][-1:] == ["Network"])
    presented = launcher_on_niri.keys_presented_settled(shell)[before:]
    shell.close_launcher()
    steps = [one.get("step") for one in presented]
    assert steps == ["\u2192 Network"], f"the space into Network wasn't timed once: {presented}"


def test_a_word_and_a_space_typed_at_once_time_only_the_last_key_as_the_step(shell: Shell) -> None:
    """Network and a space typed in one go time only the space as the step into Network.

    The keys of a burst are timed together by the search that took the step, so naming each of
    them the step would count one step as many.
    """
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    before = len(launcher_on_niri.keys_presented_settled(shell))
    typed_at_once(shell, "'network '")
    shell.launcher_until(lambda shown: shown["path"][-1:] == ["Network"])
    presented = launcher_on_niri.keys_presented_settled(shell)[before:]
    shell.close_launcher()
    steps = [one.get("step") for one in presented]
    assert steps.count("\u2192 Network") == 1, f"the step was timed other than once: {presented}"
    assert steps[-1] == "\u2192 Network", f"a key after the step was timed: {presented}"


def test_a_walk_through_a_place_is_timed_once_as_where_it_ends(shell: Shell) -> None:
    """A walk to a place inside another is timed once, as the place it ends in.

    A spot's Return and a terminal's go to walk the places on the way, as Machine on the way
    to Power, and each step was timed as its own, so one walk said two steps and the first
    left out what the last took (a council, 2026-11-17). What the places on the way took is
    said as passing, which modules/quickshell/tests/trials.qml holds.
    """
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    before = len(launcher_on_niri.keys_presented_settled(shell))
    asked = shlex.quote(json.dumps({"steps": ["machine", "power"], "words": "", "sure": False}))
    shell.as_account(f"qs -p {shell.folder} ipc call -- launcher doing {asked}")
    shell.launcher_until(lambda shown: shown["path"] == ["Machine", "Power"])
    presented = launcher_on_niri.keys_presented_settled(shell)[before:]
    shell.close_launcher()
    steps = [one.get("step") for one in presented]
    assert steps == ["\u2192 Power"], f"the walk to Power wasn't timed once as it: {presented}"


def minus_presented(shell: Shell, out: Callable[[Json], object]) -> list[Json]:
    """Press - from normal, wait until the launcher is `out`, and give every key presented since.

    It counts from once the keys before it are all presented, a step in's a frame behind its
    rows, and waits on past the first since, so a second timing a frame later is seen.
    """
    shell.normal()
    before = len(launcher_on_niri.keys_presented_settled(shell))
    shell.send_chars("-")
    shell.launcher_until(out)
    return launcher_on_niri.keys_presented_settled(shell)[before:]


def moved_presented(shell: Shell, written: float, deadline: float) -> list[Json]:
    """Give Power's changes said presented from no sooner than `written`, once there are any."""
    while time.monotonic() < deadline:
        presented = json.loads(shell.as_account(shell.ipc("updatesPresented")))
        moved = [
            one
            for one in presented
            if one["place"] == "power" and one["from"] >= written - CLOCKS_APART_MS
        ]
        if moved:
            return moved
        time.sleep(launcher_on_niri.POLL)
    return []


def test_an_answer_its_daemon_moves_is_presented_as_a_change_is(
    shell: Shell, machine: StandIn
) -> None:
    """An answer its daemon's line moves is timed from the line to the frame niri showed.

    Joel's 50 ms bar holds for every update (2026-11-10), and Power's, Heat's and every answer's
    moves are said again by asking it, which timed none of them (RCTRF8, 2026-11-11), so no
    figure could say how soon the battery's charge reached his screen.
    """
    shell.answer("battery")
    shell.launcher_until(lambda shown: shown["kind"] == "answer")
    deadline = time.monotonic() + launcher_on_niri.WAITING.total_seconds()
    assert machine.watching("place:power", deadline), "the shell never watched Power's rows"
    written = machine.told(battery(79), [])
    presented = moved_presented(shell, written, deadline)
    shell.close_launcher()
    assert presented, "the battery's move was never said presented"
    change = presented[0]
    assert change["at"] >= change["from"], f"said {change}, presented before its line"


def test_the_battery_answers_headline_opens_its_drawer(shell: Shell, machine: StandIn) -> None:
    """The battery answer's headline opens the drawer the daemon serves, its empty time worded.

    "80% · on battery" opened no drawer, so the headline was the one row saying nothing more
    (Y2YBTS, 2026-11-12 [791a3de5]). The daemon serves the time as a moment, and the shell says
    it, as it does every time a drawer holds.
    """
    records = battery(80)
    empty = int(time.time() * 1000) + 3 * 3600 * 1000
    records[0]["head"]["drawer"] = {
        "eyebrow": "Read From the Machine",
        "eyebrowNames": True,
        "title": "Battery",
        "facts": [
            {"name": "Status", "said": "on battery"},
            {"name": "Empty", "at": empty, "as": "moment", "said": "{time}, in 3 h"},
            {"name": "Read From", "said": "/sys/class/power_supply/BAT0", "identifies": True},
        ],
    }
    machine.told(records, [])
    shell.answer("battery")
    shell.launcher_until(
        lambda shown: (
            bool(shown["drawer"]["shown"])
            and shown["drawer"]["title"] == "Battery"
            and shown["drawer"]["facts"] == ["Status", "Empty", "Read From"]
        )
    )
    said = shell.showing()["drawer"]["values"]
    assert said[0] == "on battery", f"the drawer said its status as {said[0]!r}"
    assert "{time}" not in said[1], f"the empty time was left unworded: {said[1]!r}"
    assert said[1].endswith(", in 3 h"), f"the empty time lost its span: {said[1]!r}"
    shell.close_launcher()


def test_a_charge_told_in_batterys_step_is_said_there(shell: Shell, machine: StandIn) -> None:
    """A charge the daemon tells while Battery's step is shown under Machine is said there.

    Power watched the machine daemon only while its own answer or place could be seen, and a
    step into Battery under Machine was neither, so Charge kept a figure held from before as
    current (Y2YBTS, 2026-11-12 [495f1a1b]).
    """
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    shell.type_in("machine", selecting="Machine")
    shell.step_in()
    shell.launcher_until(lambda shown: "Power" in shown["rows"])
    shell.down_to("Power")
    shell.step_in()
    shell.launcher_until(lambda shown: "Battery" in shown["rows"])
    shell.down_to("Battery")
    shell.step_in()
    shell.launcher_until(lambda shown: "Charge" in shown["rows"])
    deadline = time.monotonic() + launcher_on_niri.WAITING.total_seconds()
    watched = machine.watching("place:power", deadline)
    machine.told(battery(77), [])
    shell.down_to("Charge")
    if watched:
        shell.launcher_until(lambda shown: (shown["selected"] or {}).get("side") == "77%")
    selected = shell.showing()["selected"]
    shell.close_launcher()
    assert watched, "Power's rows were never watched in Battery's step"
    assert selected["side"] == "77%", selected


def heat(degrees: int) -> list[Json]:
    """Give Heat's headline and Processor row at `degrees`, as heat_rows.rs serves them."""
    said = f"{degrees}.0 \u00b0C"
    graph = {"shape": "bar", "values": [degrees / 100], "warn": False}
    row = {
        "key": "row:heat:processor",
        "thing": "heat:processor",
        "place": "heat",
        "kind": "figure",
        "order": 1,
        "depth": 0,
        "name": "Processor",
        "side": [{"said": said, "keep": 3}],
        "graph": graph,
        "acts": [{"key": "verb:copy", "copy": said}],
    }
    head = {"said": f"{said} at its hottest, in the processor", "graph": graph}
    return [
        {"key": "head:heat", "kind": "head", "head": head},
        {"key": "row:heat:processor", "kind": "row", "row": row},
    ]


def test_heat_told_while_machine_is_shown_is_said_in_its_row(
    shell: Shell, machine: StandIn
) -> None:
    """Machine's Heat row says the heat the daemon told last, not the heat it told first.

    Heat said a change again only while its own answer was shown, or as its rows came or went
    while stepped into, and Machine's Heat row, listed in Machine's step, is neither: so the
    row kept the heat it first heard for as long as Machine was open, while the daemon read the
    sensors each second for it.
    """

    def says(degrees: int) -> Callable[[Json], bool]:
        # Heat's row is named Heat, its headline at its side, as each listed thing reads.
        heard = f"Heat \u00b7 {degrees}.0 \u00b0C"
        return lambda shown: any(thing["reads"].startswith(heard) for thing in shown["things"])

    machine.told(heat(52), [])
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    shell.type_in("machine", selecting="Machine")
    shell.step_in()
    shell.launcher_until(says(52))
    time.sleep(SETTLED.total_seconds())
    machine.told(heat(61), [])
    try:
        shell.launcher_until(says(61))
        said = True
    except AssertionError:
        said = False
    shown = [thing["reads"] for thing in shell.showing()["things"]]
    shell.close_launcher()
    assert said, f"Machine's Heat row kept the heat it first heard: {shown}"


def holds(name: str) -> Callable[[Json], bool]:
    """Give a test of what the launcher shows: whether a row named `name` is listed."""
    return lambda shown: name in shown["rows"]


def test_a_drawer_served_apart_is_watched_only_while_its_row_is_drawn(
    shell: Shell, machine: StandIn
) -> None:
    """A drawer the daemon serves apart from its row is asked for as it's drawn, and let go.

    A drawer's kernel counts move on every read, so the machine daemon tells a row's drawer only
    to a watch naming its key (VKCF3Z, 2026-11-12, d136b0dd): the launcher watches it ahead once
    its row is drawn, draws what it's told, and lets it go once the launcher closes, so nothing
    keeps the daemon reading for a drawer no one can see.
    """
    key = "drawer:power:charge"
    drawer = {
        "title": "Charge",
        "body": "The battery's own files, as Linux keeps them.",
        "facts": [{"name": "energy_now", "said": "40,000,000"}],
    }
    charge = battery(80)[1]
    machine.told(
        [
            {**charge, "row": {**charge["row"], "drawer": key}},
            {"key": key, "kind": "drawer", "drawer": drawer},
        ],
        [],
    )
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    shell.type_in("machine", selecting="Machine")
    for step, holding in (("Machine", "Power"), ("Power", "Battery"), ("Battery", "Charge")):
        shell.down_to(step)
        shell.step_in()
        shell.launcher_until(holds(holding))
    shell.down_to("Charge")
    deadline = time.monotonic() + launcher_on_niri.WAITING.total_seconds()
    asked = machine.ahead(key, deadline)
    if asked:
        shell.launcher_until(lambda shown: "40,000,000" in shown["drawer"]["values"])
        moved = [{"name": "energy_now", "said": "39,000,000"}]
        machine.told([{"key": key, "kind": "drawer", "drawer": {**drawer, "facts": moved}}], [])
        shell.launcher_until(lambda shown: "39,000,000" in shown["drawer"]["values"])
    shell.close_launcher()
    let_go = machine.ahead(key, deadline, watched=False)
    assert asked, "Charge's drawer, served apart, was never watched as it was drawn"
    assert let_go, "Charge's drawer was still watched once the launcher closed"


def test_a_drawer_apart_told_as_its_daemon_comes_back_is_drawn(
    shell: Shell, machine: StandIn
) -> None:
    """Battery, stepped into once Power's connection is back, holds the drawer Charge names.

    Power told the launcher of a change only when its keys moved or while its own answer was
    shown or stepped into, and Machine's Power, held as a place of Machine's, is neither. When
    its connection came back after Machine's, as the machine daemon's socket made again brings
    them back in two waves, Power's answer came with the same keys and was never said again,
    so the step held kept the rows from before: Battery's Charge came without the drawer it
    names, in KP0L6A's, XY9H5C's and H6EDLC's runs (2026-11-13, 1ee68dc5, 34235536,
    e02a43bf), as it would after a switch.
    """
    walk = (("Machine", "Power"), ("Power", "Battery"), ("Battery", "Charge"))
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    shell.type_in("machine", selecting="Machine")
    for step, holding in walk[:1]:
        shell.down_to(step)
        shell.step_in()
        shell.launcher_until(holds(holding))
    shell.close_launcher()
    machine.close()
    key = "drawer:power:charge"
    head, charge = battery(80)
    drawer = {"title": "Charge", "facts": [{"name": "energy_now", "said": "40,000,000"}]}
    back = StandIn(machine.path, later=("place:power",))
    try:
        back.told(
            [
                head,
                {**charge, "row": {**charge["row"], "drawer": key}},
                {"key": key, "kind": "drawer", "drawer": drawer},
            ],
            [],
        )
        # The first wave, every other connection, comes back at once on the socket made, and
        # what it remakes it remakes within a moment; Power's comes in the second.
        time.sleep(1)
        shell.as_account(shell.ipc("toggle"))
        shell.launcher_ready()
        shell.type_in("machine", selecting="Machine")
        for step, holding in walk[:2]:
            shell.down_to(step)
            shell.step_in()
            shell.launcher_until(holds(holding))
        back.let("place:power")
        deadline = time.monotonic() + launcher_on_niri.WAITING.total_seconds()
        watched = back.watching("place:power", deadline)
        # Its answer is written as its watch is heard, and read within a moment; only then
        # is Battery stepped into, as it was in the runs that failed.
        time.sleep(1)
        for step, holding in walk[2:]:
            shell.down_to(step)
            shell.step_in()
            shell.launcher_until(holds(holding))
        shell.down_to("Charge")
        try:
            shell.launcher_until(lambda shown: "40,000,000" in shown["drawer"]["values"])
            drawn = True
        except AssertionError:
            drawn = False
        shown = shell.showing()["drawer"]
        shell.close_launcher()
    finally:
        back.close()
    assert watched, "Power's rows were never watched once its daemon came back"
    assert drawn, f"Charge never drew the drawer apart it came to name: {shown}"


def test_a_drawer_served_apart_beside_the_one_drawn_comes_with_its_key(
    shell: Shell, machine: StandIn
) -> None:
    """A drawer served apart for the row beside the one drawn is held before a key reaches it.

    Watched only for the row drawn, a key to the next row left the row before's drawer out
    beside it a round trip, and its coming was never timed from the key (VKCF3Z, 2026-11-12,
    bcbe9a24). So the drawers of the rows beside it are watched too, and a key to one brings
    its own drawer, said presented from that key.
    """
    on_charge(shell, machine)
    deadline = time.monotonic() + launcher_on_niri.WAITING.total_seconds()
    beside = machine.ahead("drawer:power:rate", deadline)
    shell.normal()
    said = said_once(shell, walked_to_rate(shell), DRAWER_SAID)
    lines = [line for line in said.splitlines() if "drawer for" in line]
    shown = shell.showing()["drawer"]
    shell.close_launcher()
    assert beside, "Rate's drawer, beside the one drawn, wasn't watched before a key reached it"
    assert any('drawer for "power:rate" presented' in line for line in lines), (
        f"Rate's drawer was never said presented from the key that reached it: {lines}"
    )
    assert shown["title"] == "Rate", f"the drawer beside Rate's row was {shown['title']!r}"


def test_a_drawer_apart_is_held_again_as_the_launcher_opens_again(
    shell: Shell, machine: StandIn
) -> None:
    """Opened again on Charge, the launcher shows its drawer, still watched ahead from before.

    Each opening watches the place and its drawers ahead again, and a whole answer to the place
    that came after the drawers' let them go, so Charge's drawer never showed again until it
    changed (2026-11-12).
    """
    on_charge(shell, machine)
    shell.close_launcher()
    on_charge(shell, machine)
    shell.close_launcher()


def walked_to_rate(shell: Shell) -> int:
    """Walk down from Charge to Rate by letter, and give the log's mark at the key reaching it.

    By a letter, as Joel walks rows, since an arrow sent here comes with no press time to say
    its drawer from. Battery's own Copy row lands before or after Rate as Rate is placed.
    """

    def named(shown: Json) -> object:
        return (shown["selected"] or {}).get("name")

    def left(was: object, shown: Json) -> bool:
        return named(shown) != was

    for _ in range(3):
        since = shell.mark()
        was = named(shell.showing())
        shell.send_chars("j")
        shell.launcher_until(functools.partial(left, was))
        if named(shell.showing()) == "Rate":
            return since
    unreached = f"three keys down from Charge never reached Rate: {shell.showing()}"
    raise AssertionError(unreached)


def on_charge(shell: Shell, machine: StandIn) -> None:
    """Serve Charge and Rate with drawers apart, and step through Machine to Charge.

    It leaves the launcher open on Charge with its drawer out.
    """
    charge = battery(80)[1]
    drawn = {
        "charge": {"title": "Charge", "facts": [{"name": "energy_now", "said": "40,000,000"}]},
        "rate": {"title": "Rate", "facts": [{"name": "power_now", "said": "9,000,000"}]},
    }
    rows = [
        {
            **charge,
            "key": f"row:power:{part}",
            "row": {
                **charge["row"],
                "thing": f"power:{part}",
                "name": drawer["title"],
                "order": 2 + nth,
                "drawer": f"drawer:power:{part}",
            },
        }
        for nth, (part, drawer) in enumerate(drawn.items())
    ]
    apart = [
        {"key": f"drawer:power:{part}", "kind": "drawer", "drawer": drawer}
        for part, drawer in drawn.items()
    ]
    machine.told(rows + apart, [])
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    shell.type_in("machine", selecting="Machine")
    for step, holding in (
        ("Machine", "Power"),
        ("Power", "Battery"),
        ("Battery", "Charge"),
    ):
        shell.down_to(step)
        shell.step_in()
        shell.launcher_until(holds(holding))
    shell.down_to("Charge")
    try:
        shell.launcher_until(lambda shown: "40,000,000" in shown["drawer"]["values"])
    except AssertionError as failed:
        with machine.lock:
            watches = [sorted(watches.values()) for watches in machine.clients.values()]
        said, _ = shell.said_since(0)
        drawers = [report for report in said if "drawer" in report][-12:]
        message = f"{failed}\nwatched of the stand-in: {watches}\nsaid of drawers: {drawers}"
        raise AssertionError(message) from failed


def agent_window(address: str, window: Json, title: str) -> Json:
    """Give a session's window as the agents daemon serves it, paired with niri's window."""
    return {
        "key": f"agent-window:agents:{address}",
        "kind": "agent-window",
        "address": address,
        "kitty": {"id": "1", "within": "unix:@kitty-1"},
        "app_id": f"pi-hosted.{address}",
        "window": f"window:{window['id']}",
        "title": title,
        "orchestration": None,
    }


def test_desktops_windows_row_has_a_drawer_holding_none_or_some(shell: Shell) -> None:
    """Desktop's Windows row has a drawer: its own sentence holding none, else the names.

    Every row has a drawer giving a little more than its line (Joel, 2026-11-11 22:00); a
    place holding none had none, as Windows with only the window in front (Y2YBTS, 641b64f8).
    """
    path = shlex.quote("\u203adesktop")
    asked = f"qs -p {shell.folder} ipc call -- launcher drawers {path}"

    def drawer() -> str:
        rows = cast("list[Json]", json.loads(shell.as_account(asked)).get("rows", []))
        row = next((row for row in rows if row.get("name") == "Windows"), {})
        return str(cast("Json", row.get("drawer", {})).get("body", ""))

    def said(body: str) -> None:
        deadline = time.monotonic() + launcher_on_niri.WAITING.total_seconds()
        while drawer() != body:
            assert time.monotonic() < deadline, f"Windows' drawer said {drawer()!r}, not {body!r}"
            time.sleep(launcher_on_niri.POLL)

    said("No other windows.")
    kite = shell.opened("Kite Notes")
    lark = shell.opened("Lark Notes")
    try:
        said("Kite Notes")
    finally:
        shell.close(kite)
        shell.close(lark)


def test_a_drawer_out_says_a_window_opened_while_it_rests(shell: Shell) -> None:
    """Windows' drawer, out under Desktop, names a window opened while Joel rests on it.

    A drawer's content is kept live (Joel, 2026-11-11 20:28), the shell's own as a daemon's,
    so a window opening moves the drawer shown, not only the next one asked for.
    """
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    shell.type_in("desktop", selecting="Desktop")
    shell.step_in()
    shell.launcher_until(lambda shown: "Windows" in shown["rows"])
    shell.down_to("Windows")
    shell.launcher_until(
        lambda shown: shown["drawer"]["shown"] and shown["drawer"]["body"] == "No other windows."
    )
    # Two, since the window in front is no other window: the first is named once the second is.
    kite = shell.opened("Kite Notes")
    lark = shell.opened("Lark Notes")
    try:
        shell.launcher_until(
            lambda shown: shown["drawer"]["shown"] and "Kite Notes" in shown["drawer"]["body"]
        )
    finally:
        shell.close_launcher()
        shell.close(kite)
        shell.close(lark)


def test_captures_acts_each_say_what_they_do_in_a_drawer(shell: Shell) -> None:
    """Each of Capture's acts has a drawer saying what it does and where that lands.

    Every row has a drawer giving a little more than its line (Joel, 2026-11-11 22:00), an act's
    saying what it does, its key, where it lands and the last one there (VKCF3Z, 37e49991), then
    the act and what follows from it, Open the Folder, and folded, its command, path and format.
    """
    path = shlex.quote("\u203adesktop\u203acapture")
    asked = f"qs -p {shell.folder} ipc call -- launcher drawers {path}"
    rows = cast("list[Json]", json.loads(shell.as_account(asked)).get("rows", []))
    drawers = {str(row.get("name")): cast("Json", row.get("drawer", {})) for row in rows}
    screen = drawers.get("The Whole Screen", {})
    assert str(screen.get("body", "")).startswith("Takes a picture of the whole screen"), screen
    facts = {
        str(fact.get("label")): str(fact.get("value"))
        for fact in cast("list[Json]", screen.get("facts", []))
    }
    assert facts.get("Lands In") == "Screenshots, and on the clipboard", facts
    region = drawers.get("Record a Region", {})
    assert str(region.get("body", "")).startswith("Records the part of the screen"), region
    # A terminal's show says no acts or nerdy bits, so they're read as drawn.
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    shell.type_in("capture", selecting="Capture")
    shell.step_in()
    shell.launcher_until(lambda shown: "The Whole Screen" in shown["rows"])
    shell.down_to("The Whole Screen")
    shell.launcher_until(
        lambda shown: (
            shown["drawer"]["acts"] == ["\u21b5 Take", "Open the Folder"]
            and shown["drawer"]["nerd"] == ["Command", "Path", "Format"]
        )
    )
    shell.close_launcher()


def test_agents_windows_are_one_row_in_windows(shell: Shell) -> None:
    """Desktop's Windows lists agents' windows as one Agents row, where the one used last was.

    The agents daemon says which windows are agents' (VKCF3Z's ruling, 2026-11-11); the row
    names the session Return goes to and how many more, and the other windows keep their order.
    """

    def windows_listed() -> list[Json]:
        path = shlex.quote("\u203adesktop\u203awindows")
        asked = f"qs -p {shell.folder} ipc call -- launcher show {path}"
        return cast("list[Json]", json.loads(shell.as_account(asked)).get("rows", []))

    def folded() -> bool:
        names = [row["name"] for row in windows_listed()]
        return "Agents" in names and "Heron Notes" in names

    served = StandIn(Path(os.environ["XDG_RUNTIME_DIR"]) / "daemons" / "agents.sock")
    wren = shell.opened("Wren Agent")
    heron = shell.opened("Heron Notes")
    finch = shell.opened("Finch Agent")
    here = shell.opened("Here Notes")
    try:
        agents = [
            agent_window("WREN01", wren, "Wren Work"),
            agent_window("FINCH1", finch, "Finch Work"),
        ]
        served.told(agents, [])
        deadline = time.monotonic() + launcher_on_niri.WAITING.total_seconds()
        while not folded():
            assert time.monotonic() < deadline, f"Windows never folded: {windows_listed()}"
            time.sleep(launcher_on_niri.POLL)
        rows = windows_listed()
        names = [row["name"] for row in rows]
        assert names[:2] == ["Agents", "Heron Notes"], f"not in the order used: {names}"
        assert "Wren Agent" not in names, f"an agents' window was listed loose: {names}"
        assert "Finch Agent" not in names, f"an agents' window was listed loose: {names}"
        assert rows[0].get("side") == "Finch Work \u00b7 1 more", f"said {rows[0]}"
    finally:
        served.close()
        for window in (wren, heron, finch, here):
            shell.close(window)


@pytest.mark.usefixtures("machine")
def test_a_row_holding_only_a_copy_copies_on_return(shell: Shell) -> None:
    """Return on a served row whose one act is a copy copies it, never opening a list of it.

    Power's Charge is served with a copy alone, so stepping in showed one row named by its act's
    key (Joel, 2026-11-11: "if they have nothing under them, we shouldn't open a submenu for
    them"). It copies what the row says, whichever charge that is: inside Machine's Power,
    Power's rows aren't watched, so a figure told then isn't heard (RCTRF8's, 2026-11-12).
    """
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    shell.type_in("power", selecting="Power")
    shell.step_in()
    shell.launcher_until(
        lambda shown: shown["path"][-1:] == ["Power"] and "Battery" in shown["rows"]
    )
    shell.down_to("Battery")
    shell.step_in()
    shell.launcher_until(lambda shown: "Charge" in shown["rows"])
    shell.down_to("Charge")
    charge = shell.showing()["selected"]
    assert charge["verb"] == "copy", charge
    assert re.fullmatch(r"\d+%", charge["side"]), charge
    with shell.told("parted copied"):
        shell.send_key("ret")
    shell.launcher_closed()
    assert shell.copied() == charge["side"], shell.copied()


def test_a_drawer_out_says_what_its_daemon_tells_as_it_comes(
    shell: Shell, machine: StandIn
) -> None:
    """A drawer out says a figure its daemon tells while it's shown, with no key pressed.

    A drawer's content is kept live (Joel, 2026-11-11 20:28: "we also need to make sure that
    its content is live updated"), so Charge's drawer moves from 80% to 77% where it stands.
    """

    def drawered(pct: int) -> list[Json]:
        records = battery(pct)
        row = cast("Json", records[1]["row"])
        row["drawer"] = {"title": "Charge", "facts": [{"name": "Now", "said": f"{pct}%"}]}
        return records

    def drawn(shown: Json) -> str:
        drawer = cast("Json", shown["drawer"])
        words = [drawer["title"], drawer["body"], *cast("list[str]", drawer["values"])]
        return " ".join(str(word) for word in words)

    machine.told(drawered(80), [])
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    shell.type_in("power", selecting="Power")
    shell.step_in()
    shell.launcher_until(lambda shown: "Battery" in shown["rows"])
    shell.down_to("Battery")
    shell.step_in()
    shell.launcher_until(lambda shown: "Charge" in shown["rows"])
    shell.down_to("Charge")
    shell.launcher_until(lambda shown: shown["drawer"]["shown"] and "80%" in drawn(shown))
    deadline = time.monotonic() + launcher_on_niri.WAITING.total_seconds()
    assert machine.watching("place:power", deadline), "the shell never watched Power's rows"
    machine.told(drawered(77), [])
    shell.launcher_until(lambda shown: shown["drawer"]["shown"] and "77%" in drawn(shown))
    shell.close_launcher()


def test_on_the_selection_draws_its_actions_before_a_waking_server_answers(shell: Shell) -> None:
    """On the Selection draws pi-verbs' actions as served, Ask last, before a slow pi-verbs answers.

    pi-verbs rests after five minutes alone and takes 146 to 231 ms to answer cold (harness
    #1436), so the place draws from the list it kept and freshens it when the answer comes
    (WCJQSG, c0f83eb9). It holds pi-verbs only while it's open, and Super+X again puts it away
    (VKCF3Z, e684d7ae). The stand-in takes a second, rather than a cold 250 ms, so asking the
    shell what it shows never takes longer than the wait it's measured against.
    """

    class Waking(StandIn):
        """pi-verbs, answering each watch only after `wait` seconds, as one waking does."""

        def __init__(self, path: Path) -> None:
            """Listen on `path`, answering at once until told to wait."""
            self.wait = 0.0
            self.answered: list[float] = []
            self.open = 0
            super().__init__(path)

        def serve(self, client: socket.socket) -> None:
            """Answer one client, its watch after the wait, counting it open until it goes."""
            with self.lock:
                self.open += 1
            try:
                with contextlib.suppress(OSError), client.makefile("r") as lines:
                    for line in lines:
                        asked = json.loads(line)
                        if asked.get("ask") == "watch" and asked.get("id") == "all":
                            time.sleep(self.wait)
                            self.write(client, {"id": "all"}, watching=self.watched(asked))
                            self.answered.append(time.monotonic())
                        elif asked.get("ask") == "hello":
                            self.write(client, {"id": "hello", "version": 1})
            finally:
                with self.lock:
                    self.open -= 1

    def verb(name: str, says: str) -> Json:
        """Give an action as pi-verbs serves it, its words its name."""
        return {
            "key": f"verb:{name.lower()}",
            "kind": "verb",
            "id": name.lower(),
            "words": name,
            "says": says,
            "result": "say",
            "disruptive": False,
            "args": [],
        }

    served = Waking(Path(os.environ["XDG_RUNTIME_DIR"]) / "daemons" / "pi-verbs.sock")
    served.told([verb("Explain", "What it means"), verb("Shorten", "Fewer words")], [])
    listed = ["Explain", "Shorten", "Ask"]

    def chord() -> None:
        """Press Super+X as niri runs it, through launcher-say's socket, which a place may take.

        qs ipc's collection only ever opens it, so Super+X again putting it away is only seen
        the way the chord says it.
        """
        with socket.socket(socket.AF_UNIX) as said:
            said.connect(str(Path(os.environ["XDG_RUNTIME_DIR"]) / "launcher.sock"))
            said.sendall(b"collection selection\n")

    try:
        chord()
        shell.launcher_ready()
        shell.launcher_until(
            lambda shown: shown["rows"] == listed and shown["hint"] == "nothing selected"
        )
        chord()
        shell.launcher_closed()
        deadline = time.monotonic() + launcher_on_niri.WAITING.total_seconds()
        while served.open > 0:
            assert time.monotonic() < deadline, "the place held pi-verbs after it closed"
            time.sleep(launcher_on_niri.POLL)
        served.wait = 1.0
        chord()
        shell.launcher_until(lambda shown: shown["open"] and shown["rows"] == listed)
        drawn = time.monotonic()
        deadline = time.monotonic() + launcher_on_niri.WAITING.total_seconds()
        while len(served.answered) < 2:  # noqa: PLR2004, the first opening's answer and this one's
            assert time.monotonic() < deadline, "pi-verbs was never asked again"
            time.sleep(launcher_on_niri.POLL)
        assert drawn < served.answered[1], "the place waited for pi-verbs to draw its actions"
        openings = json.loads(shell.as_account(shell.ipc("openings")))
        logging.getLogger(__name__).info("On the Selection drew in %s ms", openings[-1]["took"])
        shell.close_launcher()
    finally:
        served.close()


def test_no_answer_raised_an_error_and_quickshell_warned_of_nothing(shell: Shell) -> None:
    """No answer raised an error, and Quickshell warned of nothing it drew."""
    log = shell.log.read_text(errors="replace").splitlines()
    # The build has no PipeWire, which the shell says once as it starts;
    # that's the sandbox's, never an answer's.
    errors = [
        line for line in log if ("ERROR" in line or "Error:" in line) and "pipewire" not in line
    ]
    assert errors == [], f"an answer raised an error: {errors}"
    # Quickshell colours its levels and categories even in its log, so a
    # colour code may stand between the two words.
    scenes = [line for line in log if re.search("WARN[^ ]* scene", line)]
    assert scenes == [], f"Quickshell warned of a scene: {scenes}"
