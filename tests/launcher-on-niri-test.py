"""The launcher and the held notices answer what's typed, on the account's whole shell on niri.

modules/quickshell/checks/core.nix runs this through pytest inside tests/nested-niri.sh, with the
shell launcher_on_niri.py starts once a run. Each case here needs only the launcher or the
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
import math
import os
import re
import shlex
import shutil
import signal
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
# How long before its line a chord is said to have run, well past any opening's own time here.
CHORD_AGO = timedelta(seconds=2)
# How long the shell is held stopped just after a chord, well past any opening's own time here.
HELD = timedelta(seconds=2)
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
    done = subprocess.run(["date", *form], capture_output=True, text=True, check=True)  # noqa: S603, S607
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
            shown["drawer"]["nerd"] == ["Unix Time", "Julian Day"]
            and shown["drawer"]["folded"] is False
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


def test_return_on_an_answer_is_kept_as_a_pick_of_its_kind(shell: Shell) -> None:
    """An opening ended by Return on a sum keeps it as a pick of a sum, not a miss.

    An answer's key is empty, so every Return on Ask, a sum or Memory was kept as a miss, and
    what the trials said of them was wrong (1JQPPC, 2026-11-11).
    """
    state = Path(os.environ.get("XDG_STATE_HOME") or Path.home() / ".local/state")

    def kept() -> list[Json]:
        lines = [
            line
            for path in state.glob("quickshell/by-shell/*/launcher-trials.jsonl")
            for line in path.read_text().splitlines()
            if line.strip()
        ]
        return [cast("Json", json.loads(line)) for line in lines]

    before = len(kept())
    copied(shell, "6*7", "42")
    deadline = time.monotonic() + launcher_on_niri.WAITING.total_seconds()
    # The shell adds the opening's line through a pipe once it closes, so it's read again until
    # it's there, a poll's length apart.
    while len(kept()) <= before:
        assert time.monotonic() < deadline, "the opening was never kept"
        time.sleep(launcher_on_niri.POLL)
    last = kept()[-1]
    assert last.get("picked") == "answer:sum", last
    assert last.get("missed") is False, last


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


def test_do_not_disturb_says_until_when_in_its_drawer(shell: Shell) -> None:
    """Do Not Disturb, off, has a drawer saying until when Return holds notices.

    Joel wants every row to say a little more beside it (2026-11-11), and Desktop's Do Not
    Disturb had none; what each fact says is tests/piece-drawers.qml's, and this is its one
    path through the launcher.
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


def test_the_shut_shell_sleeps_as_a_window_is_resized(shell: Shell) -> None:
    """With the launcher shut, a window resized over and over is heard of not once.

    The shell's niri heard every event niri sent, each tile resized or scrolled among them, and
    Joel's shut shell woke for each (2026-11-10), though only a notice drawn beside its window
    reads a tile. It now hears the windows daemon, told only the fields it reads, and asks for
    tiles only while a notice is drawn. What the shell reads of niri's windows is counted line
    by line, the window's opening proving the count runs; a wake counted from /proc took in
    whatever else the shell did then, such as the work a closing leaves for a moment after.
    """

    def heard() -> dict[str, int]:
        return cast("dict[str, int]", json.loads(shell.as_account(shell.ipc("linesHeard"))))

    before = heard()
    window = shell.opened("Dunlin Notes")
    try:
        shell.as_account(f"niri msg action focus-window --id {window['id']}")
        # The span measured, for the window's opening to reach the shell, not a wait for it.
        time.sleep(SETTLING.total_seconds())
        opening = heard()
        for one in range(RESIZES):
            shell.as_account(f"niri msg action set-column-width {'+' if one % 2 else '-'}10%")
        # The span measured, for the resizes' events to reach the shell, not a wait for them.
        time.sleep(SETTLING.total_seconds())
        resized = heard()
    finally:
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


def searched(shell: Shell) -> int:
    """Give how many searches of what's typed the launcher has run."""
    return int(shell.as_account(shell.ipc("searched")).strip())


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
    before = searched(shell)
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
    searches = searched(shell) - before
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


def notices_daemon() -> int:
    """Give the notices daemon's process, which tests/notices-beside.sh started beside the shell."""
    for named in Path("/proc").glob("[0-9]*/comm"):
        with contextlib.suppress(OSError):
            if named.read_text().strip() == "notices":
                return int(named.parent.name)
    failed = "no notices daemon runs beside the shell"
    raise AssertionError(failed)


@contextlib.contextmanager
def notices_stopped() -> Iterator[None]:
    """Hold the notices daemon stopped while what's done inside it runs, so it answers late."""
    daemon = notices_daemon()
    os.kill(daemon, signal.SIGSTOP)
    try:
        yield
    finally:
        os.kill(daemon, signal.SIGCONT)


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
    with notices_stopped():
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
    look.write_text(json.dumps(said))
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


def test_a_look_cleared_in_for_you_is_written_cleared(shell: Shell) -> None:
    """Clearing a look in For You's To Look At writes its file again, saying it was cleared.

    The write is made off the shell's thread (2026-11-10), so this is the one path showing it
    still lands, whole.
    """
    state = Path(os.environ.get("XDG_STATE_HOME") or Path.home() / ".local/state")
    looks = state / "pi/agentic-harness.pi/looks"
    looks.mkdir(parents=True, exist_ok=True)
    look = looks / "cleared.json"
    # It came just now, since one three days old goes quiet (launcher/Looks.js).
    said = {"v": 1, "id": "cleared", "title": "Cleared Look", "target": "/etc/hostname",
            "kind": "file", "state": "waiting", "at": now("-u", "+%FT%TZ")}  # fmt: skip
    look.write_text(json.dumps(said))
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
        with shell.told("ask ready"):
            shell.send_chars("dd")
        shell.send_chars("y")
        shell.launcher_until(lambda _shown: json.loads(look.read_text())["state"] == "cleared")
        written = json.loads(look.read_text())
        assert (written["id"], written["title"]) == ("cleared", "Cleared Look"), written
        shell.close_launcher()
    finally:
        look.unlink()


# How many looks wait in To Look At, enough that weighing them all again shows.
LOOKS_WAITING = 12
# How many rows an act may weigh again: the one it changed and the ask it answered.
WEIGHED_BY_AN_ACT = 2


# How long the rows weighed must hold still before a count of them is read.
WEIGHED_STILL = 1.0


def weighed_held_still(shell: Shell, timeout: float = 10.0) -> int:
    """Give how many rows the launcher has weighed, once that holds still for WEIGHED_STILL."""
    deadline = time.monotonic() + timeout
    last: int = shell.showing()["weighed"]
    since = time.monotonic()
    while time.monotonic() - since < WEIGHED_STILL:
        assert time.monotonic() < deadline, f"the rows weighed never held still: {last}"
        time.sleep(0.1)
        now = shell.showing()["weighed"]
        if now != last:
            last, since = now, time.monotonic()
    return last


def test_an_act_on_one_look_weighs_the_keys_of_no_other(shell: Shell) -> None:
    """Clearing one look in To Look At weighs its own row's keys again, and no other's.

    One act in For You's 115 rows weighed every row's keys for the foot again, 7 to 46 ms of
    the 11 to 52 its rows took to be shown, where one row had changed (2026-11-10).
    """
    state = Path(os.environ.get("XDG_STATE_HOME") or Path.home() / ".local/state")
    looks = state / "pi/agentic-harness.pi/looks"
    looks.mkdir(parents=True, exist_ok=True)
    written = [looks / f"weighed{at:02d}.json" for at in range(LOOKS_WAITING)]
    # Each came in the last hour, a minute apart, since one three days old goes quiet
    # (launcher/Looks.js).
    hour = now("-u", "-d", "1 hour ago", "+%FT%H")
    for at, look in enumerate(written):
        said = {"v": 1, "id": look.stem, "title": f"Weighed Look {at}", "kind": "file",
                "target": "/etc/hostname", "state": "waiting",
                "at": f"{hour}:{at:02d}:00Z"}  # fmt: skip
        look.write_text(json.dumps(said))
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
        with shell.told("ask ready"):
            shell.send_chars("dd")
        # Counted only once the weighing holds still, since on a loaded builder what came
        # before the act, as the rows shown again or the keys' face measured, can land
        # after it and count as its (one act weighed 9, 85PKB2, 2026-11-12).
        before = weighed_held_still(shell)
        since = shell.mark()
        shell.send_chars("y")
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

    The held notices' : line took it and the launcher's didn't, so the two lines kept two
    sets of keys; they're one line now (Joel, 2026-11-06: "they need to share more code").
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


# The longest the selection glides after its key, in ms, as Glide.js gives it.
GLIDE = 30


def test_the_selection_glides_to_its_row_and_typing_never_glides(shell: Shell) -> None:
    """An arrow glides the selection to its row within 50 ms; a letter never glides.

    The glide is the launcher's own, about 30 ms, so the selection lands within 50 ms of the
    key, rather than the theme's 150 ms that windows and menus keep (VKCF3Z, 2026-11-10).
    """
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    shell.type_in("o")
    # A row to glide from and one to glide to.
    shell.launcher_until(lambda shown: len(shown["rows"]) > 1)
    since = shell.mark()
    shell.send_key("down")

    def down_a_row(rest: str) -> bool:
        start, _, end, *lasting = rest.split()
        assert lasting[::2] == ["in", "ms"], f"the glide said no length: {rest}"
        assert 0 < int(lasting[1]) <= GLIDE, f"the glide lands past 50 ms of the key: {rest}"
        return int(end) > int(start)

    shell.reported("glides ", down_a_row, since)
    since = shell.mark()
    shell.send_chars("u")
    shell.launcher_until(lambda shown: shown["typed"] == "ou")
    said, _ = shell.said_since(since)
    shell.close_launcher()
    glided = [report for report in said if report.startswith("glides ")]
    assert not glided, f"the selection glided as a letter made the rows: {glided}"


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
    built, rounds = [], []
    for place in ("Desktop", "Machine") * 2:
        shell.type_in(place.lower(), selecting=place)
        before = int(laid_out(shell)["built"])
        shell.step_in()
        shell.launcher_until(functools.partial(stands_in, place))
        shell.normal()
        laid = laid_out(shell)
        built.append(int(laid["built"]) - before)
        rounds.append((place, laid))
        stepped_out(shell)
    shell.close_launcher()
    # The second round's steps built nothing.
    assert built[2:] == [0, 0], f"rows built by each step, Desktop then Machine twice: {built}"
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


DRAWER_SAID = re.compile(
    r'launcher: drawer for "([^"]+)" presented (\d+) ms from the key: a median (\d+) ms '
    r"before it followed, (\d+) until its frame began and (\d+) from its frame's start"
)
# Where a drawer's time from its frame's start went, where the render thread timed its swap.
DRAWER_SWAPPED = re.compile(r"from its frame's start \((\d+) to its swap and (\d+) after it\)")
DRAWERS_SAID = re.compile(r"launcher: \d+ drawers? in \S+ presented a median \d+ ms from the key")


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


def said_once(shell: Shell, since: int, line: re.Pattern[str]) -> str:
    """Give the shell's log since `since` once it holds `line`, else as the wait ends."""
    said = ""
    deadline = time.monotonic() + launcher_on_niri.WAITING.total_seconds()
    while not line.search(said) and time.monotonic() < deadline:
        said = shell.log.read_bytes()[since:].decode(errors="replace")
        time.sleep(launcher_on_niri.POLL)
    return said


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
    to the top, and both were put back, under any glide already running, so the highlight
    went its own way and the list stopped short of its first row (Joel, 2026-11-08).
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
    said, _ = shell.said_since(since)
    now = shell.showing()["selected"]["name"]
    shell.close_launcher()
    shell.close(window)
    glided = [report for report in said if report.startswith("glides ")]
    assert not glided, f"the selection glided with no key: {glided}"
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


def test_the_held_notices_are_vims_quickfix_list(shell: Shell) -> None:
    """The held notices are Vim's quickfix list, as the vi-paradigm spec's Notices sets out.

    The cursor starts on the card in front, the oldest, and the newest is line 1: k and j
    move, G and gg go to the ends, a count goes before a motion. V and k choose three, d is
    done with them, u brings them back, q says it does nothing, Escape cancels a count, and
    :q lets them go. Each key's act is told by what the list says it did.
    """
    # They stay a minute, so they're all still there when the keys come, the oldest in front.
    for summary in ["Alpha", "Bravo", "Charlie"]:
        with shell.told(f"shows card {summary}"):
            shell.as_account(f"notify-send --expire-time=60000 '{summary}' 'one of three'")
    with shell.told("keys notices"), shell.told("notice cursor Alpha"):
        shell.take_notices()
    moves = [
        (["k"], "Bravo"),
        (["k"], "Charlie"),
        (["shift-g"], "Alpha"),
        (["g", "g"], "Charlie"),
        (["2", "j"], "Alpha"),
    ]
    for keys, cursor in moves:
        with shell.told(f"notice cursor {cursor}"):
            for key in keys:
                shell.send_key(key)
    with shell.told("notice selects 1"):
        shell.send_chars("V")
    with shell.told("notice selects 3"):
        shell.send_chars("kk")
    # A second d within 300 ms joins visual's first, which tests/notice-keys.qml holds: a
    # loaded builder can take longer than that to draw the first, so here d is pressed once.
    with shell.told("notice done 3"):
        shell.send_chars("d")
    with shell.told("notice brought back 3"):
        shell.send_key("u")
    with shell.told("notice says q does nothing \u00b7 :q closes"):
        shell.send_key("q")
    # Back, the cursor is on the first of them, Charlie. Two moves up from Alpha would reach
    # Charlie again, so Bravo shows the count went with Escape.
    with shell.told("notice cursor Alpha"):
        shell.send_key("shift-g")
    with shell.told("notice cursor Bravo"):
        for key in ["2", "esc", "k"]:
            shell.send_key(key)
    with shell.told("notice mode command"):
        shell.send_chars(":")
    with shell.told("lets notices go"):
        shell.send_chars("q")
        shell.send_key("ret")
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


def test_a_terminal_is_shown_a_place_by_its_names_and_a_miss_the_nearest(shell: Shell) -> None:
    """`launcher show` is answered any place by the names its steps say, and a miss the nearest.

    The shell walks from home by its own rows (Launcher.qml's shownToTerminal), each step a
    name in any case or the start of only one (launcher/Showing.js, whose cases are
    tests/showing.qml), so a terminal says what the launcher would, a thing's own acts apart
    from its rows. The launcher stays closed: a terminal's asking never opens it.
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
    grinning = shown("desktop", "emoji", "grinning face")
    assert "Copy" in [row["name"] for row in grinning["acts"]], grinning
    assert "Copy" not in [row["name"] for row in grinning["rows"]], grinning
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
    as its things (tests/desktop/stand-in-meaning.py), and a spot it says is meant is added below
    what the letters found as typing would find it, saying where it lives and that it was found
    by its meaning. That it stands where its place would have, and is meant once by its name, is
    tests/meaning.qml's to prove; this is the wiring.
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
    assert re.fullmatch(rf"Reboot the Machine{closing}\? \(y/n\)", reboot["asks"]), reboot
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
        assert floating["asks"] == "Float Plover Notes? (y/n)", floating
        assert (listed("Plover Notes") or {}).get("is_floating") is False, "Float was done"
        closing = done("desktop", "windows", "plover notes", "close")
        assert closing["asks"] == "Close Plover Notes? (y/n)", closing
        assert listed("Plover Notes") is not None, "Close was done before it was sure"
        closed = done(path=closing["path"], sure=True)
        assert closed["done"] is True, closed
        assert closed["in"] == ["Desktop", "Windows", "Plover Notes", "Close"], closed
        until(lambda: listed("Plover Notes") is None, "Its closing")
    finally:
        for window in (kestrel, plover):
            if listed(window["title"]) is not None:
                shell.close(window)


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
    the contract's files as they're written here (tests/warp-cli-stand-in.sh). What each
    figure reads as, a step in, is tests/answers.qml's, over the record warp prints.
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
    tests/work-network-ails.qml's; this is the wiring, warp to the margin to Now.
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
    warp-cli through warp (tests/warp-cli-stand-in.sh).
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
    earlier case changed the link; warp-cli's own words for it come last (WarpDrawer.js).
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
        assert body.endswith(", with a 127 ms round trip."), str(drawer)
        facts = {fact["label"]: fact["value"] for fact in drawer["facts"]}
        assert facts["State"] == "connected", str(facts)
        assert facts["Round Trip"] == "127 ms", str(facts)
        assert list(facts)[-2:] == ["Status", "Reason"], str(facts)
        assert facts["Reason"] == "NetworkHealthy", str(facts)
    finally:
        warp_forgotten(said)


def test_the_work_network_says_whats_waiting_on_the_way_as_the_shaper_hears_it(
    shell: Shell,
) -> None:
    """The Work Network's Waiting says what the shaper heard, and unknown once it stops.

    warp reads the shaper's status.json with each read of the link (its waiting.rs), and the
    place says how long past the way at rest a question through the VPN took, after the
    Round Trip. The shaper's file goes as it stops, which is unknown, never none. A
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
    finally:
        shutil.rmtree(shaper, ignore_errors=True)
        warp_forgotten(said)


def test_network_says_how_warp_stands_before_its_link_is_read(shell: Shell) -> None:
    """The Work Network's row says how WARP stands at once, as the margin heard it, not "...".

    The margin watches how WARP stands all the time (WarpState.qml), so the row waiting on
    the link's first read says it rather than an ellipsis: degraded here, and with no round
    trip beside it, since none is read yet. Opening and closing the launcher first forgets
    what an earlier case's terminal ask kept (Warp.qml's onOpenChanged).
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
            report.startswith("holds margin") and "tunnel" in report.split()
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
        assert asking["asks"].endswith("? (y/n)"), asking
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
    (tests/desktop/stand-in-eero.py), whose console is named game console 1.
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


def test_an_opening_is_timed_from_the_chord_that_asked_for_it(shell: Shell) -> None:
    """An opening is timed from the chord that asked for it, as launcher-say says it ran.

    Joel feels the time from his press, and a shell busy with other work hears the ask late,
    which timing from the ask can't show (WCJQSG, 2026-11-10: "to its frame" isn't what he
    feels; he feels how long until it's usable). So a chord said to have run a while ago
    makes an opening at least that long from it.
    """
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


def test_an_opening_is_usable_only_once_the_shell_is_free_after_its_chord(shell: Shell) -> None:
    """An opening is usable only once the shell's thread answers on time, however soon it drew.

    Joel's openings logged 437 ms to their first frame and showed him the launcher 7 s after his
    chord, once the shell's thread came free (Joel, 2026-11-10: "It takes 7 seconds for me to see
    the launcher window"). So a shell held stopped just after a chord says the opening wasn't
    usable until at least that long after it.
    """
    with socket.socket(socket.AF_UNIX) as said:
        said.connect(str(Path(os.environ["XDG_RUNTIME_DIR"]) / "launcher.sock"))
        said.sendall(f"toggle @{time.time():.6f}\n".encode())
    with shell.frozen():
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


def screen() -> bytes:
    """Give niri's screen as it stands, through its screencopy, as raw pixels."""
    return subprocess.run(
        ["grim", "-t", "ppm", "-"],  # noqa: S607, the check gives grim
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
    before = screen()
    chord = time.time()
    chord_at = time.monotonic() * 1000
    with socket.socket(socket.AF_UNIX) as said:
        said.connect(str(Path(os.environ["XDG_RUNTIME_DIR"]) / "launcher.sock"))
        said.sendall(f"toggle @{chord:.6f}\n".encode())
    with shell.frozen():
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
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    before = screen()
    since = shell.mark()
    shell.as_account("wtype w")
    deadline = time.monotonic() + launcher_on_niri.WAITING.total_seconds()
    seen, unchanged = first_changed(before, deadline)
    assert seen >= 0, "niri's screen never showed what the key found"
    presented: list[Json] = []
    while not presented and time.monotonic() < deadline:
        presented = json.loads(shell.as_account(shell.ipc("keysPresented")))
        time.sleep(launcher_on_niri.POLL)
    assert presented, "the key was never said presented"
    key = presented[-1]
    lead = COPIED_BEFORE_PRESENTED.total_seconds() * 1000
    assert key["from"] <= key["at"] <= seen + lead, (
        f"said {key}; niri's screen showed it at {seen:.0f}"
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
    drawing, syncing, rendering, heard = (int(part) for part in drew.groups())
    assert abs(syncing + rendering + heard - drawing) <= ROUNDED, (
        f"{drawing} ms drawing, said as {syncing} syncing, {rendering} rendering"
        f" and {heard} before the shell heard it"
    )
    # A letter sets no glide going, so it landed at its first frame.
    landed = json.loads(shell.as_account(shell.ipc("keysLanded")))
    assert key in landed, f"the letter {key} didn't land at its first frame: {landed}"
    shell.close_launcher()


def test_a_key_that_glides_lands_once_its_glide_has_ended(shell: Shell) -> None:
    """An arrow that glides the selection is seen only at the frame after its glide ends.

    A highlight still moving isn't yet on its row, so a key that sets the glide going counts
    as landed at the first frame niri presented once the glide ran its length (VKCF3Z,
    2026-11-10): never before its first frame, and no sooner than its press and the glide. On
    a builder slow enough that the first frame comes after the glide's length, the two are the
    one frame, as a lent builder showed at 111 ms (RCTRF8, 2026-11-11).
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
    since = shell.mark()
    shell.send_key("down")
    lasting: list[int] = []

    def down_a_row(rest: str) -> bool:
        *_, length, _ = rest.split()
        lasting.append(int(length))
        return True

    shell.reported("glides ", down_a_row, since)
    deadline = time.monotonic() + launcher_on_niri.WAITING.total_seconds()
    first: list[Json] = []
    landed: list[Json] = []
    while not landed and time.monotonic() < deadline:
        first = json.loads(shell.as_account(shell.ipc("keysPresented")))
        glided = {key["from"] for key in first[typed:]}
        landed = [
            key
            for key in json.loads(shell.as_account(shell.ipc("keysLanded")))
            if key["from"] in glided
        ]
        time.sleep(launcher_on_niri.POLL)
    shell.close_launcher()
    assert landed, f"the arrow never landed: {first}"
    assert len(landed) == 1, f"the arrow landed more than once: {landed}"
    key, there = first[typed], landed[0]
    assert there["at"] >= key["at"], f"the arrow landed before its first frame: {key}, {there}"
    assert there["at"] - there["from"] >= lasting[-1], (
        f"the arrow landed {there['at'] - there['from']:.0f} ms from its press,"
        f" before its {lasting[-1]} ms glide ended"
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
    of what it watches, and any other ask with none.
    """

    def __init__(self, path: Path) -> None:
        """Listen on `path`, holding nothing yet."""
        self.path = path
        self.held: dict[str, Json] = {}
        # Each client watching, by what it watches (StandIn.theirs).
        self.clients: dict[socket.socket, str] = {}
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
        """Give what a watch asks for: its one kind, "place:" a place's served rows, "" all.

        A place's ready rows watch a list of kinds, its rows and those beside them, narrowed to
        the place (ReadyRows.qml), as Power's rows and headline are watched of the machine daemon.
        """
        kind = asked.get("kind")
        if isinstance(kind, str):
            return kind
        return f"place:{asked['place']}" if "place" in asked else ""

    @staticmethod
    def theirs(watching: str, record: Json) -> bool:
        """Give whether a client watching `watching` is told of `record`."""
        if watching.startswith("place:"):
            # A served record's key names its place second, "row:power:charge", "head:power".
            key = str(record["key"])
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
                elif asked.get("ask") == "watch" and asked.get("id") == "all":
                    self.write(client, {"id": "all"}, watching=self.watched(asked))
                else:
                    self.write(client, {"id": asked.get("id"), "records": []})

    def write(self, client: socket.socket, said: Json, *, watching: str | None = None) -> None:
        """Send one line to a client, a watch with every record held, which it's told of after.

        A watch gets only what it watches (StandIn.theirs). Under the lock, so no change is told
        between a watch's records and its being told.
        """
        with self.lock:
            if watching is not None:
                records = [record for record in self.held.values() if self.theirs(watching, record)]
                said = {**said, "records": records}
                self.clients[client] = watching
            client.sendall((json.dumps(said) + "\n").encode())

    def told(self, changed: list[Json], gone: list[str]) -> float:
        """Tell each client watching a change of its kind, and give when, in monotonic ms."""
        with self.lock:
            for record in changed:
                self.held[record["key"]] = record
            for key in gone:
                self.held.pop(key, None)
            at = time.monotonic() * 1000
            for client, watched in list(self.clients.items()):
                theirs = [record for record in changed if self.theirs(watched, record)]
                said = {"id": "all", "changed": theirs, "gone": gone}
                try:
                    client.sendall((json.dumps(said) + "\n").encode())
                except OSError:
                    # A connection the shell let go, as Daemon.qml does once it's refused.
                    del self.clients[client]
        return at

    def watching(self, watched: str, deadline: float) -> bool:
        """Give whether a client watches `watched` by `deadline`, so what it's told is heard."""
        while time.monotonic() < deadline:
            with self.lock:
                if watched in self.clients.values():
                    return True
            time.sleep(launcher_on_niri.POLL)
        return False

    def close(self) -> None:
        """Stop listening, let every client go and take the socket away, as a daemon stopped."""
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
    served = StandIn(Path(os.environ["XDG_RUNTIME_DIR"]) / "daemons" / "quests.sock")
    served.told([StandIn.row(0, "Kestrel 0")], [])
    yield served
    served.close()


def battery(pct: int) -> list[Json]:
    """Give Power's headline and Charge row at `pct`, as the machine daemon serves them.

    The shapes are power_rows.rs's, in the crate's wrap, as tests/power.qml serves them too.
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
    it no sooner than the frame before the time said (COPIED_BEFORE_PRESENTED), and no picture
    after it lacks it.
    """
    after = quests_shown(shell)
    before = screen()
    written = quests.told(added(), [])
    deadline = time.monotonic() + launcher_on_niri.WAITING.total_seconds()
    seen, unchanged = first_changed(before, deadline)
    assert seen >= 0, "niri's screen never showed the change"
    presented = updates_presented(shell, after, deadline)
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
    saying what it does, its key, where it lands and the last one there (VKCF3Z, 37e49991).
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
