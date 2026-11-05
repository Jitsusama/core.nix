"""The desktop test's launcher finds by name area.

Here the launcher finds what Joel types: each thing once, by every name it had, emoji in
their place, a category and a step by their words, and the field's keys. What it answers with
no machine is tests/launcher-on-niri-test.py's, on a nested niri in seconds; only its photographs
are here.

tests/desktop.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and its
opening comment says what the test proves. Every test asks for the desktop, started once a run
(desktop_session.py), and waits at its start for what it needs, so any test runs alone.
"""

from __future__ import annotations

import datetime as dt
import json
import re
from typing import TYPE_CHECKING, Any, cast

import desktop_session
import pytest
from desktop_session import LAUNCHER_OPEN, ROWS_LOOKED_AT, STARTING, charged

if TYPE_CHECKING:
    from collections.abc import Callable, Iterator

    from desktop_session import Desktop, Json
    from test_driver.machine import BaseMachine

# Enough rows for a screen's moves to go a screen and more: the launcher shows eight at once.
MANY_ROWS = 20


@pytest.fixture(scope="module")
def desktop(machine: BaseMachine, values: dict[str, Any]) -> Desktop:
    """Give the desktop, started once a run."""
    return desktop_session.started(machine, cast("desktop_session.Values", values))


@pytest.fixture(autouse=True)
def launcher_closed_after(desktop: Desktop) -> Iterator[None]:
    """Close the launcher after each test, so one failing with it open fails alone."""
    yield
    desktop.launcher_left_closed()


def field_key(field: list[Json], mark: str, word: str) -> Json:
    """Give the field's word for a key, its mark then what it does, failing if it isn't there."""
    said = [one for one in field if mark in one["text"] and one["text"].endswith(f" {word}")]
    assert len(said) == 1, (mark, word, field)
    return said[0]


def the_modes_look_as_vims(desktop: Desktop) -> None:
    """See the line in insert, then in normal by Escape, then in insert again by A.

    It starts and ends with "batt" typed, in insert, the caret at its end.
    """
    machine = desktop.machine
    # The line opens in insert, as Neovim's command line does, the foot
    # naming the mode after how things stand and the caret a bar a
    # quarter of a letter wide.
    laid = json.loads(desktop.as_account("qs ipc call launcher laidOut"))
    field = laid["field"]
    mode = next(word for word in laid["foot"] if word["text"] == "insert")
    rest = {(word["text"], word["x"]) for word in laid["foot"] if word is not mode}
    assert min(x for _, x in rest) < mode["x"], laid["foot"]
    complete = field_key(field, "Ctrl+Y", "complete")
    # It isn't flush against what's typed, and it's the field's one key in
    # insert: clearing is normal's.
    typed = next(word for word in field if word["text"] == "batt")
    assert typed["x"] < complete["x"], field
    assert not any(word["text"].endswith(" clear") for word in field), field
    assert desktop.showing()["caretShown"] is True, desktop.showing()
    # Escape gives normal: the caret one left and not drawn, since the
    # keys are on the rows, no word for the mode with the rest of the foot
    # where it was, the right arrow named beside Return now that it steps
    # in, and the left arrow's key in the field's place. It's read before the walk's
    # edits, since a wider aside they bring crowds the keys' words out
    # until the step ends; A goes back.
    machine.send_key("esc")
    desktop.launcher_until(lambda shown: (shown["mode"], shown["caret"]) == ("normal", 3))
    assert desktop.showing()["caretShown"] is False, desktop.showing()
    laid = json.loads(desktop.as_account("qs ipc call launcher laidOut"))
    held = {(word["text"], word["x"]) for word in laid["foot"]}
    assert rest <= held, (rest, laid["foot"])
    assert all("\u2192" in text for text, _ in held - rest), (rest, laid["foot"])
    assert field_key(laid["field"], "\u2190", "clear")["x"] == complete["x"], laid["field"]
    machine.send_chars("A")
    desktop.launcher_until(lambda shown: (shown["mode"], shown["caret"]) == ("insert", 4))


def walk_the_fields_keys(desktop: Desktop, charge: str) -> None:
    """Press every key the query line's insert and normal take, from "batt" typed to "battery".

    It starts with "batt" typed and the battery's answer selected, and ends with Ctrl+Y having
    completed it, in insert, as Joel would press them through niri.
    """
    machine = desktop.machine
    the_modes_look_as_vims(desktop)
    # Insert's keys are Vim's, through niri as Joel presses them
    # (QueryKeys.js holds the rest): the caret moves by the arrows, Home
    # and End, Backspace, Ctrl+W and Ctrl+U delete before it, and the
    # rows' keys are his completion menu's.

    def holds(text: str, caret: int) -> None:
        desktop.launcher_until(lambda shown: (shown["typed"], shown["caret"]) == (text, caret))

    for keys, chars, text, caret in [
        (["home"], "x", "xbatt", 1),
        (["backspace"], "", "batt", 0),
        (["end"], " sou", "batt sou", 8),
        (["ctrl-w"], "", "batt ", 5),
        (["backspace", "left"], "", "batt", 3),
        (["ctrl-u"], "bat", "batt", 3),
        (["right"], "", "batt", 4),
    ]:
        for key in keys:
            machine.send_key(key)
        machine.send_chars(chars)
        holds(text, caret)
    # Down, Tab and Ctrl+N each go to the next row, and Up, Shift+Tab and
    # Ctrl+P back.
    first = desktop.showing()["selected"]["name"]
    for ahead, back in [("down", "up"), ("tab", "shift-tab"), ("ctrl-n", "ctrl-p")]:
        machine.send_key(ahead)
        desktop.launcher_until(lambda shown: shown["selected"]["name"] != first)
        machine.send_key(back)
        desktop.launcher_until(lambda shown: shown["selected"]["name"] == first)
    machine.send_key("esc")
    desktop.launcher_until(lambda shown: (shown["mode"], shown["caret"]) == ("normal", 3))
    # Letters type nothing there, so u brings back only what the left
    # arrow took, the selection staying on the battery.
    machine.send_chars("z")
    machine.send_key("left")
    desktop.launcher_until(lambda shown: shown["typed"] == "")
    machine.send_chars("u")
    desktop.launcher_until(
        lambda shown: (
            shown["typed"] == "batt"
            and shown["kind"] == "answer"
            and (shown["selected"] or {}).get("name", "").startswith(f"{charge}%")
        )
    )
    # i a I A go into insert at the caret, after it, at the start and at
    # the end, each Escape back putting the caret one left; cc and S clear
    # the line into insert.
    for into, caret, after in [("i", 3, 2), ("a", 3, 2), ("I", 0, 0), ("A", 4, None)]:
        machine.send_chars(into)
        desktop.launcher_until(
            lambda shown: (shown["mode"], shown["caret"]) == ("insert", caret)  # noqa: B023
        )
        if after is not None:
            machine.send_key("esc")
            desktop.launcher_until(
                lambda shown: (shown["mode"], shown["caret"]) == ("normal", after)  # noqa: B023
            )
    for change in ["cc", "S"]:
        machine.send_key("esc")
        desktop.launcher_until(lambda shown: shown["mode"] == "normal")
        machine.send_chars(change)
        desktop.launcher_until(lambda shown: (shown["mode"], shown["typed"]) == ("insert", ""))
        machine.send_chars("batt")
        holds("batt", 4)
    desktop.launcher_until(
        lambda shown: (
            shown["kind"] == "answer"
            and (shown["selected"] or {}).get("name", "").startswith(f"{charge}%")
        )
    )
    machine.send_key("ctrl-y")
    desktop.launcher_until(lambda shown: shown["typed"] == "battery")


def test_the_launcher_lists_each_thing_once_for_everything_joel_types(desktop: Desktop) -> None:
    """The launcher lists each thing once, for everything Joel types."""
    machine = desktop.machine

    def settled_on(query: str) -> Json:
        """Wait until the launcher holds exactly this query, with Return's row settled.

        Give what it shows then. Exactly, so a left arrow lost on a busy host can't pass the
        last query's letters off as this one's. It's asked first, since it has mostly settled
        by then, and asked again after a report says it has: a report is made as each of its
        properties changes, so one may pair the new query with the last one's rows.
        """

        def holds(shown: Json) -> bool:
            row = shown["selected"]
            return bool(shown["typed"] == query and (row is None or row["settled"]))

        shown = desktop.showing()
        while not holds(shown):
            desktop.launcher_until(holds)
            shown = desktop.showing()
        return shown

    picked_in_a_week = [
        "1p", "a", "agents", "ai", "b", "bto", "btop", "ca", "caf", "caff", "came",
        "captu", "ch", "chr", "chro", "chrom", "chrome", "close", "donot", "dot", "gimp",
        "ink", "kit", "kitt", "kitty", "left", "light", "lo", "loc", "lock", "mute", "ova",
        "pow", "powe", "re", "reb", "rebo", "rest", "runs", "shrim", "sl", "sla", "slac",
        "slack", "slack theme", "soun", "spend", "spo", "spot", "spoti", "sus", "susp",
        "time", "time in gmt", "tme", "tupl", "tuple", "volume", "wifi",
    ]  # fmt: skip
    surveyed = [
        "s", "tu", "batt", "12*7", "sound", "pi", "emoji", "copy", "key", "screen",
        "notes", "power", "open", "turn",
    ]  # fmt: skip
    audited = [
        "keyboard layout", "stop rec", "tuesday", "this window", "headphones",
        "bluetooth", "clipboard", "keyboard", "dim", "keyboard light", "camera",
        "quest", "terminal", "below", "zsh",
    ]  # fmt: skip
    listed_twice: list[str] = []
    looked_at = 0
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    for query in picked_in_a_week + surveyed + audited:
        # Each query is typed in insert, and the left arrow clears it in normal.
        desktop.insert()
        machine.send_chars(query)
        shown = settled_on(query)
        things: list[Json] = shown["things"]
        looked_at += len([thing for thing in things if thing["is"]])
        for field in ("is", "reads"):
            given = [thing[field] for thing in things if thing[field]]
            again = sorted({one for one in given if given.count(one) > 1})
            if again:
                listed_twice.append(f"{query!r}: {field} {again}")
        # None of these leaves a step held, but one that did would
        # have the next typed inside it, so it's closed and opened.
        if shown["held"]:
            desktop.close_launcher()
            desktop.as_account("qs ipc call launcher toggle")
            desktop.launcher_ready()
        else:
            desktop.normal()
            machine.send_key("left")
    desktop.close_launcher()
    assert listed_twice == [], "a thing was listed twice"
    # Most of those find something, so the rows were looked at.
    assert looked_at > ROWS_LOOKED_AT


def test_a_collection_or_half_a_word_is_asked_and_the_fields_keys_edit_it(desktop: Desktop) -> None:
    """A collection is found by its name, half a word asks, and the field's keys edit it."""
    machine = desktop.machine
    charge = charged(desktop)
    # A collection is found by its name, and Return goes straight into
    # it.
    # Beside the field, it says what sort of thing each is.
    for typed, collection in [("sound", "Sound"), ("notices", "Notices")]:
        desktop.answer(typed)
        # The wait ends before the loop moves on, so it reads this pass's name.
        desktop.launcher_until(
            lambda shown: (
                (shown["selected"] or {}).get("name") == collection  # noqa: B023
                and shown["kind"] == "collection"
            )
        )
        desktop.close_launcher()
    # Half a word asks its question too: "batt" is the battery, and
    # Ctrl+Y finishes the word, the field saying so at its right edge.
    # A copy kept from before saying "battery", a later word, never takes
    # the answer's place, as the battery's own copied answer did when it
    # said "on battery".
    desktop.as_account("echo -n 'kept from the battery' | wl-copy > /dev/null 2>&1")
    desktop.answer("kept")
    desktop.launcher_until(
        lambda shown: any(thing["is"] == "copy:kept from the battery" for thing in shown["things"])
    )
    desktop.close_launcher()
    desktop.answer("batt")
    desktop.launcher_until(
        lambda shown: (
            shown["kind"] == "answer"
            and (shown["selected"] or {}).get("name", "").startswith(f"{charge}%")
        )
    )
    walk_the_fields_keys(desktop, charge)
    machine.send_key("ret")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10))
    assert desktop.as_account("wl-paste --no-newline").startswith(f"{charge}%, ")
    # Each answer by its one word, and an old one asks nothing.
    # "lag" answers from lag-watch's last ten seconds.
    for typed, said in [
        ("battery", f"{charge}%"),
        ("heat", ""),
        ("memory", ""),
        ("lag", ""),
    ]:
        desktop.answer(typed)
        # The wait ends before the loop moves on, so it reads this pass's answer.
        desktop.launcher_until(
            lambda shown: (
                shown["kind"] == "answer"
                and (shown["selected"] or {}).get("name", "").startswith(said)  # noqa: B023
            )
        )
        desktop.close_launcher()
    # A name starting the same way is chosen over an answer only
    # started: "pow" is Power off, by its name.
    desktop.answer("pow")
    desktop.launcher_until(
        lambda shown: (
            shown["kind"] not in ("", "answer")
            and (shown["selected"] or {}).get("name", "").lower().startswith("pow")
        )
    )
    desktop.close_launcher()


def test_the_launcher_finds_emoji_in_their_own_place_and_only_there(desktop: Desktop) -> None:
    """The launcher finds emoji in their own place, and only there."""
    machine = desktop.machine
    closed = desktop.in_session(LAUNCHER_OPEN)
    # The main search lists no emoji, only the place they're in.
    desktop.answer("tada")
    assert "Party popper" not in desktop.showing()["rows"]
    desktop.close_launcher()
    desktop.answer("emoji")
    desktop.launcher_until(lambda shown: shown["rows"] == ["Emoji"])
    machine.send_key("ret")
    desktop.launcher_until(lambda shown: shown["held"] == ["emoji"])
    # Its name capitalised, as every row's is.
    desktop.type_in("tada", selecting="Party popper")
    machine.send_key("ret")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
    assert desktop.as_account("wl-paste --no-newline") == "🎉"
    machine.send_key("meta_l-e")
    desktop.launcher_ready()
    desktop.type_in("smiling")
    desktop.photograph("launcher-emoji")
    desktop.type_in(" cat")
    desktop.photograph("launcher-emoji-all")
    machine.send_key("ret")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
    # The grinning cat with smiling eyes, first among the smiling cats.
    assert desktop.as_account("wl-paste --no-newline") == "😸"
    # Super+E opens the emoji straight away, so "kitty" is a cat there,
    # where the main search would open the terminal.
    machine.send_key("meta_l-e")
    desktop.launcher_ready()
    desktop.type_in("kitty")
    machine.send_key("ret")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
    assert desktop.as_account("wl-paste --no-newline") == "🐱"
    # Over a window, Return pastes the emoji into it as well as copying
    # it, as Super+V's copies do, and a step in are Paste and Copy.
    desktop.as_account(
        "niri msg action spawn -- kitty --title emoji-here "
        "sh -c 'stty -icanon -echo; cat > /tmp/emoji-pasted'"
    )
    machine.wait_until_succeeds(
        desktop.in_session("niri msg focused-window | grep -q emoji-here"),
        timeout=STARTING,
    )
    machine.send_key("meta_l-e")
    desktop.launcher_ready()
    desktop.type_in("tada", selecting="Party popper")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("verb") == "paste")
    desktop.step_in()
    desktop.launcher_until(lambda shown: shown["rows"] == ["Paste", "Copy"])
    with desktop.told("pastes into kitty"):
        machine.send_key("ret")
    machine.wait_until_succeeds(
        desktop.in_session("grep -q \U0001f389 /tmp/emoji-pasted"), timeout=dt.timedelta(seconds=10)
    )
    desktop.as_account("pkill -f 'cat > /tmp/emoji-pasted'")
    machine.wait_until_fails(
        desktop.in_session("niri msg windows | grep -q emoji-here"),
        timeout=dt.timedelta(seconds=30),
    )


def test_a_categorys_word_and_a_space_step_into_it_and_a_command_finishes_by_ctrl_y(
    desktop: Desktop,
) -> None:
    """A category's word and a space step into it, and a command finishes by Ctrl+Y."""
    machine = desktop.machine
    # The word and a space become a chip, the field cleared, listing
    # what it holds; what's typed then searches only inside it.
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    machine.send_chars("network ")
    desktop.launcher_until(
        lambda shown: (
            shown["held"] == ["network"]
            and shown["typed"] == ""
            and {"Address", "Speed"} <= set(shown["rows"])
        )
    )
    # What it holds is in its own order, Wi-Fi before what it's
    # connected by, then its typed commands, last, in theirs.
    held = desktop.showing()["rows"]
    ip = "ip \u27e8address or network\u27e9"
    assert held.index("Address") < held.index("Speed") < held.index(ip), held
    assert held.index(ip) < held.index("mac \u27e8hardware address\u27e9"), held
    desktop.photograph("launcher-category")
    desktop.type_in("add", selecting="Address")
    # In normal, the left arrow takes what's typed, then steps out with
    # the word put back.
    desktop.normal()
    machine.send_key("left")
    desktop.launcher_until(lambda shown: shown["held"] == ["network"] and shown["typed"] == "")
    machine.send_key("left")
    desktop.launcher_until(lambda shown: shown["held"] == [] and shown["typed"] == "network")
    desktop.close_launcher()
    # A question that starts with a category's word is still a question.
    desktop.answer("time in utc")
    desktop.launcher_until(lambda shown: shown["held"] == [] and shown["kind"] == "answer")
    desktop.close_launcher()
    # A command is in the place whose answer it gives, its form shown;
    # Ctrl+Y puts its start in the field, out at the top, where it's
    # finished.
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    machine.send_chars("network ")
    desktop.launcher_until(lambda shown: shown["held"] == ["network"] and ip in shown["rows"])
    desktop.photograph("launcher-commands")
    desktop.type_in("ip", selecting="ip \u27e8address or network\u27e9")
    machine.send_key("ctrl-y")
    desktop.launcher_until(lambda shown: shown["held"] == [] and shown["typed"] == "ip ")
    desktop.type_in("10.0.0.0/22")
    desktop.launcher_until(lambda shown: shown["kind"] == "answer")
    desktop.close_launcher()


def test_every_name_a_thing_had_still_finds_it(desktop: Desktop) -> None:
    """Every name a thing had still finds it."""
    words = json.loads(desktop.as_account("cat ~/.config/quickshell/launcher/words.json"))
    kept = [
        (old, str(entry["name"]))
        for entry in [*words["things"], *words["categories"]]
        for old in entry.get("once", [])
    ]
    # Every one the vocabulary keeps, so a rename that lost its row fails
    # here rather than with Joel: whatever it selects, a row, a spot or an
    # answer, says what it was once called. One whose thing this machine
    # hasn't got, as a VPN, finds nothing by its new name either, and is
    # passed over; the count of those found proves the walk looked.
    fewest = 15
    lost = []
    absent = []
    for old, name in kept:
        desktop.answer(old.lower())
        try:
            desktop.launcher_until(
                lambda shown: shown["footer"] == f"Once called {old}.",  # noqa: B023
                timeout=dt.timedelta(seconds=5),
            )
        except AssertionError:
            shown = desktop.showing()
            desktop.close_launcher()
            desktop.answer(name.lower())
            try:
                desktop.launcher_until(
                    lambda there: bool(there["rows"]) or there["kind"] == "answer",
                    timeout=dt.timedelta(seconds=5),
                )
                found = [thing["is"] for thing in desktop.showing()["things"][:3]]
                lost.append(f"{old} -> {name}: {shown['rows'][:5]} {shown['footer']!r}, {found}")
            except AssertionError:
                absent.append(name)
        desktop.close_launcher()
    print(f"walked {len(kept)} old names, {len(absent)} absent here: {absent}")
    assert lost == [], lost
    assert len(kept) - len(absent) >= fewest, absent


def waits_then_lets_go(desktop: Desktop, key: str, typed: str | None = None) -> None:
    """Type a key that begins a pair, and wait until it's waited for its second and let go.

    The key is as the launcher names it, typed as its own text unless told what to type. It's
    heard from before the key, since the wait can be over before one begun after it starts.
    """
    before = desktop.mark()
    desktop.machine.send_chars(key if typed is None else typed)
    begun: list[str] = []

    def let_go(said: str) -> bool:
        begun.append(json.loads(said)["begun"])
        return key in begun and begun[-1] == ""

    desktop.reported("showing ", let_go, before)


def walk_the_rows_moves(desktop: Desktop) -> None:
    """Press every move normal takes through a search's rows, as Joel would through niri.

    They're Vim's, with counts, as QueryKeys.js holds them (tests/query-keys.qml): this is
    that each reaches the rows through niri, lands where the list has its rows in view, and
    that a count shows in the foot until its move.
    """
    machine = desktop.machine
    desktop.answer("s")
    desktop.launcher_until(lambda shown: shown["count"] >= MANY_ROWS and shown["current"] == 0)
    desktop.normal()
    last = desktop.showing()["count"] - 1

    def at(row: int) -> Json:
        """Wait until this row is selected, with nothing begun, and give what's shown then."""
        desktop.launcher_until(
            lambda shown: (shown["current"], shown["counted"], shown["begun"]) == (row, "", "")
        )
        return desktop.showing()

    def press(keys: str) -> None:
        for key in keys.split():
            if len(key) == 1:
                machine.send_chars(key)
            else:
                machine.send_key(key)

    for keys, row in [("j", 1), ("down", 2), ("k", 1), ("up", 0), ("G", last), ("g g", 0)]:
        press(keys)
        at(row)
    # A count stands in the mode word's place in the foot until its move, which goes that
    # many, and before G names the row.
    machine.send_chars("3")
    desktop.launcher_until(lambda shown: shown["counted"] == "3")
    foot = json.loads(desktop.as_account("qs ipc call launcher laidOut"))["foot"]
    assert any(word["text"] == "3" for word in foot), foot
    machine.send_chars("j")
    at(3)
    for keys, row in [("1 2 k", 0), ("5 G", 4), ("2 g g", 1)]:
        press(keys)
        at(row)
    # Escape with a count begun only lets it go, as Vim's does, and the next steps back.
    machine.send_chars("5")
    desktop.launcher_until(lambda shown: shown["counted"] == "5")
    machine.send_key("esc")
    desktop.launcher_until(lambda shown: shown["counted"] == "")
    assert desktop.showing()["open"], "an Escape cancelling a count closed the launcher"
    # A g waits his timeoutlen for its second, then lets go, so a j after moves.
    waits_then_lets_go(desktop, "g")
    machine.send_chars("j")
    # Half a screen moves the rows as far as the selection, so it keeps its place in view;
    # a screen goes on a list's height, eight rows.
    start = 2
    shown = at(start)
    top = shown["inView"]["top"]
    half = (shown["inView"]["bottom"] - top + 1) // 2
    machine.send_key("ctrl-d")
    desktop.launcher_until(
        lambda shown: shown["current"] == start + half and shown["inView"]["top"] == top + half
    )
    machine.send_key("ctrl-u")
    desktop.launcher_until(
        lambda shown: shown["current"] == start and shown["inView"]["top"] == top
    )
    screen = start + 8
    for keys, row in [
        ("ctrl-f", screen),
        ("ctrl-b", start),
        ("pgdn", screen),
        ("pgup", start),
        ("ctrl-f", screen),
    ]:
        press(keys)
        at(row)
    # H M L go to the top, middle and bottom rows wholly in view, as the list has them.
    view = desktop.showing()["inView"]
    for keys, row in [
        ("H", view["top"]),
        ("L", view["bottom"]),
        ("M", view["top"] + (view["bottom"] - view["top"]) // 2),
        ("2 H", view["top"] + 1),
    ]:
        press(keys)
        at(row)
    desktop.close_launcher()


def walk_the_rows_acts(desktop: Desktop) -> None:
    """Press every act normal takes on a search's rows besides its moves, through niri.

    l steps into a row and - out of it, as his oil's do; :12 goes to the 12th row, on the
    command line in the find line's place, its menu saying where it goes, until its Return;
    and q, ZZ and :q each close the launcher.
    """
    machine = desktop.machine
    desktop.answer("emoji")
    desktop.launcher_until(lambda shown: shown["rows"] == ["Emoji"])
    desktop.normal()
    machine.send_chars("l")
    desktop.launcher_until(lambda shown: shown["held"] == ["emoji"])
    machine.send_chars("-")
    desktop.launcher_until(lambda shown: shown["held"] == [] and shown["typed"] == "emoji")
    desktop.close_launcher()
    desktop.answer("s")
    desktop.launcher_until(lambda shown: shown["count"] >= MANY_ROWS and shown["current"] == 0)
    desktop.normal()
    row = 12
    machine.send_chars(f":{row}")
    desktop.launcher_until(
        lambda shown: (
            (shown["command"] or {}).get("line") == str(row)
            and shown["command"]["menu"] == ["{n} Go to a Row"]
        )
    )
    machine.send_key("ret")
    desktop.launcher_until(
        lambda shown: (
            shown["current"] == row - 1 and shown["command"] is None and shown["mode"] == "normal"
        )
    )
    for keys in ["q", "ZZ", ":q\n"]:
        machine.send_chars(keys)
        desktop.launcher_until(lambda shown: not shown["open"])
        desktop.answer("s")
        desktop.normal()
    desktop.close_launcher()


def walk_the_rows_unsearched(desktop: Desktop) -> None:
    """Press / ? n N on a search's rows in normal, through niri, and see nothing change.

    The find line is the list's one way to find (VKCF3Z's vi-paradigm spec, The Find Line), so
    the rows have no search of their own: the selection, the mode and what's typed stay put.
    """
    machine = desktop.machine
    desktop.answer("s")
    desktop.launcher_until(lambda shown: shown["count"] >= MANY_ROWS and shown["current"] == 0)
    desktop.normal()
    machine.send_chars("j")
    desktop.launcher_until(lambda shown: shown["current"] == 1)
    machine.send_chars("/?nN", delay=desktop_session.KEY_PACE)
    # A move after them shows every key before it was taken, and took nothing.
    machine.send_chars("j")
    desktop.launcher_until(
        lambda shown: (shown["current"], shown["mode"], shown["typed"]) == (2, "normal", "s")
    )
    desktop.close_launcher()


def walk_the_rows_washed(desktop: Desktop) -> None:
    """Type a find and see the letters it matched washed in the rows drawn, the selected one's too.

    As his Neovim's hlsearch washes a match (Search), and in normal as in the find line, since a
    find stays until it's changed. A row found by another word than its name washes nothing.
    """
    desktop.answer("s")
    desktop.launcher_until(lambda shown: shown["count"] >= MANY_ROWS and shown["current"] == 0)

    def washed() -> list[Json]:
        return cast(
            "list[Json]", json.loads(desktop.as_account("qs ipc call launcher laidOut"))["rows"]
        )

    for mode in ["insert", "normal"]:
        if mode == "normal":
            desktop.normal()
        rows = washed()
        runs = [run for row in rows for run in row["washed"]]
        assert runs, f'in {mode} no row drawn for "s" washed a letter: {rows}'
        assert all(run.lower() == "s" for run in runs), f"in {mode} washed {runs}, not s"
        unwashed = [row["reads"] for row in rows if row["byName"] and not row["washed"]]
        assert not unwashed, f"in {mode} rows found by name washed nothing: {unwashed}"
        selected = next(row for row in rows if row["selected"])
        name = selected["reads"].split(" \u00b7 ")[0]
        if any(word.lower().startswith("s") for word in re.split(r"\W+", name)):
            assert selected["washed"], f"in {mode} the selected {name!r} washed nothing"
    desktop.close_launcher()


def walk_the_rows_keys(desktop: Desktop) -> None:
    """Press normal's Tab, Space ?, F1 and Space w on a search's rows, through niri.

    Tab goes to the place used last and back, as his Space Tab goes to the file used last;
    Space ? and F1 list every key in the drawer until Escape or again; Space alone waits for a
    second key and lets go; and Space w closes the launcher.
    """
    machine = desktop.machine
    desktop.answer("emoji")
    desktop.launcher_until(lambda shown: shown["rows"] == ["Emoji"])
    desktop.normal()
    machine.send_chars("l")
    desktop.launcher_until(lambda shown: shown["held"] == ["emoji"])
    machine.send_chars("-")
    desktop.launcher_until(lambda shown: shown["held"] == [] and shown["typed"] == "emoji")
    machine.send_key("tab")
    desktop.launcher_until(lambda shown: shown["held"] == ["emoji"])
    machine.send_key("tab")
    desktop.launcher_until(lambda shown: shown["held"] == [] and shown["typed"] == "emoji")

    # What the drawer draws, since its facts keep the last plate as it goes back in.
    def listed(listing: bool) -> None:
        desktop.launcher_until(
            lambda shown: (
                shown["open"]
                and shown.get("listing") == listing
                and ("Space ?  F1" in shown["drawer"]["drawnFacts"]) == listing
                and ("Insert" in shown["drawer"]["drawnFacts"]) == listing
            )
        )
        drawer = desktop.showing()["drawer"]
        assert drawer["labelsWhole"], f"a key's name is broken: {drawer}"

    machine.send_chars(" ?")
    listed(listing=True)
    machine.send_key("esc")
    listed(listing=False)
    machine.send_key("f1")
    listed(listing=True)
    machine.send_key("f1")
    listed(listing=False)
    waits_then_lets_go(desktop, "Space", " ")
    assert desktop.showing()["open"], "a Space alone closed the launcher"
    machine.send_chars(" w")
    desktop.launcher_until(lambda shown: not shown["open"])


def test_a_search_lists_each_thing_once_and_normal_moves_through_it(desktop: Desktop) -> None:
    """A search lists each thing once, and normal's keys move and act on it as Vim's do."""
    walk_the_rows_moves(desktop)
    walk_the_rows_acts(desktop)
    walk_the_rows_unsearched(desktop)
    walk_the_rows_washed(desktop)
    walk_the_rows_keys(desktop)
    # People stands alone for everyone in it; the
    # session's locking and do not disturb draw their keys' chords, so
    # those keys aren't listed again, the lock by its own name;
    # kitty draws Super+Return's;
    # and Capture holds the screenshots its keys take. Nothing is
    # found by letters scattered through it.
    stood_for = {
        "Lock the screen",
        "Hold notices, or let them show",
        "Open a terminal",
        "Take a screenshot of the screen",
        "Take a screenshot of the window",
    }

    def once(leading: list[str]) -> Callable[[Json], bool]:
        """Give a test that the rows lead with these, list nothing stood for, and home none twice.

        Two windows may share a title, told apart by where they are, so a search's rows are
        held to what stands for what; home's, with no windows but the focused one, are held to
        once.
        """

        def holds(shown: Json) -> bool:
            rows: list[str] = shown["rows"]
            return (
                rows[: len(leading)] == leading
                and (shown["typed"] != "" or len(set(rows)) == len(rows))
                and all(rows.count(name) == leading.count(name) for name in stood_for)
            )

        return holds

    # What a keyboard's caps lock says may follow "lock": that's
    # another thing. Home, with nothing typed, lists nothing twice
    # either, a collection picked often staying in its own place.
    for typed, leading in [
        ("people", ["People"]),
        ("lock", ["Lock"]),
        ("kitty", ["kitty"]),
        ("do not disturb", ["Do Not Disturb"]),
        ("capture", ["Capture"]),
        ("", []),
    ]:
        desktop.answer(typed)
        desktop.launcher_until(once(leading))
        desktop.close_launcher()
    desktop.answer("zqxj")
    desktop.launcher_until(lambda shown: shown["rows"] == [])
    desktop.close_launcher()
    # The speakers' tuning plays through them, and is no program.
    desktop.answer("volume")
    desktop.launcher_until(lambda shown: "Turn the volume up" in shown["rows"])
    assert "pebble_x_plus_tuning_output" not in desktop.showing()["rows"]
    desktop.close_launcher()


def test_a_step_finds_its_rows_by_their_hidden_words_and_its_parts_move_in_place(  # noqa: PLR0915
    desktop: Desktop,
) -> None:
    """A step finds its rows by their hidden words, and its parts move in place."""
    machine = desktop.machine
    # Inside the addresses, the tunnel is found by the kernel's name for
    # it, a word it's known by but never says, as from the top.
    machine.succeed(
        "ip tuntap add dev tailscale0 mode tun"
        " && ip address add 100.100.1.2/32 dev tailscale0 && ip link set tailscale0 up"
    )
    desktop.answer("address")
    desktop.step_in()
    desktop.launcher_until(lambda shown: "VPN" in shown["rows"])
    desktop.type_in("tailscale0", selecting="VPN")
    assert "tailscale0" not in desktop.showing()["rows"]
    # In normal, the left arrow takes what's typed away, the selection
    # staying on the VPN; u right after puts it back, and the right arrow
    # then steps into it.
    desktop.normal()
    machine.send_key("left")
    desktop.launcher_until(
        lambda shown: shown["typed"] == "" and (shown["selected"] or {}).get("name") == "VPN"
    )
    machine.send_chars("u")
    desktop.launcher_until(
        lambda shown: (
            shown["typed"] == "tailscale0" and (shown["selected"] or {}).get("name") == "VPN"
        )
    )
    desktop.step_in()
    desktop.launcher_until(lambda shown: "Address" in shown["rows"])
    # Stepping back out lands on the row it came from.
    machine.send_key("left")
    desktop.launcher_until(
        lambda shown: len(shown["held"]) == 1 and (shown["selected"] or {}).get("name") == "VPN"
    )
    desktop.close_launcher()
    machine.succeed("ip link delete tailscale0")
    # The processors, a step into Machine, are named as the thing they
    # are, and their cores, a step further, read again each second there
    # as at the top: made busy, every core says so, and the selection
    # stays on its row, which says so too, as read again rather than as
    # first shown.
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    machine.send_chars("machine ")
    desktop.launcher_until(
        lambda shown: (
            shown["held"] == ["computer"]
            and any(thing["is"] == "answer:machine:cpu" for thing in shown["things"])
        )
    )
    # Named Processor where it's held, as when stepped into; nothing in
    # Machine repeats its name.
    rows = desktop.showing()["rows"]
    assert "Processor" in rows, rows
    assert "Machine" not in rows, rows
    processors = [thing["is"] for thing in desktop.showing()["things"]].index("answer:machine:cpu")
    for _ in range(processors):
        machine.send_key("down")
    desktop.launcher_until(
        lambda shown: (shown["selected"] or {}).get("is") == "answer:machine:cpu"
    )
    desktop.step_in()
    desktop.launcher_until(
        lambda shown: shown["path"] == ["Machine", "Processor"] and "Cores" in shown["rows"]
    )
    desktop.down_to("Cores")
    desktop.step_in()
    desktop.launcher_until(lambda shown: shown["path"] == ["Machine", "Processor", "Cores"])
    cores = desktop.showing()["rows"]
    if len(cores) > 1:
        machine.send_key("down")
    chosen = cores[min(1, len(cores) - 1)]
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == chosen)
    busy = re.compile(r"Core \d+ \u00b7 ([89]\d|100)%")

    def all_busy(shown: Json) -> bool:
        """Whether every core reads busy, the step's Copy, after them, aside."""
        each = [thing["reads"] for thing in shown["things"] if thing["reads"].startswith("Core ")]
        return len(each) > 0 and all(busy.fullmatch(reads) for reads in each)

    # At rest not every core is busy, so every core saying it is below
    # is each read again.
    assert not all_busy(desktop.showing()), "the cores were busy before anything made them so"
    # A loop for each core, as the shell's own builtins, since a
    # transient unit's path has no coreutils.
    spinning = "(while :; do :; done) & " * int(machine.succeed("nproc"))
    machine.succeed(f"systemd-run --unit=busy-cores -p Nice=19 sh -c '{spinning}wait'")
    desktop.launcher_until(all_busy, timeout=dt.timedelta(seconds=60))
    shown = desktop.showing()
    assert shown["selected"]["name"] == chosen, "the selection left its row"
    assert re.search("^([89]\\d|100)%$", shown["selected"]["side"]), (
        "the selection kept its row as first read"
    )
    machine.succeed("systemctl stop busy-cores")
    # Back out, it's on the row it came from.
    machine.send_key("left")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Cores")
    desktop.close_launcher()


def test_a_question_is_asked_on_return_and_answered_where_it_was_typed(desktop: Desktop) -> None:
    """A question is asked on Return and answered where it was typed."""
    machine = desktop.machine

    def selected(shown: Json) -> Json:
        return cast("Json", shown["selected"] or {})

    def close() -> None:
        desktop.close_launcher()

    # Asked by its first word, a model gives way to what's found, so
    # "do not disturb" is Do not disturb.
    desktop.answer("do not disturb")
    first = selected(desktop.showing())
    assert first.get("name") != "Ask", f"asking a model came before the setting: {first}"
    close()

    # Ending in "?", it offers to ask, and nothing is asked until Return,
    # which keeps the launcher open while the answer is written in place:
    # its first sentence as the row's name, the whole of it below the rows.
    desktop.answer("what is a fjord?")
    offered = selected(desktop.showing())
    assert (offered.get("name"), offered.get("verb")) == ("Ask", "ask"), offered
    machine.send_key("ret")
    desktop.launcher_until(
        lambda shown: (
            shown["open"]
            and selected(shown).get("name") == "A fjord is a long, narrow inlet of the sea."
            and "Glaciers carved" in selected(shown).get("reads", "")
            and str(shown["footer"]).startswith("answered in")
            and selected(shown).get("verb") == "copy"
        )
    )
    close()

    # One the model's provider refuses says why, in the row.
    desktop.answer("why does this fail?")
    machine.send_key("ret")
    desktop.launcher_until(
        lambda shown: selected(shown).get("name") == "the model's provider refused"
    )
    close()


def test_the_answers_are_photographed_for_the_gallery(desktop: Desktop) -> None:
    """The launcher's answers are photographed for the gallery, and only there.

    What they answer is tests/launcher-on-niri-test.py's to prove, in seconds on a nested niri; a
    person still looks at how each is drawn on the laptop's screen, so the gallery keeps them.
    """
    if not desktop.values["photographs"]:
        return
    for typed, picture in [
        ("date", "launcher-date"),
        ("350 f in c", "launcher-units"),
        ("time in utc", "launcher-time"),
        ("#f53", "launcher-colour"),
        ("10.0.0.0/22", "launcher-answer"),
        ("weather tokyo", "launcher-offline"),
    ]:
        desktop.answer(typed)
        desktop.launcher_until(lambda shown: bool(shown["rows"]))
        desktop.photograph(picture)
        desktop.close_launcher()
    # Inside the time here and the date, whose parts are the clock's and the
    # calendar's, each a step in.
    for typed, first, picture in [
        ("time", "Zone", "launcher-time-inside"),
        ("date", "ISO Date", "launcher-date-inside"),
    ]:
        desktop.answer(typed)
        desktop.step_in()
        # Each wait ends before the loop moves on, so it reads this pass's part.
        desktop.launcher_until(lambda shown: first in shown["rows"])  # noqa: B023
        desktop.photograph(picture)
        desktop.close_launcher()
    # The time here counts each second while it's shown, and a tick never moves the
    # selection or changes the card's size: UTC stays selected, among as many rows.
    desktop.answer("time")
    desktop.step_in()
    desktop.launcher_until(lambda shown: "UTC" in shown["rows"])
    desktop.down_to("UTC")
    before = desktop.showing()
    was = before["selected"]["side"]
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("side") != was)
    after = desktop.showing()
    assert (after["selected"] or {}).get("name") == "UTC", f"a tick moved the selection: {after}"
    assert after["rows"] == before["rows"], f"a tick changed the rows: {before} then {after}"
    desktop.close_launcher()


def test_no_answer_raised_an_error(desktop: Desktop) -> None:
    """No answer raised an error."""
    log = desktop.shell_said("ERROR", "Error:")
    assert "ERROR" not in log, "an answer raised an error"
    assert "Error:" not in log, "an answer raised an error"


def test_quickshell_warned_of_nothing_it_drew_all_along(desktop: Desktop) -> None:
    """Quickshell warned of nothing it drew all along."""
    # Quickshell colours its levels and categories even in the journal,
    # so a colour code may stand between the two words.
    log = desktop.as_account(
        "journalctl --user -u quickshell --no-pager -o cat | grep -a -E 'WARN[^ ]* scene' || true"
    )
    assert log.strip() == "", f"Quickshell warned of a scene: {log}"
