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
            machine.send_key("esc")
            machine.wait_until_fails(
                desktop.in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10)
            )
            desktop.as_account("qs ipc call launcher toggle")
            desktop.launcher_ready()
        else:
            machine.send_key("left")
    machine.send_key("esc")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10))
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
        machine.send_key("esc")
    # Half a word asks its question too: "batt" is the battery, and
    # Tab finishes the word, the field saying so at its right edge.
    # A copy kept from before saying "battery", a later word, never takes
    # the answer's place, as the battery's own copied answer did when it
    # said "on battery".
    desktop.as_account("echo -n 'kept from the battery' | wl-copy > /dev/null 2>&1")
    desktop.answer("kept")
    desktop.launcher_until(
        lambda shown: any(thing["is"] == "copy:kept from the battery" for thing in shown["things"])
    )
    machine.send_key("esc")
    desktop.answer("batt")
    desktop.launcher_until(
        lambda shown: (
            shown["kind"] == "answer"
            and (shown["selected"] or {}).get("name", "").startswith(f"{charge}%")
        )
    )
    field = json.loads(desktop.as_account("qs ipc call launcher laidOut"))["field"]
    tab = field_key(field, "\u21e5", "complete")
    # The left arrow takes what's typed away only where the field says
    # so, after Tab's place, and the right arrow right after puts it
    # back, what it took standing in grey, the field saying so in the
    # left arrow's place meanwhile. With Ctrl it moves the caret, as in
    # any field: a letter typed then lands at the start.
    clear = field_key(field, "\u2190", "clear")
    # Neither is flush against what's typed: both stand after it, Tab's
    # first.
    typed = next(word for word in field if word["text"] == "batt")
    assert typed["x"] < tab["x"] < clear["x"], field
    machine.send_key("ctrl-left")
    machine.send_chars("x")
    desktop.launcher_until(lambda shown: shown["typed"] == "xbatt")
    machine.send_key("backspace")
    desktop.launcher_until(lambda shown: shown["typed"] == "batt")
    machine.send_key("left")
    desktop.launcher_until(lambda shown: shown["typed"] == "")
    field = json.loads(desktop.as_account("qs ipc call launcher laidOut"))["field"]
    # What it took stands in grey where it was typed, as any hint in an
    # empty field starts where the first letter will.
    took = next(word for word in field if word["text"] == "batt")
    assert took["x"] == typed["x"], field
    # The right arrow is said in the left arrow's place, which holds still,
    # and Tab's place stays empty, since there's no completion.
    assert field_key(field, "\u2192", "restore")["x"] == clear["x"], field
    assert not any("\u21e5" in word["text"] for word in field), field
    machine.send_key("right")
    desktop.launcher_until(
        lambda shown: (
            shown["typed"] == "batt"
            and shown["kind"] == "answer"
            and (shown["selected"] or {}).get("name", "").startswith(f"{charge}%")
        )
    )
    machine.send_key("tab")
    desktop.launcher_until(lambda shown: shown["typed"] == "battery")
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
        machine.send_key("esc")
    # A name starting the same way is chosen over an answer only
    # started: "pow" is Power off, by its name.
    desktop.answer("pow")
    desktop.launcher_until(
        lambda shown: (
            shown["kind"] not in ("", "answer")
            and (shown["selected"] or {}).get("name", "").lower().startswith("pow")
        )
    )
    machine.send_key("esc")


def test_the_launcher_finds_emoji_in_their_own_place_and_only_there(desktop: Desktop) -> None:
    """The launcher finds emoji in their own place, and only there."""
    machine = desktop.machine
    closed = desktop.in_session(LAUNCHER_OPEN)
    # The main search lists no emoji, only the place they're in.
    desktop.answer("tada")
    assert "Party popper" not in desktop.showing()["rows"]
    machine.send_key("esc")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
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


def test_a_categorys_word_and_a_space_step_into_it_and_a_command_finishes_by_tab(
    desktop: Desktop,
) -> None:
    """A category's word and a space step into it, and a command finishes by Tab."""
    machine = desktop.machine
    closed = desktop.in_session(LAUNCHER_OPEN)
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
    # The left arrow takes what's typed, then steps out with the word
    # put back.
    machine.send_key("left")
    desktop.launcher_until(lambda shown: shown["held"] == ["network"] and shown["typed"] == "")
    machine.send_key("left")
    desktop.launcher_until(lambda shown: shown["held"] == [] and shown["typed"] == "network")
    machine.send_key("esc")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
    # A question that starts with a category's word is still a question.
    desktop.answer("time in utc")
    desktop.launcher_until(lambda shown: shown["held"] == [] and shown["kind"] == "answer")
    machine.send_key("esc")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
    # A command is in the place whose answer it gives, its form shown; Tab
    # puts its start in the field, out at the top, where it's finished.
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    machine.send_chars("network ")
    desktop.launcher_until(lambda shown: shown["held"] == ["network"] and ip in shown["rows"])
    desktop.photograph("launcher-commands")
    desktop.type_in("ip", selecting="ip \u27e8address or network\u27e9")
    machine.send_key("tab")
    desktop.launcher_until(lambda shown: shown["held"] == [] and shown["typed"] == "ip ")
    desktop.type_in("10.0.0.0/22")
    desktop.launcher_until(lambda shown: shown["kind"] == "answer")
    machine.send_key("esc")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))


def test_every_name_a_thing_had_still_finds_it(desktop: Desktop) -> None:
    """Every name a thing had still finds it."""
    machine = desktop.machine
    closed = desktop.in_session(LAUNCHER_OPEN)
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
            machine.send_key("esc")
            machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
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
        machine.send_key("esc")
        machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
    print(f"walked {len(kept)} old names, {len(absent)} absent here: {absent}")
    assert lost == [], lost
    assert len(kept) - len(absent) >= fewest, absent


def test_a_search_lists_each_thing_once(desktop: Desktop) -> None:
    """A search lists each thing once."""
    machine = desktop.machine
    closed = desktop.in_session(LAUNCHER_OPEN)
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
        machine.send_key("esc")
        machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
    desktop.answer("zqxj")
    desktop.launcher_until(lambda shown: shown["rows"] == [])
    machine.send_key("esc")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
    # The speakers' tuning plays through them, and is no program.
    desktop.answer("volume")
    desktop.launcher_until(lambda shown: "Turn the volume up" in shown["rows"])
    assert "pebble_x_plus_tuning_output" not in desktop.showing()["rows"]
    machine.send_key("esc")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))


def test_a_step_finds_its_rows_by_their_hidden_words_and_its_parts_move_in_place(  # noqa: PLR0915
    desktop: Desktop,
) -> None:
    """A step finds its rows by their hidden words, and its parts move in place."""
    machine = desktop.machine
    closed = desktop.in_session(LAUNCHER_OPEN)
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
    # The left arrow takes what's typed away, the selection staying on
    # the VPN; the right arrow right after puts it back, and then steps
    # into it.
    machine.send_key("left")
    desktop.launcher_until(
        lambda shown: shown["typed"] == "" and (shown["selected"] or {}).get("name") == "VPN"
    )
    machine.send_key("right")
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
    machine.send_key("esc")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
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
    machine.send_key("esc")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))


def test_a_question_is_asked_on_return_and_answered_where_it_was_typed(desktop: Desktop) -> None:
    """A question is asked on Return and answered where it was typed."""
    machine = desktop.machine
    closed = desktop.in_session(LAUNCHER_OPEN)

    def selected(shown: Json) -> Json:
        return cast("Json", shown["selected"] or {})

    def close() -> None:
        machine.send_key("esc")
        machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))

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
    machine = desktop.machine
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
        machine.send_key("esc")
        desktop.launcher_until(lambda shown: not shown["open"])


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
