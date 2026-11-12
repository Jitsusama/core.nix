"""The desktop test's launcher finds by name area.

Here the launcher finds what Joel types: each thing once, by every name it had, emoji in
their place, a category and a step by their words, the field's keys, and what Spotify found by
its name holds once it's read, which needs its client's desktop entry. What it answers with
no machine is tests/launcher-on-niri-test.py's, on a nested niri in seconds; only its shots
are here.

tests/desktop.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and its
opening comment says what the test proves. Every test asks for the desktop, started once a run
(desktop_session.py), and waits at its start for what it needs, so any test runs alone.
"""

from __future__ import annotations

import datetime as dt
import json
import re
import shlex
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
# How long a program's Return may take from the launcher hearing it to handing it on: less than
# the 300 ms a doubled Return is waited out (Options.keyTiming.wait), so none is.
FRESH_RETURN_MS = 300
# How many rows Ctrl+]'s look for where a row lives may list before it gives up (Launcher.qml's
# livesMost).
LIVES_MOST = 3000
# The Ctrl+] presses walk_the_rows_follows makes, each a look: two from home and one in Lock's
# own place. Ctrl+I follows the jump list, not the row, so it looks for nothing.
FOLLOWS = 3


@pytest.fixture(scope="module")
def desktop(machine: BaseMachine, values: dict[str, Any]) -> Desktop:
    """Give the desktop, started once a run."""
    return desktop_session.started(machine, cast("desktop_session.Values", values))


@pytest.fixture(autouse=True)
def launcher_closed_after(desktop: Desktop) -> Iterator[None]:
    """Close the launcher after each test, so one failing with it open fails alone."""
    yield
    desktop.launcher_left_closed()


def placed(foot: list[Json]) -> set[tuple[str, int]]:
    """Give each of the foot's words with where it starts, the clock by its place alone.

    A minute may turn between two reads of the foot, so the clock's words are never compared.
    """
    return {
        ("clock" if re.fullmatch(r"\d{1,2}:\d{2}", word["text"]) else word["text"], word["x"])
        for word in foot
    }


def status_facts(foot: list[Json], told: list[str]) -> tuple[int, set[tuple[str, int]]]:
    """Give where the foot's mode mark starts, and its status facts with where each starts.

    The facts are the words between the mark and the first of the keys told: the mark reads
    each mode's own three capitals, and each mode tells its own keys, so neither is a fact.
    """
    keys = {key.split(" ", 1)[0] for key in told}
    ordered = sorted(foot, key=lambda word: word["x"])
    keys_from = min((word["x"] for word in ordered if word["text"] in keys), default=None)
    assert keys_from is not None, (told, foot)
    facts = [word for word in ordered[1:] if word["x"] < keys_from]
    return ordered[0]["x"], {(text, x) for text, x in placed(facts) if "<font" not in text}


def the_modes_look_as_vims(desktop: Desktop) -> None:
    """See the line on the find, then in normal by Escape, then on the find again by A.

    It starts and ends with "batt" typed, on the find, the caret at its end.
    """
    machine = desktop.machine
    # The line opens on the find, Vim's / line, led by its / in the accent,
    # the foot naming it by its mark alone, never by a word, and
    # the caret a bar a quarter of a letter wide.
    shown = desktop.showing()
    assert (shown["mode"], shown["sigil"], shown["sigilHue"]) == ("find", "/", "accent"), shown
    laid = json.loads(desktop.as_account("qs ipc call launcher laidOut"))
    field = laid["field"]
    assert not any(word["text"] in ("insert", "find") for word in laid["foot"]), laid["foot"]
    mark, facts = status_facts(laid["foot"], shown["selected"]["told"])
    # The line holds only the find and how far it narrows: Ctrl+Y's completing is said in
    # the foot, by its key and lowercase word, with every other hint, and no key is on the
    # line.
    keyed = [word["text"] for word in field if word["text"].endswith((" complete", " clear"))]
    assert not keyed, field
    assert "Ctrl+Y complete" in desktop.showing()["selected"]["told"], desktop.showing()
    assert desktop.showing()["caretShown"] is True, desktop.showing()
    # Escape gives normal: the caret one left and not drawn, since the
    # keys are on the rows, no word for the mode with the rest of the foot
    # where it was, normal's own keys named by letter beside Return, the
    # command line's and the key list's always, and no key in the field's
    # place, since - steps out rather than clearing. It's read before the
    # walk's edits, since a wider aside they bring crowds the keys' words out
    # until the step ends; A goes back.
    machine.send_key("esc")
    desktop.launcher_until(lambda shown: (shown["mode"], shown["caret"]) == ("normal", 3))
    shown = desktop.showing()
    assert shown["caretShown"] is False, shown
    # The / goes, its column kept, since normal's keys are on the rows.
    assert shown["sigil"] == "", shown
    laid = json.loads(desktop.as_account("qs ipc call launcher laidOut"))
    # The mode's mark and the clock, first of the facts, hold their places. The rest of the
    # glance may read its least, giving way to normal's keys as draws-still walks it, and the
    # keys are normal's own, drawn with their marks.
    now_mark, now_facts = status_facts(laid["foot"], shown["selected"]["told"])
    assert now_mark == mark, laid["foot"]
    first = min(facts, key=lambda fact: fact[1])
    assert first[0] == "clock", facts
    assert first in now_facts, (facts, laid["foot"])
    # The answer holds more, so Return goes in, and y, the one way left to copy it, comes
    # after it; the key list's stays just before Return.
    told = desktop.showing()["selected"]["told"]
    assert told[:2] == ["\u21b5 open", "y copy"], told
    assert "Tab in" not in told, told
    assert told[-2:] == [": commands", "Space ? keys"], told
    assert not any("\u2192" in text for text in told), told
    assert not any(word["text"].endswith(" clear") for word in laid["field"]), laid["field"]
    machine.send_chars("A")
    desktop.launcher_until(lambda shown: (shown["mode"], shown["caret"]) == ("find", 4))


def walk_the_fields_keys(desktop: Desktop, charge: str) -> None:
    """Press every key the line's find and normal take, from "batt" typed to "battery".

    It starts with "batt" typed and the battery's answer selected, and ends with Ctrl+Y having
    completed it, on the find, as Joel would press them through niri.
    """
    machine = desktop.machine
    the_modes_look_as_vims(desktop)
    # The find's keys are Vim's, through niri as Joel presses them
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
    # Letters type nothing there, and one with no meaning says so in the foot until the next
    # key; q says how to close and closes nothing; the find and the battery stay.
    machine.send_chars("z")
    desktop.launcher_until(lambda shown: shown["saying"] == ["z not here"])
    desktop.shoot("launcher-not-here")
    machine.send_chars("q")
    desktop.launcher_until(lambda shown: shown["saying"] == ["q does nothing", ":q closes"])
    desktop.shoot("launcher-q-does-nothing")
    machine.send_chars("j")
    desktop.launcher_until(lambda shown: shown["saying"] == [])
    machine.send_chars("k")
    desktop.launcher_until(
        lambda shown: (
            shown["open"]
            and shown["typed"] == "batt"
            and shown["kind"] == "answer"
            and (shown["selected"] or {}).get("name", "").startswith(f"{charge}%")
        )
    )
    # i a I A go back to the find at the caret, after it, at the start and
    # at the end, each Escape back putting the caret one left; cc and S
    # clear the line onto the find.
    for into, caret, after in [("i", 3, 2), ("a", 3, 2), ("I", 0, 0), ("A", 4, None)]:
        machine.send_chars(into)
        desktop.launcher_until(
            lambda shown: (shown["mode"], shown["caret"]) == ("find", caret)  # noqa: B023
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
        desktop.launcher_until(lambda shown: (shown["mode"], shown["typed"]) == ("find", ""))
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
        # Each query is typed on the find, and Ctrl+U clears it there, as on
        # Vim's / line.
        desktop.on_find()
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
            machine.send_key("ctrl-u")
    desktop.close_launcher()
    assert listed_twice == [], "a thing was listed twice"
    # Most of those find something, so the rows were looked at.
    assert looked_at > ROWS_LOOKED_AT


def finds_where_the_notices_went(desktop: Desktop) -> None:
    """Find where the notices went, For You's Desktop, by their place's words.

    Said where it is, Tab and Return step in as Super+N does, unread or not, and - climbs back
    out the way it stepped in, through For You (VKCF3Z, 2026-11-09: a rename never loses a row).
    """
    into = ["now", "For You", "Desktop"]
    for typed in ["notices", "notifications"]:
        desktop.answer(typed)
        desktop.launcher_until(lambda shown: "Desktop" in shown["rows"])
        desktop.down_to("Desktop")
        desktop.launcher_until(
            lambda shown: (shown["selected"] or {}).get("side") == "Now \u203a For You"
        )
        if typed == "notices":
            # Normal's Tab steps in; the find's goes to the next row, as Vim's completion does.
            desktop.normal()
            desktop.machine.send_key("tab")
            desktop.launcher_until(lambda shown: shown["held"] == into)
            desktop.close_launcher()
    desktop.machine.send_key("ret")
    desktop.launcher_until(lambda shown: shown["held"] == into)
    desktop.normal()
    desktop.machine.send_key("minus")
    desktop.launcher_until(lambda shown: shown["held"] == ["now", "For You"])
    desktop.close_launcher()


def test_a_collection_or_half_a_word_is_asked_and_the_fields_keys_edit_it(desktop: Desktop) -> None:
    """A collection is found by its name, half a word asks, and the field's keys edit it."""
    machine = desktop.machine
    charge = charged(desktop)
    # A collection is found by its name, and Return goes straight into
    # it.
    # Beside the field, it says what sort of thing each is.
    for typed, collection in [("sound", "Sound"), ("clipboard", "Clipboard")]:
        desktop.answer(typed)
        # The wait ends before the loop moves on, so it reads this pass's name.
        desktop.launcher_until(
            lambda shown: (
                (shown["selected"] or {}).get("name") == collection  # noqa: B023
                and shown["kind"] == "collection"
            )
        )
        desktop.close_launcher()
    finds_where_the_notices_went(desktop)
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
    # Return from the find goes into the answer, which holds the battery's parts, rather
    # than copying it (Joel, 2026-11-06: "i type memory in the launcher, hit enter and it
    # copies and does not go into more details"); y copies it there.
    desktop.step_in()
    desktop.launcher_until(lambda shown: shown["open"] and len(shown["held"]) == 1)
    desktop.close_launcher()
    desktop.answer("battery")
    desktop.normal()
    since = desktop.mark()
    machine.send_chars("yy")
    desktop.reported("yanked 1", lambda rest: rest == "", since)
    assert desktop.as_account("wl-paste --no-newline").startswith(f"{charge}% \u00b7 ")
    desktop.close_launcher()
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
    # A program's name and one fresh Return opens it, with no press more and no wait: the
    # Return is done as it's heard, in far less than the while a doubled Return is waited
    # out, and the window comes.
    windows = 'niri msg --json windows | grep -o \'"app_id":"kitty"\' | wc -l'
    kitties = int(desktop.as_account(windows))
    desktop.answer("kitty")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("is") == "program:kitty")
    machine.send_key("ret")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10))
    returned = desktop.showing()["returned"]
    assert returned["act"] == "ran", returned
    took = returned["done"] - returned["heard"]
    whole = returned["done"] - returned["pressed"]
    print(f"Return opened kitty {whole:.1f} ms after it was pressed, {took:.1f} after heard")
    assert took < FRESH_RETURN_MS, returned
    machine.wait_until_succeeds(
        desktop.in_session(f"test $({windows}) -gt {kitties}"), timeout=dt.timedelta(seconds=60)
    )
    desktop.as_account("niri msg action close-window")


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
    desktop.shoot("launcher-emoji")
    desktop.type_in(" cat")
    desktop.shoot("launcher-emoji-all")
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
    desktop.follow()
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
    desktop.shoot("launcher-category")
    desktop.type_in("add", selecting="Address")
    # In normal, - steps out at once, the word put back.
    desktop.step_out()
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
    desktop.shoot("launcher-commands")
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
    # hasn't got, as a VPN, or kitty's SSH to a Machine with no machine
    # known, isn't found by its new name either, and is passed over; the
    # count of those found proves the walk looked. Only the thing itself
    # counts as found by its name, never Ask, which offers to take any
    # sentence (the net, 2026-11-08: every run since #2362 failed on Host).
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

            def by_its_name(there: Json, name: str = name) -> bool:
                """Say whether the thing itself is found by its name, never Ask."""
                return name in there["rows"] or (there.get("selected") or {}).get("name") == name

            try:
                desktop.launcher_until(by_its_name, timeout=dt.timedelta(seconds=5))
                found = [thing["is"] for thing in desktop.showing()["things"][:3]]
                lost.append(f"{old} -> {name}: {shown['rows'][:5]} {shown['footer']!r}, {found}")
            except AssertionError:
                absent.append(name)
        desktop.close_launcher()
    print(f"walked {len(kept)} old names, {len(absent)} absent here: {absent}")
    assert lost == [], lost
    assert len(kept) - len(absent) >= fewest, absent


def test_wifi_finds_the_home_network_after_wi_fi_itself(desktop: Desktop) -> None:
    """Typing wifi finds the Home Network too, after Wi-Fi, which it names first.

    Joel reaches his home network by typing wifi (QEST-20261111-PN2VZC), since what it is to
    him is the Wi-Fi at home, yet wifi is his word for this machine's own Wi-Fi first, picked
    in a week of his typing. This machine has both, with the eero daemon's stand-in.
    """
    desktop.answer("wifi")
    try:
        desktop.launcher_until(
            lambda shown: "Wi-Fi" in shown["rows"] and "Home Network" in shown["rows"],
            timeout=dt.timedelta(seconds=10),
        )
    except AssertionError:
        listed = f"wifi listed {desktop.showing()['rows']}"
        raise AssertionError(listed) from None
    rows = desktop.showing()["rows"]
    assert rows.index("Wi-Fi") < rows.index("Home Network"), rows
    desktop.close_launcher()


def waits_then_escape_lets_go(desktop: Desktop, key: str, typed: str | None = None) -> None:
    """Type a key that begins a pair, see it wait for its second, and let it go by Escape.

    The key is as the launcher names it, typed as its own text unless told what to type. It
    waits however long, as his which-key holds a begun key, so Escape is what lets it go.
    """
    desktop.machine.send_chars(key if typed is None else typed)
    desktop.launcher_until(lambda shown: shown["begun"] == key)
    desktop.machine.send_key("esc")
    desktop.launcher_until(lambda shown: shown["begun"] == "")


def arrows_move_as_letters(desktop: Desktop) -> None:
    """Press each arrow in normal from the first row: Down and Up are j and k, with a count.

    In normal, in a buffer, the arrows move the cursor (Joel, 2026-11-05 12:1x), as their
    letters do. Right and Left never step in or out, so on the rows they say "not here", as
    every key with no meaning does, until the next key.
    """
    machine = desktop.machine
    for keys, row in [(["down"], 1), (["down"], 2), (["up"], 1), (["up"], 0)]:
        for key in keys:
            machine.send_key(key)
        desktop.launcher_until(lambda shown: shown["current"] == row)  # noqa: B023
    counted = 2
    machine.send_chars(str(counted))
    machine.send_key("down")
    desktop.launcher_until(lambda shown: shown["current"] == counted and shown["counted"] == "")
    machine.send_chars("gg")
    desktop.launcher_until(lambda shown: shown["current"] == 0)
    for arrow in ("right", "left"):
        desktop.machine.send_key(arrow)
        desktop.launcher_until(
            lambda shown: (
                shown["current"] == 0
                and shown["saying"][-1:] != []
                and shown["saying"][-1].endswith("not here")
            )
        )
        desktop.machine.send_chars("k")
        desktop.launcher_until(lambda shown: shown["saying"] == [])


def walk_the_gutter(desktop: Desktop) -> None:
    """See normal's gutter number a long list from its first row, and Space u l switch it.

    The cursor row's number is absolute and the rest count from it, as his Neovim's
    relativenumber does, made a number a row in view however long the list; Space u l numbers
    each row as it is and back; the narrowed find says its count after what's typed; and on
    the find it numbers each row as it is, in the same room, as his numbertoggle does.
    """
    machine = desktop.machine

    def numbered(first: list[str], relative: bool) -> Json:
        desktop.launcher_until(
            lambda shown: (
                shown["gutter"]["shown"]
                and shown["gutter"]["relative"] == relative
                and shown["gutter"]["numbers"][: len(first)] == first
            )
        )
        return desktop.showing()

    shown = numbered(["1", "1", "2", "3"], relative=True)
    gutter = shown["gutter"]
    assert gutter["cursor"] == "1", gutter
    # A number a slot in view, not a row of the list's.
    assert len(gutter["numbers"]) <= gutter["made"] < shown["count"], (gutter, shown["count"])
    made = gutter["made"]
    print(f"the gutter made {made} numbers for a list of {shown['count']} rows")
    machine.send_chars("j")
    assert numbered(["1", "2", "1", "2"], relative=True)["gutter"]["cursor"] == "2"
    machine.send_chars("G")
    desktop.launcher_until(lambda shown: shown["current"] == shown["count"] - 1)
    gutter = desktop.showing()["gutter"]
    assert gutter["cursor"] == str(desktop.showing()["count"]), gutter
    assert gutter["made"] == made, f"the gutter made {gutter['made']} numbers, then {made}"
    machine.send_chars("gg")
    numbered(["1", "1"], relative=True)
    machine.send_chars(" ul")
    numbered(["1", "2", "3"], relative=False)
    desktop.shoot("launcher-gutter-absolute")
    machine.send_chars(" ul")
    numbered(["1", "1", "2"], relative=True)
    desktop.shoot("launcher-gutter")
    # The find narrows the list, which normal says after it, in the field: how many it found,
    # the gutter and the status line saying where the cursor is.
    count = desktop.showing()["count"]
    field = json.loads(desktop.as_account("qs ipc call launcher laidOut"))["field"]
    assert any(word["text"] == f"\u00b7 {count} found" for word in field), field
    # On the find it numbers each row as it is, in the same room, so no row moves, and normal
    # counts from the cursor again (Joel, 2026-11-05: "real numbers in find mode and
    # relative numbers in command mode").
    room = desktop.showing()["gutter"]["room"]
    assert room > 0, desktop.showing()["gutter"]
    machine.send_chars("A")
    desktop.launcher_until(lambda shown: shown["mode"] == "find")
    assert numbered(["1", "2", "3"], relative=False)["gutter"]["room"] == room
    desktop.normal()
    numbered(["1", "1"], relative=True)


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

    for keys, row in [("j", 1), ("j", 2), ("k", 1), ("k", 0), ("G", last), ("g g", 0)]:
        press(keys)
        at(row)
    arrows_move_as_letters(desktop)
    walk_the_gutter(desktop)
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
    # Escape with a count begun only lets it go, as Vim's does; it never closes.
    machine.send_chars("5")
    desktop.launcher_until(lambda shown: shown["counted"] == "5")
    machine.send_key("esc")
    desktop.launcher_until(lambda shown: shown["counted"] == "")
    assert desktop.showing()["open"], "an Escape cancelling a count closed the launcher"
    # A g waits for its second, and Escape lets it go, so a j after moves.
    waits_then_escape_lets_go(desktop, "g")
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

    Return steps into a row and - out of it, as his oil's do, while h, l and Backspace, which
    move along a line a row hasn't, go nowhere and say Vim's way instead, and - at the top says
    it has nowhere to go; :12 goes to the 12th row, on the command line in the find line's
    place, its menu saying where it goes, until its Return; and ZZ and :q each close the
    launcher.
    """
    machine = desktop.machine
    desktop.answer("emoji")
    desktop.launcher_until(lambda shown: shown["rows"] == ["Emoji"])
    desktop.normal()
    said = {"l": ["l not here", "\u21b5 open"], "h": ["h not here", "- up"]}
    # Each saying waited on differs from the one before it, so none is read stale. A step in
    # starts its own find, and - comes back to the one it left, as oil's does.
    for held, typed, chars, key, saying in [
        ([], "emoji", "l", "", said["l"]),
        ([], "emoji", "h", "", said["h"]),
        ([], "emoji", "-", "", ["- not here"]),
        (["emoji"], "", "", "ret", []),
        (["emoji"], "", "h", "", said["h"]),
        (["emoji"], "", "l", "", said["l"]),
        (["emoji"], "", "", "backspace", said["h"]),
        ([], "emoji", "-", "", []),
    ]:
        machine.send_chars(chars)
        if key:
            machine.send_key(key)
        wanted = (held, typed, "normal", saying)

        def stands(shown: Json, wanted: tuple[object, ...] = wanted) -> bool:
            """Say whether the launcher holds, finds, waits and says what this press wants."""
            return (shown["held"], shown["typed"], shown["mode"], shown["saying"]) == wanted

        desktop.launcher_until(stands)
    # Ctrl+] goes into an emoji's acts, which hold nothing more, so there
    # it says so and stays.
    machine.send_key("ret")
    desktop.launcher_until(lambda shown: shown["held"] == ["emoji"])
    desktop.follow()
    desktop.launcher_until(lambda shown: len(shown["held"]) == 2)  # noqa: PLR2004
    machine.send_key("ctrl-bracket_right")
    desktop.launcher_until(
        lambda shown: shown["saying"] == ["Ctrl+] not here"] and len(shown["held"]) == 2  # noqa: PLR2004
    )
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
    for keys in ["ZZ", ":q\n"]:
        machine.send_chars(keys)
        desktop.launcher_until(lambda shown: not shown["open"])
        desktop.answer("s")
        desktop.normal()
    desktop.close_launcher()


def lettered(rows: list[str], at: int, letter: str, way: int) -> int:
    """Give the next row from `at`, going `way`, whose name begins with `letter`, or -1.

    A small letter finds either case, as his smartcase does, and nothing goes round an end.
    """
    at += way
    while 0 <= at < len(rows):
        if rows[at][:1].lower() == letter:
            return at
        at += way
    return -1


def walk_the_rows_finds(desktop: Desktop) -> None:
    """Press / ? n N f F ; , on a search's rows in normal, through niri, as Vim's go.

    The find line is the list's one way to find (VKCF3Z's vi-paradigm spec, The Find Line):
    / and ? go to it, emptied, going down and up, and n and N go on through the rows it found,
    round past an end as Vim's wrapscan says. f and F go to the next row and the one before
    whose name begins with the letter typed after, ; and , again (Joel, 2026-11-05).
    """
    machine = desktop.machine
    for fresh in "/?":
        desktop.answer("s")
        desktop.launcher_until(lambda shown: shown["count"] >= MANY_ROWS)
        desktop.normal()
        machine.send_chars(fresh)
        desktop.launcher_until(lambda shown: (shown["mode"], shown["typed"]) == ("find", ""))
        machine.send_chars("s")
        desktop.launcher_until(lambda shown: shown["count"] >= MANY_ROWS and shown["current"] == 0)
        desktop.normal()
        # n goes the find's way: down after /, and up after ?, round from the first to the last.
        machine.send_chars("n")
        last = desktop.showing()["count"] - 1
        went = (1, []) if fresh == "/" else (last, ["search hit TOP, continuing at BOTTOM"])
        desktop.launcher_until(lambda shown: (shown["current"], shown["saying"]) == went)  # noqa: B023
        desktop.close_launcher()
    desktop.answer("s")
    desktop.launcher_until(lambda shown: shown["count"] >= MANY_ROWS and shown["current"] == 0)
    desktop.normal()
    last = desktop.showing()["count"] - 1
    for keys, went in [
        ("2n", (2, [])),
        ("N", (1, [])),
        ("G", (last, [])),
        ("n", (0, ["search hit BOTTOM, continuing at TOP"])),
    ]:
        machine.send_chars(keys)
        desktop.launcher_until(lambda shown: (shown["current"], shown["saying"]) == went)  # noqa: B023
    rows: list[str] = desktop.showing()["rows"]
    letter = rows[1][:1].lower()
    here = lettered(rows, 0, letter, 1)
    again = lettered(rows, here, letter, 1)
    machine.send_chars(f"f{letter}")
    desktop.launcher_until(lambda shown: shown["current"] == here)
    machine.send_chars(";")
    if again < 0:
        said = [f"no row below begins with {letter}"]
        desktop.launcher_until(lambda shown: (shown["current"], shown["saying"]) == (here, said))
    else:
        desktop.launcher_until(lambda shown: shown["current"] == again)
        machine.send_chars(",")
        desktop.launcher_until(lambda shown: shown["current"] == here)
    # F goes up to one, or says there's none, the selection staying put.
    back = lettered(rows, here, letter, -1)
    went = (back, []) if back >= 0 else (here, [f"no row above begins with {letter}"])
    machine.send_chars(f"F{letter}")
    desktop.launcher_until(lambda shown: (shown["current"], shown["saying"]) == went)
    desktop.close_launcher()
    # With nothing typed n has nothing to go on with, as Vim's E35 says.
    desktop.answer("")
    desktop.normal()
    machine.send_chars("n")
    desktop.launcher_until(lambda shown: shown["saying"] == ["E35: No previous regular expression"])
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


# How each hint's mark in the foot is pressed through niri, where it isn't a letter.
HINT_PRESSES = {
    "\u21b5": (["ret"], ""),
    "Space ?": ([], " ?"),
    "Ctrl+]": (["ctrl-bracket_right"], ""),
}


def standing(shown: dict[str, Any]) -> tuple[object, ...]:
    """Say what a key in normal changes: where it is, what it lists, what it says and waits on."""
    return (
        shown["open"],
        shown["operator"],
        shown["mode"],
        shown["held"],
        (shown["command"] or {}).get("line"),
        shown.get("listing"),
        shown["typed"],
        shown["current"],
    )


def every_hint_changes(desktop: Desktop, depth: int) -> None:
    """Press each hint normal's foot draws on Emoji's rows, at this depth, each afresh.

    The foot's hints come from the table that handles the keys, so a hint that changes nothing
    is a key the foot names wrongly. Each is pressed on a launcher opened anew, so what one
    did can't stand in for the next.
    """

    def opened() -> list[str]:
        desktop.answer("emoji")
        desktop.launcher_until(lambda shown: shown["rows"] == ["Emoji"])
        desktop.normal()
        for _ in range(depth):
            desktop.machine.send_key("ret")
        desktop.launcher_until(lambda shown: len(shown["held"]) == depth)
        return cast("list[str]", desktop.showing()["selected"]["told"])

    told = opened()
    assert told[0].startswith("\u21b5"), told
    # Tab in comes first after Return, on a row Return opens that holds parts, as an emoji.
    assert ("Tab in" in told[1:2]) == bool(depth), told
    assert told[-1] == "Space ? keys", told
    for at, hint in enumerate(told):
        mark = hint.rsplit(" ", 1)[0]
        keys, chars = HINT_PRESSES.get(mark, ([], mark))
        if at > 0:
            opened()
        was = standing(desktop.showing())
        for key in keys:
            desktop.machine.send_key(key)
        desktop.machine.send_chars(chars)
        desktop.launcher_until(lambda shown: standing(shown) != was)  # noqa: B023
        desktop.close_launcher()


def walk_the_rows_keys(desktop: Desktop) -> None:
    """Press normal's places, Space ?, F1, Space w and Escape on a search's rows, through niri.

    Space Tab goes to the place used last and back, as his goes to the file used last; Ctrl+O
    and Tab, which is Ctrl+I, go back and forward through the places stepped into; Space ? and
    F1 list every key in the drawer until Escape or again; Space alone waits for a second key
    until Escape lets it go; Space w closes the launcher, and Escape at rest a step in does
    nothing, :q closing it. Every hint the foot draws, at home and a step in, is first pressed
    and seen to do something.
    """
    machine = desktop.machine
    every_hint_changes(desktop, 0)
    every_hint_changes(desktop, 1)
    desktop.answer("emoji")
    desktop.launcher_until(lambda shown: shown["rows"] == ["Emoji"])
    desktop.normal()
    machine.send_key("ret")
    desktop.launcher_until(lambda shown: shown["held"] == ["emoji"])
    machine.send_chars("-")
    desktop.launcher_until(lambda shown: shown["held"] == [] and shown["typed"] == "emoji")
    machine.send_chars(" ")
    machine.send_key("tab")
    desktop.launcher_until(lambda shown: shown["held"] == ["emoji"])
    machine.send_chars(" ")
    machine.send_key("tab")
    desktop.launcher_until(lambda shown: shown["held"] == [] and shown["typed"] == "emoji")
    machine.send_key("ctrl-o")
    desktop.launcher_until(lambda shown: shown["held"] == ["emoji"])
    machine.send_key("ctrl-i")
    desktop.launcher_until(lambda shown: shown["held"] == [] and shown["typed"] == "emoji")

    # What the drawer draws, since its facts keep the last plate as it goes back in.
    def listed(listing: bool) -> None:
        desktop.launcher_until(
            lambda shown: (
                shown["open"]
                and shown.get("listing") == listing
                and ("Space ?  F1" in shown["drawer"]["drawnFacts"]) == listing
                and ("The find: the / line" in shown["drawer"]["drawnFacts"]) == listing
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
    waits_then_escape_lets_go(desktop, "Space", " ")
    assert desktop.showing()["open"], "a Space alone closed the launcher"
    machine.send_chars(" w")
    desktop.launcher_until(lambda shown: not shown["open"])
    follows_from_the_find(desktop)
    escape_at_rest_stays(desktop)
    # A colon as the find's first letter opens the command line, where :q closes.
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    machine.send_chars(":q")
    desktop.launcher_until(lambda shown: (shown["command"] or {}).get("line") == "q")
    machine.send_key("ret")
    desktop.launcher_until(lambda shown: not shown["open"])


def follows_from_the_find(desktop: Desktop) -> None:
    """See the find's foot name Tab in on a row holding parts, and Tab go in from there.

    A program's commands and windows are its parts, and the find is where Joel types its name,
    so the find goes in by the key normal does and says so (Joel, 2026-11-05: "in programs i
    don't understand how to go to sub commands on a program"; 2026-11-07: "Tab opens its
    page"), its first row the program's own Return.
    """
    desktop.answer("kitty")
    desktop.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("is") == "program:kitty" and shown["mode"] == "find"
        )
    )
    told = desktop.showing()["selected"]["told"]
    assert "Tab in" in told, told
    assert "Ctrl+] in" not in told, told
    desktop.machine.send_key("tab")
    # A step in, the find starts empty, so kitty's commands aren't narrowed to "kitty", and
    # the cursor is on the first, Go To or Start, as Return on kitty would have done.
    desktop.launcher_until(
        lambda shown: (
            len(shown["held"]) == 1
            and shown["mode"] == "find"
            and shown["typed"] == ""
            and shown["current"] == 0
        )
    )
    desktop.close_launcher()


def escape_at_rest_stays(desktop: Desktop) -> None:
    """Press Escape at rest a step in, which does nothing, and close by :q.

    As Vim's in normal, it neither closes nor steps out, and says nothing (Joel, 2026-11-05:
    "fuck escape quitting").
    """
    machine = desktop.machine
    desktop.answer("emoji")
    desktop.launcher_until(lambda shown: shown["rows"] == ["Emoji"])
    desktop.follow()
    # A held place's rows are numbered as the search's are (87ED6K, for the picker's folders).
    desktop.launcher_until(
        lambda shown: (
            shown["held"] == ["emoji"]
            and shown["gutter"]["shown"]
            and shown["gutter"]["numbers"][:3] == ["1", "1", "2"]
        )
    )
    for _ in range(2):
        machine.send_key("esc")
    # A key after the two shows they were heard, and did nothing.
    machine.send_chars("j")
    desktop.launcher_until(lambda shown: shown["current"] == 1)
    shown = desktop.showing()
    assert (shown["open"], shown["held"], shown["mode"], shown["saying"]) == (
        True,
        ["emoji"],
        "normal",
        [],
    ), shown
    machine.send_chars(":q\n")
    desktop.launcher_until(lambda shown: not shown["open"])


def walk_the_rows_follows(desktop: Desktop) -> None:
    """Press Ctrl+] on a row found from home, and Ctrl+T, Ctrl+O, Ctrl+I and Tab, through niri.

    Lock holds no parts: no verbs, and its Return closes the launcher, where Keep Awake's keeps
    it and so holds its own act. So Ctrl+] goes to the place it lives, the cursor on it there,
    as Vim follows a tag, and Ctrl+T comes back, as Vim's pops its tag stack, saying Vim's E73
    with nothing to pop. The place it went to is on the jump list too. In the row's own place
    Ctrl+] means nothing. Return is never pressed, so the screen never locks.
    """
    machine = desktop.machine
    desktop.answer("lock")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("is") == "session:Lock")
    desktop.normal()
    # A row whose foot says Tab in holds parts, and Ctrl+] would go into them instead.
    desktop.launcher_until(
        lambda shown: (
            ": commands" in shown["selected"]["told"] and "Tab in" not in shown["selected"]["told"]
        )
    )
    # Where it lives is read from the first follow and held to after, so the walk says the
    # place is the same each time without naming the catalogue's crumbs.
    lives: list[object] = []

    def followed() -> None:
        def there(shown: Json) -> bool:
            """Say whether the launcher holds Lock's own place, onto it, nothing typed."""
            onto = (shown["selected"] or {}).get("is") == "session:Lock"
            held = shown["held"]
            if not (onto and shown["typed"] == "" and held != []):
                return False
            if not lives:
                lives.append(held)
            return bool(held == lives[0])

        desktop.launcher_until(there)

    def came_back() -> None:
        desktop.launcher_until(
            lambda shown: (
                (shown["held"], shown["typed"], (shown["selected"] or {}).get("is"))
                == ([], "lock", "session:Lock")
            )
        )

    machine.send_key("ctrl-bracket_right")
    followed()
    # A follow is one jump, as Vim's tag is, however many steps down its place lies.
    jumps = desktop.showing()["jumps"]
    assert bool(jumps == {"back": 1, "ahead": 0}), f"a follow made jumps {jumps}"
    for key, then in [
        ("ctrl-t", came_back),
        ("ctrl-bracket_right", followed),
        ("ctrl-o", came_back),
        ("ctrl-i", followed),
    ]:
        machine.send_key(key)
        then()
    # Lock holds nothing, so Tab, which goes into what a row holds, says it isn't here, as l
    # does, and moves nothing.
    machine.send_key("tab")
    desktop.launcher_until(lambda shown: shown["saying"] == ["Tab not here"])
    followed()
    machine.send_key("ctrl-bracket_right")
    desktop.launcher_until(lambda shown: shown["saying"] == ["Ctrl+] not here"])
    machine.send_key("ctrl-t")
    came_back()
    machine.send_key("ctrl-t")
    # Vim's errors read where the command line is, as E492 does.
    desktop.launcher_until(
        lambda shown: shown["command"] is None and shown["commandSaid"] == "E73: Tag stack empty"
    )
    desktop.close_launcher()
    # Each Ctrl+] with no parts to go into looks for the row's own place from home on its key,
    # nothing walked before: the trace holds each look's ms and the rows it listed and made,
    # said here for the reviewer to read. Lock lives deepest of the kinds Ctrl+] looks for,
    # found from home two places down, and each look found it within its budget.
    spans: list[Json] = json.loads(desktop.as_account("qs ipc call launcher traced"))
    opening = [span for span in spans if span["name"] == "open"][-1]
    looks = [
        span["about"]
        for span in spans
        if span["trace"] == opening["trace"]
        and span["name"] == "follow"
        and "took" in span["about"]
    ]
    said = [(look["took"], look["listed"], look["made"]) for look in looks]
    print(f"Ctrl+] looked for its place in (ms, rows listed, rows made): {said}")
    assert len(looks) == FOLLOWS, looks
    assert all(isinstance(look["took"], int) for look in looks), looks
    assert all(0 < look["listed"] <= LIVES_MOST for look in looks), looks
    assert not any(look["spent"] for look in looks), f"a look gave up: {looks}"


def walk_a_found_program_follows(desktop: Desktop) -> None:
    """Press Ctrl+] on kitty found from home, and Ctrl+T, through niri.

    A row that holds more goes into its parts, as the foot's Tab in says, before where it
    lives, and Ctrl+T comes back from there to what was typed, as from any place followed.
    """
    machine = desktop.machine
    desktop.answer("kitty")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("is") == "program:kitty")
    desktop.normal()
    machine.send_key("ctrl-bracket_right")
    desktop.launcher_until(lambda shown: shown["held"][-1:] == ["kitty"])
    machine.send_key("ctrl-t")
    desktop.launcher_until(
        lambda shown: (
            (shown["held"], shown["typed"], (shown["selected"] or {}).get("is"))
            == ([], "kitty", "program:kitty")
        )
    )
    desktop.close_launcher()


def test_a_search_lists_each_thing_once_and_normal_moves_through_it(desktop: Desktop) -> None:
    """A search lists each thing once, and normal's keys move and act on it as Vim's do."""
    walk_the_rows_moves(desktop)
    walk_the_rows_acts(desktop)
    walk_the_rows_finds(desktop)
    walk_the_rows_washed(desktop)
    walk_the_rows_keys(desktop)
    walk_the_rows_follows(desktop)
    walk_a_found_program_follows(desktop)
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


def test_a_step_finds_its_rows_by_their_hidden_words_and_its_parts_move_in_place(
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
    desktop.follow()
    desktop.launcher_until(lambda shown: "VPN" in shown["rows"])
    desktop.type_in("tailscale0", selecting="VPN")
    assert "tailscale0" not in desktop.showing()["rows"]
    # Return steps into it.
    desktop.follow()
    desktop.launcher_until(lambda shown: "Address" in shown["rows"])
    # Stepping back out, by -, lands on the row it came from, with its find.
    desktop.step_out()
    desktop.launcher_until(
        lambda shown: (
            len(shown["held"]) == 1
            and shown["typed"] == "tailscale0"
            and (shown["selected"] or {}).get("name") == "VPN"
        )
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
        desktop.next_row()
    desktop.launcher_until(
        lambda shown: (shown["selected"] or {}).get("is") == "answer:machine:cpu"
    )
    desktop.follow()
    desktop.launcher_until(
        lambda shown: shown["path"] == ["Machine", "Processor"] and "Cores" in shown["rows"]
    )
    desktop.down_to("Cores")
    desktop.follow()
    desktop.launcher_until(lambda shown: shown["path"] == ["Machine", "Processor", "Cores"])
    cores = desktop.showing()["rows"]
    if len(cores) > 1:
        desktop.next_row()
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
    desktop.step_out()
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


def shoot_the_calendars(desktop: Desktop) -> None:
    """Shoot each calendar's plate in its own script, its names whole and on one line.

    Its title is in the script's cut beside the serif and its figures in the one beside the mono.
    Seen from nowhere, the Hebrew and Islamic say their day begins at sunset; seen from
    Greenwich, the Hebrew says at what time, shot again.
    """
    for calendar in ["Chinese", "Japanese", "Hebrew", "Islamic"]:
        shoot_a_calendar(desktop, calendar, f"launcher-date-{calendar.lower()}")
    said = {"v": 2, "latitude": 51.4769, "longitude": -0.0005, "accuracy": 3000, "when": 1,
            "place": None, "zone": None}  # fmt: skip
    desktop.found_at(said)
    try:
        shoot_a_calendar(desktop, "Hebrew", "launcher-date-hebrew-sunset", timed=True)
    finally:
        desktop.found_at(None)


def shoot_a_calendar(desktop: Desktop, calendar: str, picture: str, *, timed: bool = False) -> None:
    """Shoot one calendar's plate once it's drawn, its names whole and on one line.

    Timed, it's shot once the plate says at what time its day begins.
    """

    def drawn(shown: Json) -> bool:
        drawer = shown["drawer"]
        values = [str(value) for value in drawer.get("values") or []]
        return (
            drawer["out"] is True
            and (shown["selected"] or {}).get("name") == calendar
            and len(drawer["facts"]) > 1
            and (not timed or any(re.fullmatch(r"\d{2}:\d{2}", value) for value in values))
        )

    desktop.answer("date")
    desktop.follow()
    desktop.launcher_until(lambda shown: "Calendars" in shown["rows"])
    desktop.down_to("Calendars")
    desktop.follow()
    desktop.launcher_until(lambda shown: calendar in shown["rows"])
    desktop.down_to(calendar)
    desktop.launcher_until(drawn)
    drawer = desktop.showing()["drawer"]
    assert drawer["labelsWhole"], f"a {calendar} fact's name is broken: {drawer}"
    assert drawer["besideOnOneLine"], f"a {calendar} value wraps beside its name: {drawer}"
    assert drawer["digitsStandAsFigures"], f"{calendar} digits stand smaller: {drawer}"
    desktop.shoot(picture)
    desktop.close_launcher()


def test_the_answers_are_shot_for_the_gallery(desktop: Desktop) -> None:
    """The launcher's answers are shot for the gallery, and only there.

    What they answer is tests/launcher-on-niri-test.py's to prove, in seconds on a nested niri; a
    person still looks at how each is drawn on the laptop's screen, so the gallery keeps them.
    """
    if not desktop.values["shots"]:
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
        desktop.shoot(picture)
        desktop.close_launcher()
    # Inside the time here and the date, whose parts are the clock's and the
    # calendar's, each a step in.
    for typed, first, picture in [
        ("time", "Zone", "launcher-time-inside"),
        ("date", "ISO Date", "launcher-date-inside"),
    ]:
        desktop.answer(typed)
        desktop.follow()
        # Each wait ends before the loop moves on, so it reads this pass's part.
        desktop.launcher_until(lambda shown: first in shown["rows"])  # noqa: B023
        desktop.shoot(picture)
        desktop.close_launcher()
    shoot_the_calendars(desktop)
    # How chrony keeps the clock, once it keeps it by the VM's own time server: inside Clock,
    # and inside Servers with the chosen one's plate out beside it.
    desktop.machine.wait_until_succeeds(
        "chronyc -c -N sources | grep -q '^^,[*],time.harbour.test,'",
        timeout=dt.timedelta(seconds=90),
    )
    for holder, first, picture in [
        ("Clock", "Offset", "launcher-time-clock"),
        ("Servers", "time.harbour.test", "launcher-time-servers"),
    ]:
        # Ctrl+] goes into what keeps the time, as Return does.
        desktop.answer("time")
        desktop.follow()
        desktop.launcher_until(lambda shown: holder in shown["rows"])  # noqa: B023
        desktop.down_to(holder)
        desktop.follow()
        desktop.launcher_until(lambda shown: first in shown["rows"])  # noqa: B023
        desktop.down_to(first)
        if holder == "Servers":
            desktop.launcher_until(
                lambda shown: (
                    shown["drawer"]["out"] is True
                    and shown["drawer"]["title"] == "time.harbour.test"
                )
            )
            # Each fact is its name and its value on one line, or its name with its value
            # under it: no name broken, no value wrapping beside one, no word broken.
            desktop.launcher_until(
                lambda shown: any(v is not None for v in shown["drawer"]["values"])
            )
            drawer = desktop.showing()["drawer"]
            assert drawer["labelsWhole"], f"a fact's name is broken: {drawer}"
            assert drawer["besideOnOneLine"], f"a value wraps beside its name: {drawer}"
            assert drawer["valuesWhole"], f"a fact's value breaks a word: {drawer}"
        desktop.shoot(picture)
        desktop.close_launcher()
    # The time here counts each second while it's shown, and a tick never moves the
    # selection or changes the card's size: UTC stays selected, among as many rows.
    desktop.answer("time")
    desktop.follow()
    desktop.launcher_until(lambda shown: "UTC" in shown["rows"])
    desktop.down_to("UTC")
    before = desktop.showing()
    was = before["selected"]["side"]
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("side") != was)
    after = desktop.showing()
    assert (after["selected"] or {}).get("name") == "UTC", f"a tick moved the selection: {after}"
    assert after["rows"] == before["rows"], f"a tick changed the rows: {before} then {after}"
    desktop.close_launcher()


def test_spotify_signed_out_offers_sign_in_and_says_so(desktop: Desktop) -> None:
    """Spotify, never signed in here, offers Sign In inside it and says signed out on its row.

    Stepping in runs the account's own spotify-recent, which finds no sign-in and says so by
    its exit status: the provider, its command and its words wired together, as only a machine
    with Spotify's client shows. Its many cases are tests/spotify-program.qml's.
    """
    desktop.answer("spotify")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Spotify")
    desktop.into_program()
    # Spotify is read as it's stepped into, so its acts come once it answers, and the launcher
    # keeps what it showed in place, a row come later at the end (Steady.js).
    desktop.launcher_until(lambda shown: "Sign In" in shown["rows"])
    desktop.shoot("launcher-spotify-signed-out")
    desktop.step_out()
    desktop.launcher_until(
        lambda shown: (shown["selected"] or {}).get("note") == "signed out",
    )
    desktop.close_launcher()


def test_a_terminal_is_shown_a_place_as_the_running_shell_answers(desktop: Desktop) -> None:
    """`launcher` in a terminal says a place as the running shell answers, and a miss on stderr.

    It does nothing that ends something unasked: with no terminal, it's refused.

    What the shell answers is tests/launcher-on-niri-test.py's, and how it's laid out
    in-a-terminal's own tests'; here the program the account is given reaches the shell the
    session runs, WARP's place and drawer among what it says, read from the stand-in
    warp-cli. The gallery keeps it in kitty at Joel's scales.
    """
    machine = desktop.machine
    reads_cold_for_a_terminal(desktop)
    shown = json.loads(desktop.as_account("launcher show machine --json"))
    assert shown["in"] == ["Machine"], shown
    assert "Disk" in [row["name"] for row in shown["rows"]], shown
    # Only --drawers asks the shell to build each row's drawer, as each screen has.
    screens = json.loads(desktop.as_account("launcher show --json desktop screens"))
    assert screens["rows"], screens
    assert not any("drawer" in row for row in screens["rows"]), screens
    drawn = json.loads(desktop.as_account("launcher show --drawers --json desktop screens"))
    held = {row["name"]: "drawer" in row for row in drawn["rows"]}
    assert any(held.values()), f"--drawers gave no row of Screens its drawer: {held}"
    status, said = machine.execute(desktop.in_session("launcher show mahcine 2>&1 >/dev/null"))
    assert status == 1, (status, said)
    assert "did you mean Machine?" in said, said
    # An act that ends something, with no terminal to ask on, is refused unasked, and says so
    # by its own status.
    not_sure = 3
    status, said = machine.execute(
        desktop.in_session("launcher do machine power reboot </dev/null 2>&1 >/dev/null")
    )
    assert status == not_sure, (status, said)
    assert "--yes does it" in said, said
    # One that ends nothing needs no terminal: Machine opens in the launcher, closed before.
    assert not desktop.showing()["open"], "the launcher was open before launcher do"
    status, said = machine.execute(desktop.in_session("launcher do machine </dev/null 2>&1"))
    assert status == 0, (status, said)
    desktop.launcher_until(lambda shown: shown["open"])
    desktop.close_launcher()
    contract = desktop.values["warp_contract"]
    desktop.as_account(
        "mkdir -p ~/warp-cli && for asked in status tunnel-stats settings dns-stats; do"
        f" install -m 644 {contract}/$asked.json ~/warp-cli/$asked.json; done"
    )
    try:
        a_terminal_is_shown_warp(desktop)
        if not desktop.values["shots"]:
            return
        for scale in ("1.5", "2"):
            desktop.as_account(f"niri msg output winit scale {scale}")
            for asked, picture in [
                ("launcher show", "terminal-home"),
                # Screens, which niri answers at once, where Disk waits on the
                # machine's daemon in the VM.
                ("launcher show desktop screens", "terminal-screens"),
                ("launcher show mahcine", "terminal-miss"),
                ("launcher show desktop screens --drawers", "terminal-drawers"),
                ("launcher do machine power reboot", "terminal-asks"),
                ("launcher show network work-network", "terminal-warp"),
                (
                    "launcher show network work-network round-trip",
                    "terminal-warp-round-trip",
                ),
            ]:
                window = desktop.opened(
                    picture, f"sh -c {shlex.quote(f'echo {asked}; {asked}; exec sleep infinity')}"
                )
                desktop.shoot(f"{picture}-at-{scale.replace('.', '-')}")
                desktop.close(window)
        desktop.as_account("niri msg output winit scale 1")
    finally:
        desktop.as_account("rm -rf ~/warp-cli")


def a_terminal_is_shown_warp(desktop: Desktop) -> None:
    """Hold `launcher show network work-network` to WARP's state and Health, as text and JSON.

    A terminal's ask is answered at once, so the first says WARP is still asking, and the next
    says what warp printed for it (Warp.qml); once two reads are in, Round Trip's drawer says
    whose figures they are, as VKCF3Z ruled.
    """
    machine = desktop.machine
    asking = dt.timedelta(seconds=30)
    machine.wait_until_succeeds(
        desktop.in_session("launcher show network work-network --json | grep -q connected"),
        timeout=asking,
    )
    warp = json.loads(desktop.as_account("launcher show network work-network --json"))
    assert warp["in"] == ["Network", "Work Network"], warp
    assert warp["row"]["side"].startswith("connected"), warp["row"]
    health = next(row for row in warp["rows"] if row["name"] == "Health")
    assert health["side"] == "healthy", health
    said = desktop.as_account("launcher show network work-network")
    assert "connected" in said, said
    assert re.search(r"Health\s+healthy", said), said
    # Each word the program is given is a step, so a name of two is one with a hyphen.
    machine.wait_until_succeeds(
        desktop.in_session(
            "launcher show network work-network round-trip --json | grep -q eyebrow"
        ),
        timeout=asking,
    )
    trip = json.loads(desktop.as_account("launcher show network work-network round-trip --json"))
    assert trip["row"]["drawer"]["eyebrow"] == "via WARP \u00b7 tunnel", trip["row"]
    said = desktop.as_account("launcher show network work-network round-trip")
    assert "via WARP \u00b7 tunnel" in said, said


def reads_cold_for_a_terminal(desktop: Desktop) -> None:
    """With the launcher closed, a terminal is shown what's read only once it's asked for.

    Wi-Fi looks for networks, Firmware asks fwupd, and a keyboard's, a camera's and a
    screen's drawers read their files, each only while something sees them: here a
    terminal's show does.
    Each is asked once, so it's said within the terminal's own wait or not at all, and from
    nothing read: the shell starts again, with the backlight's daemon stopped, whose word
    of each step would have the screen's light read anyway, and the cards forget what
    earlier looks heard until wifid says none nearby.
    """
    desktop.launcher_left_closed()
    machine = desktop.machine
    desktop.as_account("systemctl --user stop backlight")
    try:
        desktop.as_account("systemctl --user restart quickshell")
        machine.wait_until_succeeds(
            desktop.in_session("qs ipc show | grep -q launcher"), timeout=STARTING
        )
        each_read_cold(desktop)
    finally:
        desktop.as_account("systemctl --user start backlight")


def each_read_cold(desktop: Desktop) -> None:
    """Ask each place read only once it's asked for, once, with the shell just started."""
    machine = desktop.machine
    # Flushed until wifid says nothing is nearby, since a look still going when the cards
    # forget can bring Office back; its nearby alone, since a network saved or joined is named
    # elsewhere in what it says, and only once wifid has answered, so a wifid that won't isn't
    # taken for one hearing none.
    wifi = "varlinkctl --json=short call /run/wifid/io.jitsusama.Wifi io.jitsusama.Wifi"
    machine.wait_until_succeeds(
        "for card in $(busctl tree --list fi.w1.wpa_supplicant1"
        " | grep -E '/Interfaces/[0-9]+$'); do busctl call fi.w1.wpa_supplicant1"
        ' "$card" fi.w1.wpa_supplicant1.Interface FlushBSS u 0; done'
        f" && said=$({wifi}.Describe '{{}}')"
        ' && printf %s "$said" | grep -q \'"nearby":\''
        ' && ! printf %s "$said" | grep -qE \'"nearby":\\[[^]]\'',
        timeout=dt.timedelta(seconds=30),
    )

    def rows(*steps: str) -> dict[str, Json]:
        words = " ".join(shlex.quote(step) for step in steps)
        shown = json.loads(desktop.as_account(f"launcher show --json --drawers {words}"))
        return {row["name"]: row for row in shown["rows"]}

    networks = rows("network", "wi-fi")
    if "Office" not in networks:
        # What wifid said of the look, to say why.
        print(machine.execute("journalctl -b --unit=wifid --lines=15 --no-pager")[1])
        print(machine.execute(f"{wifi}.Describe '{{}}'")[1])
    assert "Office" in networks, networks
    firmware = rows("machine", "firmware")
    assert "System Firmware" in firmware, firmware
    cameras = rows("machine", "cameras")
    assert cameras["Built-In Camera"].get("drawer"), cameras
    keyboards = rows("keyboard", "keyboards")
    assert keyboards["The Laptop's Keyboard"].get("drawer"), keyboards
    screens = rows("desktop", "screens")
    lit = screens["Smithay Winit"].get("drawer") or {}
    assert "Brightness" in {fact.get("label") for fact in lit.get("facts", [])}, lit


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
