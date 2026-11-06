"""The launcher and the held notices answer what's typed, on the account's whole shell on niri.

modules/quickshell/checks.nix runs this through pytest inside tests/nested-niri.sh, with the
shell launcher_on_niri.py starts once a run. Each case here needs only the launcher or the
notices, what they're typed and what they draw, so none boots a machine; a case needing the
machine's battery, lights, services, other programs or niri's binds pressed stays in
tests/desktop/. Every test starts with the
launcher closed and leaves it so, so any test runs alone.
"""

from __future__ import annotations

import json
import re
import subprocess
import time
from datetime import timedelta
from typing import TYPE_CHECKING, cast

import launcher_on_niri
import pytest

if TYPE_CHECKING:
    from collections.abc import Iterator

    from launcher_on_niri import Json, Shell


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
    shell.launcher_until(lambda shown: "GiB used" in (shown["selected"] or {}).get("name", ""))
    shell.normal()
    assert "y copy" in shell.showing()["selected"]["told"], shell.showing()["selected"]
    since = shell.mark()
    shell.send_chars("yy")
    shell.reported("yanked 1", lambda rest: rest == "", since)
    assert "GiB used" in shell.copied(), shell.copied()
    shell.close_launcher()
    # A sum holds nothing to go into, so its Return copies it still.
    copied(shell, "12*7", "84")


def test_the_date_is_copied_and_the_power_acts_ask_before_they_act(shell: Shell) -> None:
    """The date is copied, the power acts ask first, and a bind's words find its action."""
    # The date in full, as the clock has it; the day may turn while it's
    # asked, so either will do.
    before = now("+%A %-d %B %Y")
    assert yanked(shell, "date") in [before, now("+%A %-d %B %Y")]
    # The power actions answer to their names, and only those; closed by
    # :q, never Return, since Return would do them.
    for typed, action, verb in [
        ("power off", "Power Off", "Power Off"),
        ("reboot", "Reboot", "Reboot"),
        ("log out", "Log Out", "Log Out"),
    ]:
        shell.answer(typed)
        # Each wait ends before the loop moves on, so it reads this pass's action.
        shell.launcher_until(
            lambda shown: (
                (shown["selected"] or {}).get("name") == action  # noqa: B023
                and shown["kind"] == "session"
            )
        )
        # Return asks rather than doing it: it holds the action, its
        # one verb saying what a second Return would do and what it
        # closes.
        shell.send_key("ret")
        shell.launcher_until(
            lambda shown: (
                shown["held"] == [action]  # noqa: B023
                and re.fullmatch(
                    f"{verb} Now(, Closing (the Window|\\d+ Windows))?",  # noqa: B023
                    (shown["selected"] or {}).get("name", ""),
                )
                is not None
            )
        )
        shell.close_launcher()
    # Asked to be sure, the foot says Log Out's command whole, since it
    # ends the session and Return only writes it on the : line, banged:
    # the step was the asking, so the line runs past it.
    shell.answer("log out")
    shell.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Log Out")
    shell.send_key("ret")
    shell.launcher_until(
        lambda shown: "\u21b5 :logout!" in (shown["selected"] or {}).get("told", [])
    )
    # Written and left, the line goes with the launcher, so the next opening
    # is on the find, not on a line still held.
    shell.send_key("ret")
    shell.launcher_until(lambda shown: (shown["command"] or {}).get("line") == "logout!")
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
        ("keep", "Keep Awake", "start"),
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
TURNING = timedelta(seconds=70)


def test_the_closed_launcher_draws_nothing_as_the_time_it_would_show_turns(shell: Shell) -> None:
    """The closed launcher draws nothing as a minute turns, though its foot says the time.

    Qt draws a window for any change inside it, clear or hidden alike, so the closed
    launcher drew a frame nobody saw each time its foot's facts changed: the time each
    minute, the lag, the volume (2026-11-01). The sandbox has no volume to change, so the
    minute and the lag are what's watched here.
    """
    frames = f"qs -p {shell.folder} ipc call frames"
    turning = time.monotonic() + TURNING.total_seconds()
    while int(now("+%S")) < FROM_SECOND:
        assert time.monotonic() < turning, f"the clock never reached {FROM_SECOND} s past a minute"
        time.sleep(0.5)
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
    typed_at_once(shell, "'needs you' -k Return")
    shell.launcher_until(lambda shown: shown["path"][-1:] == ["Needs You"])
    shell.close_launcher()


def laid_out(shell: Shell) -> Json:
    """Give where the launcher draws each word, once two reads in a row agree."""
    was = None
    for _ in range(20):
        now = json.loads(shell.as_account(shell.ipc("laidOut")))
        if now == was:
            return cast("Json", now)
        was = now
    return cast("Json", was)


def test_the_selection_glides_to_its_row_and_typing_never_glides(shell: Shell) -> None:
    """An arrow glides the selection to its row; a letter, making rows afresh, never does."""
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    shell.type_in("o")
    # A row to glide from and one to glide to.
    shell.launcher_until(lambda shown: len(shown["rows"]) > 1)
    since = shell.mark()
    shell.send_key("down")

    def down_a_row(rest: str) -> bool:
        start, _, end = rest.split()
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
    shell.send_key("up" if shell.showing()["rows"].index(title) > 0 else "down")
    shell.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Edgewise")
    laid = json.loads(shell.as_account(shell.ipc("laidOut")))
    row = next(row for row in laid["rows"] if row["reads"].startswith(title))
    shell.close_launcher()
    shell.close(long)
    shell.close(short)
    assert named(row["words"])["x"] == start, row


def test_the_held_notices_are_vims_quickfix_list(shell: Shell) -> None:
    """The held notices are Vim's quickfix list, as the vi-paradigm spec's Notices sets out.

    The cursor starts on the card in front, the oldest, and the newest is line 1: k and j
    move, G and gg go to the ends, a count goes before a motion. V and k choose three, d is
    done with them and a second d is taken with it, u brings them back, q says it does
    nothing, Escape cancels a count, and :q lets them go. Each key's act is told by what the
    list says it did.
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
    # The second d, within the wait, joins visual's first rather than waiting on its own.
    with shell.told("notice done 3"):
        shell.send_chars("dd")
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
