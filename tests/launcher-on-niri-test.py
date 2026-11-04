"""The launcher answers what's typed into it, on the account's whole shell on a nested niri.

modules/quickshell/checks.nix runs this through pytest inside tests/nested-niri.sh, with the
shell launcher_on_niri.py starts once a run. Each case here needs only the launcher and what
it's typed, so none boots a machine; a case needing the machine's battery, lights, services,
other programs or niri's binds stays in tests/desktop/. Every test starts with the launcher
closed and leaves it so, so any test runs alone.
"""

from __future__ import annotations

import json
import re
import subprocess
from typing import TYPE_CHECKING

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


def now(form: str) -> str:
    """Give the time now, as date gives it in this form."""
    done = subprocess.run(["date", form], capture_output=True, text=True, check=True)  # noqa: S603, S607
    return done.stdout.strip()


def copied(shell: Shell, typed: str, expected: str) -> None:
    """Ask the launcher, take its answer with Return, and check that it copied this."""
    shell.answer(typed)
    # It says so as it closes.
    with shell.told("parted copied"):
        shell.send_key("ret")
    shell.launcher_closed()
    assert shell.copied() == expected


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
    shell.send_key("esc")
    # The zones come from the list built from tzdata, so this proves it's
    # there. The minute may turn while it's asked, so either will do.
    before = now("-u +%H:%M")
    shell.answer("time in utc")
    shell.send_key("ret")
    shell.launcher_closed()
    assert shell.copied() in [before, now("-u +%H:%M")]
    # A theme's colour is found by its name, as the theme exports it.
    theme = json.loads(shell.as_account("cat $XDG_CONFIG_HOME/jitsusama/theme.json"))
    copied(shell, "colour accent", theme["colours"]["accent"].lower())
    # Slack takes its theme only by hand, so the launcher copies it,
    # in the theme's own colours.
    slack = ["accent", "selection", "success", "warning"]
    copied(shell, "slack theme", ",".join(theme["colours"][colour] for colour in slack))
    # The build has no network, so what has to ask one says it couldn't,
    # and nothing breaks. Each waits for its answer to settle, failed;
    # asked again while that failure stands, it says so at once. When a
    # failure is asked again is the launcher's answers check's to say.
    for typed in ["8.8.8.8", "weather tokyo", "100 usd in cad", "address"]:
        shell.answer(typed)
        shell.send_key("esc")
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
    shell.send_key("esc")
    shell.launcher_closed()
    # The rain is the weather's to answer: here asks where here is, since
    # nothing here says, and a place's rain asks the same forecast. That
    # the next rain is said from the hours is the launcher's rain check's
    # to show.
    shell.answer("rain")
    shell.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "where?")
    shell.send_key("esc")
    shell.launcher_closed()
    shell.answer("rain in tokyo")
    shell.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name") == "Open-Meteo didn't answer."
            and shown["footer"] == ""
        )
    )
    shell.send_key("esc")


def test_the_date_is_copied_and_the_power_acts_ask_before_they_act(shell: Shell) -> None:
    """The date is copied, the power acts ask first, and a bind's words find its action."""
    # The date in full, as the clock has it; the day may turn while it's
    # asked, so either will do.
    before = now("+%A %-d %B %Y")
    shell.answer("date")
    shell.send_key("ret")
    shell.launcher_closed()
    assert shell.copied() in [before, now("+%A %-d %B %Y")]
    # The power actions answer to their names, and only those; Escape,
    # never Return, since Return would do them.
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
        shell.send_key("esc")
        shell.launcher_closed()
    # Asked to be sure, the foot says Log Out's verb whole, its first
    # word alone saying nothing.
    shell.answer("log out")
    shell.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Log Out")
    shell.send_key("ret")
    shell.launcher_until(
        lambda shown: "\u21b5 log out" in (shown["selected"] or {}).get("told", [])
    )
    shell.send_key("esc")
    shell.launcher_closed()
    # A key's action, by the other words its bind lists.
    shell.answer("fullscreen")
    shell.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name")
            == "Fill the screen with the window, or put it back"
            and shown["kind"] == "shortcut"
        )
    )
    shell.send_key("esc")
    shell.launcher_closed()


def test_the_power_acts_are_found_by_their_names_and_by_the_names_they_had(shell: Shell) -> None:
    """The power acts are found by their names, and by the names they had."""
    # Short names, each found by the start Joel types; Escape, never
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
        shell.send_key("esc")
        shell.launcher_closed()
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
    shell.send_key("esc")
    shell.launcher_closed()


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
    shell.send_key("esc")
    shell.launcher_closed()


def test_no_answer_raised_an_error_and_quickshell_warned_of_nothing(shell: Shell) -> None:
    """No answer raised an error, and Quickshell warned of nothing it drew."""
    log = shell.log.read_text(errors="replace").splitlines()
    errors = [line for line in log if "ERROR" in line or "Error:" in line]
    assert errors == [], f"an answer raised an error: {errors}"
    # Quickshell colours its levels and categories even in its log, so a
    # colour code may stand between the two words.
    scenes = [line for line in log if re.search("WARN[^ ]* scene", line)]
    assert scenes == [], f"Quickshell warned of a scene: {scenes}"
