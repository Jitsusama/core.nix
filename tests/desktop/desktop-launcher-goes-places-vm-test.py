"""The desktop test's launcher goes places area.

Here the launcher goes places: its home, catalogue and drawers, the volume, a detail inside a
thing, programs' acts, and what an opening makes and traces.

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

# pytest finds a fixture by its name in the test's module, so the stand-in Google's is
# brought here.
from desktop_session import (
    LAUNCHER_OPEN,
    STARTING,
    google,  # noqa: F401
    lighthouse_listed,
    repeating,
)

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


def test_the_screen_never_locks_by_itself_however_long_the_test_waits(desktop: Desktop) -> None:
    """The screen never locks by itself here, however long the test goes without a key.

    A loaded builder let five minutes pass with no key from the test, typing by wtype counting
    for none, and the idle lock fell mid-run, failing every subtest after it behind the lock
    (2026-11-03). The lock is still asked for by its key and by logind.
    """
    line = desktop.as_account("systemctl --user show swayidle -p ExecStart --value")
    assert " timeout " not in line, f"swayidle locks the screen by itself: {line}"
    assert " lock " in line, f"swayidle no longer locks when logind asks: {line}"


def test_the_launcher_shows_the_volumes_chords_and_q_closes_it(desktop: Desktop) -> None:
    """The launcher shows the volume's chords, and :q closes it.

    Escape puts the line in normal mode, and :q there closes it, as Escape never does.
    """
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("volume")
    desktop.photograph("launcher-volume")
    desktop.close_launcher()


def test_an_opening_says_what_each_source_made(desktop: Desktop) -> None:
    """An opening says, once settled, how many rows each source made, and so does a closing."""
    machine = desktop.machine
    said = (
        "journalctl --user --unit=quickshell --no-pager --output=cat --since=-30s"
        " | grep -E 'launcher: {} made {} rows \\({}\\) and [0-9]+ texts{}'"
    )
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    # Opening makes rows, each source's counted and named.
    opening = said.format("opening", "[1-9][0-9]*", "[a-z]+ [1-9][0-9]*.*", "")
    machine.wait_until_succeeds(desktop.in_session(opening), timeout=dt.timedelta(seconds=20))
    desktop.close_launcher()
    # Closing is said too, though it may make nothing new, with which
    # sources changed since it opened, if any did.
    closing = said.format("closing", "[0-9]+", ".*", ", after [0-9]+ changes \\(.*\\)")
    machine.wait_until_succeeds(desktop.in_session(closing), timeout=dt.timedelta(seconds=20))


def test_the_launcher_sets_the_volume_to_a_number_and_says_how_loud_it_is(desktop: Desktop) -> None:
    """The launcher sets the volume to a number, and says how loud it is."""
    machine = desktop.machine
    # Muted first, since a level asked for is one to be heard.
    desktop.as_account("wpctl set-mute @DEFAULT_AUDIO_SINK@ 1")
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("volume 40")
    desktop.launcher_until(
        lambda shown: (shown["selected"] or {}).get("name", "").endswith(" to 40%")
    )
    desktop.land()
    machine.send_key("ret")
    # Exactly that, and no longer muted.
    machine.wait_until_succeeds(
        desktop.in_session("wpctl get-volume @DEFAULT_AUDIO_SINK@ | grep -qx 'Volume: 0.40'"),
        timeout=dt.timedelta(seconds=10),
    )
    # It changed something, so the launcher stays until it's closed.
    desktop.close_launcher()
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("volume")
    desktop.launcher_until(
        lambda shown: (shown["selected"] or {}).get("name", "").startswith("40%, through ")
    )
    desktop.photograph("launcher-volume-told")
    # A step in, each device's level drawn as a bar.
    desktop.follow()
    desktop.photograph("launcher-volume-parts")
    desktop.close_launcher()


def test_the_drawer_goes_by_the_selected_rows_kind(desktop: Desktop) -> None:
    """The drawer stays in for a place, a thing with nothing more to say, and an act."""
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    # Home opens on Now, whose drawer is the moment's plate (look 3), and the
    # place below it has no details, so the drawer goes in there (docs/design.md,
    # the drawer: it goes by kind, not by what a row happens to hold).
    desktop.launcher_until(lambda shown: (shown.get("drawer") or {}).get("moment") is not None)
    desktop.next_row()
    desktop.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("is") != "collection:on"
            and (shown.get("drawer") or {}).get("out") is False
        )
    )
    # A program found by a search is a thing, but with nothing beyond its
    # name and side the drawer would only repeat it, so it stays in (Joel,
    # 2026-10-29: "never show the side panel when it is just repeating the
    # currently selected text"). Now's meeting shows one that's out.
    desktop.type_in("kitty", selecting="kitty")
    desktop.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name") == "kitty"
            and (shown.get("drawer") or {}).get("out") is False
        )
    )
    # Its acts are not things, so going into it by Ctrl+], as Return would
    # open it, puts it away.
    desktop.follow()
    desktop.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name") != "kitty"
            and (shown.get("drawer") or {}).get("out") is False
        )
    )
    desktop.close_launcher()


# Stepping back out of a collection finds the launcher as it was left,
# on the row that was selected, not back at the top.


def test_the_launcher_remembers_where_it_was_when_stepping_back(desktop: Desktop) -> None:
    """The launcher remembers where it was when stepping back."""
    desktop.notes_window()
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()

    def selected() -> str:
        """Give the name of the launcher's selected row, or nothing."""
        return cast("str", (desktop.showing()["selected"] or {}).get("name", ""))

    # Down past the first row to Programs, a collection the VM always has
    # some of (Agents, above it, is empty here).
    while True:
        before = selected()
        desktop.next_row()
        # The wait ends before the loop moves on, so it reads this pass's name.
        desktop.launcher_until(
            lambda shown: (shown["selected"] or {}).get("name", "") != before  # noqa: B023
        )
        if desktop.showing()["selected"]["is"] == "collection:applications":
            break
    chosen = selected()
    desktop.follow()
    # Open programs come first, so kitty, whose notes are open, leads the
    # programs closed, however they're named.
    desktop.launcher_until(lambda shown: shown["rows"][:1] == ["kitty"])
    desktop.next_row()
    desktop.step_out()
    desktop.launcher_until(
        lambda shown: shown["held"] == [] and (shown["selected"] or {}).get("name") == chosen
    )
    desktop.close_launcher()


def at_rotation(shown: Json) -> bool:
    """Say whether the launcher holds a screen, inside Desktop's Screens, its Rotation selected."""
    return bool(
        shown["open"]
        and shown["path"][:2] == ["Desktop", "Screens"]
        and len(shown["path"]) == len(["Desktop", "Screens", "a screen"])
        and (shown["selected"] or {}).get("name") == "Rotation"
    )


def test_a_detail_inside_a_thing_is_found_from_the_root_and_return_goes_there(
    desktop: Desktop,
) -> None:
    """A detail inside a thing is found from the root, and Return goes there."""
    machine = desktop.machine
    where = "Desktop \u203a Screens \u203a "

    def spot(shown: Json) -> int:
        """Give where the screen's Rotation is listed, saying where it lives, or -1."""
        for at, thing in enumerate(shown["things"]):
            if thing["reads"].startswith("Rotation ") and where in thing["reads"]:
                return at
        return -1

    def find_rotation() -> None:
        """Open the launcher on the screen's Rotation, found from the root."""
        desktop.answer("rotation")
        desktop.launcher_until(lambda shown: spot(shown) >= 0)
        for _ in range(spot(desktop.showing())):
            desktop.next_row()
        desktop.launcher_until(
            lambda shown: (
                (shown["selected"] or {}).get("name") == "Rotation"
                and str((shown["selected"] or {}).get("side", "")).startswith(where)
            )
        )

    # A screen's rotation is a step inside the screen, inside Screens,
    # inside Desktop; from the root it's found by its name, saying so,
    # below the focused screen's answer. In normal, l goes nowhere, as a
    # row has no line to move along, and says Vim's way in instead, the
    # foot naming Return and - by the keys that do them (Joel, 2026-11-05:
    # "where in vi does that step into shit?").
    find_rotation()
    desktop.normal()
    told = desktop.showing()["selected"]["told"]
    assert told[0] == "\u21b5 go", told
    assert not any(said.startswith(("l ", "h ")) for said in told), told
    machine.send_key("l")
    desktop.launcher_until(
        lambda shown: shown["saying"] == ["l not here", "\u21b5 open"] and shown["held"] == []
    )
    # Return goes there: the crumbs say each step, and the spot is selected.
    machine.send_key("ret")
    desktop.launcher_until(at_rotation)
    # Its three steps are one jump, as Vim's :tag is: Ctrl+O goes straight back to the
    # search it was found by, and Tab, which is Ctrl+I, forward to it again.
    desktop.normal()
    machine.send_key("ctrl-o")
    desktop.launcher_until(lambda shown: shown["held"] == [] and shown["typed"] == "rotation")
    machine.send_key("tab")
    desktop.launcher_until(at_rotation)
    screen = desktop.showing()["path"][-1]
    # h goes nowhere either, saying -, which goes back up, onto the
    # screen it was inside.
    desktop.normal()
    machine.send_key("h")
    desktop.launcher_until(lambda shown: shown["saying"] == ["h not here", "- up"])
    machine.send_key("minus")
    desktop.launcher_until(
        lambda shown: (
            shown["path"] == ["Desktop", "Screens"]
            and (shown["selected"] or {}).get("name") == screen
        )
    )
    # And on out the way it came, each step with the row it went through
    # selected, to the search it was found by (Joel, 2026-10-26: every way
    # in climbs back out the same way).
    machine.send_key("minus")
    desktop.launcher_until(
        lambda shown: (
            shown["path"] == ["Desktop"] and (shown["selected"] or {}).get("name") == "Screens"
        )
    )
    machine.send_key("minus")
    desktop.launcher_until(
        lambda shown: (
            shown["held"] == []
            and shown["typed"] == "rotation"
            and (shown["selected"] or {}).get("name") == "Rotation"
            and str((shown["selected"] or {}).get("side", "")).startswith(where)
        )
    )
    desktop.close_launcher()
    # An answer's part is found the same way: the stand-in battery's
    # Health, inside Battery, inside Power, inside Machine.
    inside = "Machine \u203a Power \u203a Battery"

    def health(shown: Json) -> int:
        """Give where the battery's Health is listed, saying where it lives, or -1."""
        for at, thing in enumerate(shown["things"]):
            if thing["reads"].startswith("Health ") and inside in thing["reads"]:
                return at
        return -1

    desktop.answer("health")
    desktop.launcher_until(lambda shown: health(shown) >= 0)
    for _ in range(health(desktop.showing())):
        desktop.next_row()
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("side") == inside)
    machine.send_key("ret")
    desktop.launcher_until(
        lambda shown: (
            shown["path"] == ["Machine", "Power", "Battery"]
            and (shown["selected"] or {}).get("name") == "Health"
        )
    )
    desktop.close_launcher()


def test_the_first_search_after_the_shell_starts_finds_everything_made_ready(
    desktop: Desktop,
) -> None:
    """The first search after the shell starts finds details and texts made ready already."""
    # Every switch restarts the shell, and its first search of three letters
    # made every detail inside things there and then, that opening's slowest
    # key; a few seconds after it starts, they're made while it's closed.
    # The mark comes after the restart, since the shell being stopped makes
    # them too, half a second after the last subtest closed it.
    desktop.as_account("systemctl --user restart quickshell")
    since = desktop.mark()
    desktop.reported(
        "spots made while closed ",
        lambda rest: json.loads(rest)["kept"] == [],
        since,
        dt.timedelta(seconds=60),
    )
    # So are the texts every collection's search looks through, which its
    # first search of two letters made, 968 ms on one of Joel's keys.
    desktop.reported(
        "texts made ready while closed", lambda rest: rest == "", since, dt.timedelta(seconds=60)
    )
    since = desktop.mark()
    desktop.answer("rotation")
    desktop.launcher_until(
        lambda shown: any(thing["reads"].startswith("Rotation ") for thing in shown["things"])
    )
    desktop.close_launcher()
    said = desktop.as_account(
        "journalctl --user --unit=quickshell --no-pager --output=cat"
        f" --after-cursor={shlex.quote(since)}"
    )
    assert "report[spots made on a key]" not in said, said
    # Only the queue and Agents, which make their rows only while the
    # launcher's open, so a closed launcher has none of theirs to make
    # ready, have any to make.
    made = [
        json.loads(line.split("report[texts made ready on a key ", 1)[1].removesuffix("]"))
        for line in said.splitlines()
        if "report[texts made ready on a key " in line
    ]
    assert all(set(each) <= {"jobs", "agents"} for each in made), made


def test_a_closing_makes_again_only_the_details_whose_source_changed(
    desktop: Desktop,
) -> None:
    """A closing makes again only the details inside things whose source changed since."""
    # Every closing made each detail inside things again, though nothing it
    # was made from had changed: Sound's 111 rows on each of Joel's closings,
    # with no change to Sound at all. The first closing here makes what
    # changed since the last subtest's; the second, with nothing done to
    # Sound between them, keeps Sound's.
    said: list[dict[str, list[str]]] = []

    def heard(rest: str) -> bool:
        said.append(json.loads(rest))
        return True

    def closing() -> dict[str, list[str]]:
        """Open the launcher and close it, giving what that closing made and kept."""
        since = desktop.mark()
        desktop.answer("rotation")
        desktop.launcher_until(
            lambda shown: any(thing["reads"].startswith("Rotation ") for thing in shown["things"])
        )
        desktop.close_launcher()
        desktop.reported("spots made while closed ", heard, since)
        return said[-1]

    closing()
    assert "computer/sound" in closing()["kept"], said[-1]
    # A battery coming or going is a change to Power, so the next closing
    # makes Battery's details again. UPower can list it only after the
    # details were first made, since a switch restarts both, and they were
    # kept without Health from then on: the battery's Health was never
    # found from the root (the net's run of this file, 2026-11-07).
    machine = desktop.machine
    battery = "upower --enumerate | grep -q test_battery"
    machine.succeed("rmmod test_power")
    machine.wait_until_fails(battery)
    assert "energy/power" in closing()["made"], said[-1]
    machine.succeed("modprobe test_power")
    machine.wait_until_succeeds(battery)
    assert "energy/power" in closing()["made"], said[-1]


def test_an_opening_is_traced_and_sent_to_the_collector_as_it_closes(desktop: Desktop) -> None:
    """An opening is traced, a span a step with where its first row lay, and sent as it closes.

    Joel saw a void above a menu's first row under load, and asked that the shell trace itself
    (2026-10-31). Each opening is a trace: a step into Machine and the step back home are spans
    under it, each ended as its list settled, holding where the first row lay. As it closes the
    opening is sent whole, one line, to the collector.
    """
    machine = desktop.machine
    heard = "$XDG_RUNTIME_DIR/collector/heard.jsonl"
    closed = desktop.in_session(LAUNCHER_OPEN)
    # First an opening with no collector listening, as when the collector
    # is down: it's dropped without a warning, and what's sent after it
    # still arrives.
    since = desktop.mark()
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.as_account("qs ipc call launcher toggle")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
    desktop.as_account("systemctl --user start stand-in-collector")
    try:
        machine.wait_until_succeeds(
            desktop.in_session("test -S $XDG_RUNTIME_DIR/collector/lines.sock"),
            timeout=dt.timedelta(seconds=10),
        )
        desktop.as_account("qs ipc call launcher toggle")
        desktop.launcher_ready()
        desktop.down_to("Machine")
        desktop.follow()
        desktop.launcher_until(lambda shown: shown["path"][-1:] == ["Machine"])
        desktop.step_out()
        desktop.launcher_until(lambda shown: shown["held"] == [])
        desktop.as_account("qs ipc call launcher toggle")
        machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
        spans: list[Json] = json.loads(desktop.as_account("qs ipc call launcher traced"))
        opening = [span for span in spans if span["name"] == "open"][-1]
        mine = [span for span in spans if span["trace"] == opening["trace"]]
        assert opening["end"] is not None, opening
        steps = [span for span in mine if span["name"] == "step"]
        backs = [span for span in mine if span["name"] == "back"]
        assert [span["about"]["into"] for span in steps] == ["category:computer"], mine
        assert len(backs) == 1, mine
        for span in steps + backs:
            assert span["parent"] == opening["span"], span
            assert span["end"] is not None, span
            assert span["about"]["firstAt"] == 0, span
        # Sent whole as it closed: one line, its trace's spans in it.
        line = machine.wait_until_succeeds(
            desktop.in_session(f"grep -a {opening['trace']} {heard} | tail -1 | grep ."),
            timeout=dt.timedelta(seconds=10),
        )
        sent = json.loads(line)["resourceSpans"][0]
        service = sent["resource"]["attributes"][0]["value"]["stringValue"]
        assert service == "launcher", sent["resource"]
        names = sorted(span["name"] for span in sent["scopeSpans"][0]["spans"])
        assert {"open", "step", "back"} <= set(names), names
        # Both sends were over by the time the second arrived. The sender is
        # the one socket with a property of its own, so Qt names it
        # Socket_QML_<n>; a daemon not up yet warns as a plain Socket as it
        # retries, which isn't the send's.
        warned = desktop.as_account(
            "journalctl --user -u quickshell --no-pager -o cat"
            f" --after-cursor={shlex.quote(since)}"
            " | grep -a -F quickshell.io.socket | grep -a -F Socket_QML_; true"
        )
        assert warned.strip() == "", f"a send warned:\n{warned}"
    finally:
        desktop.as_account("systemctl --user stop stand-in-collector")


@pytest.mark.usefixtures("google")
def test_a_programs_acts_are_found_from_the_top_by_their_names_below_the_program(
    desktop: Desktop,
) -> None:
    """A program's acts are found from the top by their own names, saying where they live.

    Return does the act, as it does inside the program, and Ctrl+] in normal
    goes into the program onto it, where it lives, so - climbs back out. Typing the program's
    name lists the program first, and only those acts whose own names it starts a word of.
    """
    machine = desktop.machine
    closed = desktop.in_session(LAUNCHER_OPEN)
    opened = "/run/user/1000/google-opened"
    machine.succeed(f"rm -f {opened}")
    # Meet's desktop entry's own New Meeting, by its name.
    desktop.answer("new meeting")
    desktop.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name") == "New Meeting"
            and shown["selected"]["note"] == "Programs \u203a Meet"
        )
    )
    # Ctrl+] goes into Meet, where it lives, onto New Meeting, the foot
    # saying so furthest from Return.
    desktop.normal()
    told = desktop.showing()["selected"]["told"]
    assert told[1:2] == ["Ctrl+] in"], told
    desktop.follow()
    desktop.launcher_until(
        lambda shown: (
            # Programs, then Meet.
            shown["held"][1:] == ["Meet"] and (shown["selected"] or {}).get("name") == "New Meeting"
        )
    )
    desktop.as_account("qs ipc call launcher toggle")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
    # Return starts it, as inside Meet, and the launcher goes.
    desktop.answer("new meeting")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "New Meeting")
    machine.send_key("ret")
    machine.wait_until_succeeds(
        f"grep -qxF https://meet.google.com/new {opened}", timeout=dt.timedelta(seconds=30)
    )
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
    # Meet's own acts it opts in: Join by Code asks for its code there.
    desktop.answer("join by code")
    desktop.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name") == "Join by Code"
            and shown["selected"]["note"] == "Programs \u203a Meet"
        )
    )
    machine.send_key("ret")
    desktop.launcher_until(lambda shown: shown["held"][-2:] == ["Meet", "Join by Code"])
    machine.send_chars("abc-defg-hij")
    machine.send_key("ret")
    machine.wait_until_succeeds(
        f"grep -qxF https://meet.google.com/abc-defg-hij {opened}",
        timeout=dt.timedelta(seconds=30),
    )
    desktop.close_launcher()
    # And Copy a New Meeting's Link, while Meet's scope is granted.
    desktop.answer("copy a new meeting")
    desktop.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name") == "Copy a New Meeting's Link"
            and shown["selected"]["note"] == "Programs \u203a Meet"
        )
    )
    desktop.as_account("qs ipc call launcher toggle")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
    # Typing the program's name finds it first, and an act only by its own
    # name, never by the program's in its note.
    desktop.answer("meet")
    desktop.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name") == "Meet"
            and "New Meeting" in shown["rows"]
            and shown["rows"].index("Meet") < shown["rows"].index("New Meeting")
            and "Join by Code" not in shown["rows"]
        )
    )
    desktop.as_account("qs ipc call launcher toggle")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))


def test_machine_holds_power_which_holds_the_battery_and_what_ends_a_session(
    desktop: Desktop,
) -> None:
    """Machine holds Power, which holds the battery, its profile and what ends a session."""
    machine = desktop.machine
    power = [
        "Battery", "Power Profile", "Keep Awake", "Sleep", "Lock", "Log Out", "Reboot",
        "Power Off",
    ]  # fmt: skip
    # Machine lists Lag only once lag-watch has written its first ten
    # seconds, and reads it as it's stepped into, never after. In a session
    # that has long gone by; run first, this subtest can beat it there.
    machine.wait_until_succeeds(
        desktop.in_session('test -s "$XDG_RUNTIME_DIR/lag-watch/status.json"'),
        timeout=STARTING,
    )
    desktop.answer("machine")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Machine")
    desktop.step_in("ret")
    # Power comes first, saying how it stands, the charge and what it's
    # doing, as Processor says how busy it is, never a list of what it
    # holds. Memory is a row of its own, before Heat, as the processor is,
    # Disk after it and Lag last, since what it points at is the
    # processor, the memory and the disk.
    # The queue is Agents', so Machine keeps only the machine's own.
    desktop.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name") == "Power"
            and "Queue" not in shown["rows"]
            and any(
                re.match(r"Power \u00b7 \d+%, ", str(thing["reads"])) for thing in shown["things"]
            )
            and "Memory" in shown["rows"]
            and shown["rows"][shown["rows"].index("Memory") + 1 :][:3] == ["Heat", "Disk", "Lag"]
        )
    )
    desktop.step_in("ret")
    desktop.launcher_until(
        lambda shown: shown["path"] == ["Machine", "Power"] and shown["rows"][:8] == power
    )
    # The battery's figure holds more, so Return steps into it, as the right
    # arrow does, rather than copying and closing, and its copy is listed
    # last inside (Joel, 2026-10-28).
    desktop.down_to("Battery")
    machine.send_key("ret")
    desktop.launcher_until(
        lambda shown: (
            shown["open"]
            and shown["path"] == ["Machine", "Power", "Battery"]
            and shown["rows"][-1:] == ["Copy"]
        )
    )
    desktop.close_launcher()
    # Found by its name from the top, it opens where it lives, and -, in
    # normal mode, climbs out through Machine.
    desktop.answer("power")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Power")
    desktop.step_in("ret")
    desktop.launcher_until(
        lambda shown: shown["path"] == ["Machine", "Power"] and shown["rows"][:8] == power
    )
    desktop.normal()
    machine.send_key("minus")
    desktop.launcher_until(
        lambda shown: (
            shown["path"] == ["Machine"] and (shown["selected"] or {}).get("name") == "Power"
        )
    )
    # A name going on from the word and a space is searched for where it's
    # typed, so "power off" never steps into Power under the keys.
    desktop.close_launcher()
    desktop.answer("power off")
    desktop.launcher_until(
        lambda shown: shown["path"] == [] and (shown["selected"] or {}).get("name") == "Power Off"
    )
    desktop.close_launcher()


def test_lag_stands_in_its_place_until_its_first_ten_seconds_then_says_them(
    desktop: Desktop,
) -> None:
    """Lag stands in its place in Machine until its first ten seconds, then says them."""
    # As just after a start, before lag-watch has written its first window
    # (UTMDGL, 2026-11-03): Lag was missing from Machine then, and stayed
    # missing while Machine was open, however long after the window came.
    desktop.as_account(
        "systemctl --user stop lag-watch.service && rm -f $XDG_RUNTIME_DIR/lag-watch/status.json"
    )

    def lag(shown: Json) -> Json | None:
        """Lag's row as it's shown in its place after Disk, or None."""
        rows = shown["rows"]
        if "Disk" not in rows or rows[rows.index("Disk") + 1 :][:1] != ["Lag"]:
            return None
        return next(
            (thing for thing in shown["things"] if str(thing["reads"]).startswith("Lag")), None
        )

    try:
        desktop.answer("machine")
        desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Machine")
        desktop.step_in("ret")
        desktop.launcher_until(lambda shown: (lag(shown) or {}).get("gone") is True)
        # Its first ten seconds say themselves where it stands, with
        # Machine still open.
        desktop.as_account("systemctl --user start lag-watch.service")
        desktop.launcher_until(
            lambda shown: shown["path"] == ["Machine"] and (lag(shown) or {}).get("gone") is False,
            timeout=dt.timedelta(seconds=40),
        )
        desktop.close_launcher()
    finally:
        desktop.as_account("systemctl --user start lag-watch.service")


def test_the_launchers_catalogue_opens_places_does_actions_and_offers_what_ends_something(
    desktop: Desktop,
) -> None:
    """The launcher's catalogue opens places, does actions and offers what ends something."""
    machine = desktop.machine
    closed = desktop.in_session(LAUNCHER_OPEN)
    held = "systemd-inhibit --list --no-pager | grep Caffeinate"
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))

    def listed() -> list[Json]:
        return cast("list[Json]", json.loads(desktop.as_account("qs ipc call launcher list")))

    def ran(entry: Json) -> Json:
        said = desktop.as_account(f"qs ipc call launcher run {shlex.quote(str(entry['id']))}")
        return cast("Json", json.loads(said))

    def one(entries: list[Json], words: str, within: str) -> Json:
        found = [e for e in entries if e["words"] == words and e["in"] == within]
        assert len(found) == 1, f"{words} in {within}: {found}"
        return found[0]

    entries = listed()
    ids = [entry["id"] for entry in entries]
    assert len(ids) == len(set(ids)), "an id is listed twice"
    # Programs lists each program, but not each one's own Quit.
    kitty = one(entries, "kitty", "Programs")
    assert kitty["kind"] == "action"
    assert not [e for e in entries if e["in"].startswith("Programs \u203a ")]
    # Windows lists each window but not each one's own Size, and a
    # keyboard's keys, which only say what they do, aren't listed.
    assert not [e for e in entries if e["in"].startswith("Desktop \u203a Windows \u203a ")]
    assert not [e for e in entries if "keyboard:key:" in e["id"]]
    # A collection's own parts are listed, as a step into it shows them:
    # Agents' Orchestrators, and inside it each orchestrator.
    lighthouse_listed(desktop)
    machine.wait_until_succeeds(
        desktop.in_session("qs ipc call launcher list | grep -q Lighthouse"),
        timeout=dt.timedelta(seconds=10),
    )
    # Run, it does what its Return does, going to its window or opening it
    # again where it's closed (#1069), so the launcher doesn't open on it.
    lighthouse = one(listed(), "Lighthouse", "Agents \u203a Orchestrators")
    assert lighthouse["kind"] == "action", lighthouse
    assert ran(lighthouse)["done"]
    machine.fail(closed)
    # A place opens there.
    assert ran(one(entries, "Power", "Machine")) == {
        "done": True,
        "said": "opened Machine \u203a Power",
    }
    desktop.launcher_until(lambda shown: shown["open"] and shown["path"] == ["Machine", "Power"])
    desktop.close_launcher()
    # One that ends something is shown ready, never done.
    reboot = one(entries, "Reboot", "Machine \u203a Power")
    assert reboot["disruptive"]
    assert ran(reboot)["offered"]
    desktop.launcher_until(
        lambda shown: (
            shown["path"] == ["Machine", "Power"]
            and (shown["selected"] or {}).get("name") == "Reboot"
        )
    )
    desktop.close_launcher()
    # A harmless one is done at once, and once it ends something, offered.
    awake = one(entries, "Keep Awake", "Machine \u203a Power")
    assert not awake["disruptive"]
    assert ran(awake) == {"done": True, "said": "Machine \u203a Power \u203a Keep Awake"}
    machine.wait_until_succeeds(f"{held} | grep -qw idle", timeout=dt.timedelta(seconds=10))
    sleep = one(listed(), "Let It Sleep", "Machine \u203a Power")
    assert sleep["id"] == awake["id"], "its id changed with its state"
    assert ran(sleep)["offered"]
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Let It Sleep")
    desktop.land()
    machine.send_key("ret")
    machine.wait_until_fails(held, timeout=dt.timedelta(seconds=10))
    desktop.close_launcher()
    # One not there says so.
    assert "error" in ran({"id": "nothing:here"})


def test_every_row_has_a_return_act(desktop: Desktop) -> None:
    """Every row the launcher shows does something on Return."""
    # Joel, 2026-10-26: a row that holds more steps in, a row that only
    # shows a value copies it, and a row that can do neither doesn't
    # belong in the launcher. The launcher walks every row from home down.
    said = cast("Json", json.loads(desktop.as_account("qs ipc call launcher actless")))
    actless = cast("list[Json]", said["rows"])
    print(f"walked {said['walked']} rows")
    # Home, its places, what each holds and deeper, which in the VM is a
    # few hundred rows: a walk that stopped at home would find nothing wrong.
    fewest = 100
    assert said["walked"] > fewest, said["walked"]
    assert actless == [], "\n".join(f"{row['in']} \u203a {row['words']}" for row in actless)
    # A thing with one row of its own, as the time, the date and the
    # weather are, shows as that row wherever it's listed, never as a
    # version a surface drew itself (Joel, 2026-10-28).
    assert said["ownVersions"] == [], "\n".join(said["ownVersions"])
    # And no step leads with a copy where it holds more, as docs/design.md's
    # Return rule has it: "Copy is never the first row of a step, unless
    # it's the only one, and is always the last".
    copy_first = cast("list[Json]", said["copyFirst"])
    assert copy_first == [], "Copy leads a step, though it comes last:\n" + "\n".join(
        f"{row['in']} \u203a {row['words']}" for row in copy_first
    )
    # And a row whose Return steps in over an act of its own leads the step
    # with that act, as docs/design.md's Return rule has it: "its likeliest
    # act first inside, so Return twice does it".
    skipped = cast("list[Json]", said["skipped"])
    assert skipped == [], "A step doesn't lead with the act Return stepped over:\n" + "\n".join(
        f"{row['in']} \u203a {row['words']}" for row in skipped
    )
    # And no row's drawer only says the row again (docs/design.md, the
    # drawer).
    assert repeating(said) == "", "A drawer only repeats its row:\n" + repeating(said)
    # And nothing holds words or a picture for a drawer it never gets,
    # since the drawer goes by kind (docs/design.md, the drawer): a thing
    # whose maker forgot to say so, or words with nowhere to be read.
    assert said["undrawn"] == [], "Held for a drawer, though not a thing:\n" + "\n".join(
        said["undrawn"]
    )


def test_the_launchers_home_lists_the_categories_to_browse_each_saying_what_it_holds(
    desktop: Desktop,
) -> None:
    """The launcher's home lists the categories to browse, each saying what it holds."""
    machine = desktop.machine
    # Desktop holds This window while one is focused, so one is, whatever
    # ran before.
    desktop.focused_kitty()
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    # The same rows every time: Now first, then each place in the order
    # the approved proposal gives them, Disk inside Machine (Joel, 2026-10-26),
    # and People after Programs (RSCH-20261028-2NK1E5).
    home = ["Now", "Agents", "Programs", "People", "Desktop", "Machine", "Network", "Keyboard"]
    desktop.launcher_until(lambda shown: shown["rows"] == home)

    def selected_is() -> str:
        """Give what the launcher's selected row is, or nothing."""
        return cast("str", (desktop.showing()["selected"] or {}).get("is", ""))

    def foot() -> tuple[int, int]:
        """Give where the foot's keys start, and how long how things stand reads."""
        shown = desktop.showing()
        return shown["keysAt"], len(shown["glance"])

    # Down through every row to the last. The foot holds still on the
    # way, whatever each row's keys say: the keys in their places, and
    # how things stand beside them as long (a minute turning keeps its
    # length).
    feet = {selected_is(): foot()}
    for _row in home[1:]:
        before = selected_is()
        desktop.next_row()
        # The wait ends before the loop moves on, so it reads this pass's row.
        desktop.launcher_until(
            lambda shown: (shown["selected"] or {}).get("is", "") != before  # noqa: B023
        )
        feet[selected_is()] = foot()
    assert selected_is() == "collection:keyboard"
    assert len(set(feet.values())) == 1, f"the foot moved as the arrows went: {feet}"
    # Each says what it holds after its name, in its own order, from the
    # vocabulary; Now says what's on, or that nothing is.
    reads = {thing["is"]: thing["reads"] for thing in desktop.showing()["things"]}
    assert reads["category:desktop"].startswith("Desktop \u00b7 This Window, Windows, Screens, "), (
        f"desktop reads {reads['category:desktop']!r}"
    )
    assert reads["category:computer"].startswith("Machine \u00b7 Power, Bluetooth, "), (
        f"machine reads {reads['category:computer']!r}"
    )
    # A place names the places it holds, by their names, never the words
    # its typed commands start with: Network once read "ip, is, split, mac".
    held = reads["category:network"].split(" \u00b7 ", 1)[1].split(", ")
    assert all(name[:1].isupper() or name == "\u2026" for name in held), (
        f"network reads {reads['category:network']!r}"
    )
    assert reads["collection:on"].startswith("Now \u00b7 "), f"now reads {reads['collection:on']!r}"
    assert reads["collection:applications"].startswith("Programs \u00b7 "), reads
    # Home holds still while the machine's queue changes, which a busy
    # machine writes all the time: it once made home again and moved it
    # from under the selection. Nothing of the machine's shows at home,
    # so the queue asks nothing again there at all.
    scrolled = desktop.showing()
    since = desktop.mark()
    queued = {"v": 1, "heartbeat": float(machine.succeed("date +%s")), "machine": {}, "jobs": []}
    desktop.as_account(
        f"echo '{json.dumps(queued)}' > $XDG_RUNTIME_DIR/jobs/.q"
        " && mv $XDG_RUNTIME_DIR/jobs/.q $XDG_RUNTIME_DIR/jobs/queue.json"
    )
    still = desktop.showing()
    assert (still["scrolled"], still["selected"]) == (scrolled["scrolled"], scrolled["selected"]), (
        f"home moved as the queue changed: from {scrolled} to {still}"
    )
    desktop.photograph("launcher-home-categories")
    desktop.close_launcher()
    said = desktop.as_account(
        f"journalctl --user --unit=quickshell --no-pager --output=cat --after-cursor='{since}'"
    )
    assert "asked again by jobs" not in said, f"the queue asked home again: {said}"
    # Ctrl+Y, which completes, does nothing and says nothing with nothing
    # typed, since the arrows reach every category; Return on one steps
    # into it. The foot names no key for completing either.
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    first = desktop.showing()["selected"]
    assert not any(key.startswith(("\u21e5", "Ctrl+Y")) for key in first["told"]), first
    machine.send_key("ctrl-y")
    desktop.next_row()
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("is") == "collection:agents")
    # Had Ctrl+Y moved to the first place, the arrow would be past it.
    assert first["is"] == "collection:on", first
    # Past Programs and People.
    for _step in range(3):
        desktop.next_row()
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("is") == "category:desktop")
    desktop.step_in("ret")
    desktop.launcher_until(
        lambda shown: (
            shown["held"] == ["desktop"]
            # This window is there only while one is focused.
            and [row for row in shown["rows"] if row != "This Window"][:2] == ["Windows", "Screens"]
        )
    )
    desktop.close_launcher()


# The programs using the processors and the memory are read as the
# launcher opens, so they're among the parts the first time each is
# asked, even stepped into at once, where nothing is asked again.


def test_the_machines_programs_are_there_the_first_time_its_asked(desktop: Desktop) -> None:
    """The machine's programs are there the first time it's asked."""
    for figure in ("processor", "memory"):
        desktop.answer(figure)
        desktop.follow()
        desktop.launcher_until(lambda shown: "Using It" in shown["rows"])
        desktop.close_launcher()


def test_a_word_typed_whole_lands_on_what_it_says(desktop: Desktop) -> None:
    """A word typed whole lands on what it says."""
    # Agents lists pi's sessions, so there has to be one.
    desktop.pi_session()

    def selects(key: str) -> Callable[[Json], bool]:
        """Give a test that Return's row is the one with this key."""
        return lambda shown: bool(shown["selected"]) and shown["selected"]["is"] == key

    # Each by its one word, the name it shows. "light" once found
    # nothing.
    lands: list[tuple[str, Callable[[Json], bool]]] = [
        ("do not disturb", selects("session:quiet")),
        ("agents", selects("collection:agents")),
        ("clipboard", selects("collection:clipboard")),
        ("light", lambda shown: "Brighten the screen" in shown["rows"]),
    ]
    for typed, holds in lands:
        desktop.answer(typed)
        desktop.launcher_until(holds)
        desktop.close_launcher()


def test_capture_recent_finds_past_its_nine_by_name_then_by_what_a_screenshot_says(
    desktop: Desktop,
) -> None:
    """Typing finds what was taken past Recent's nine, by its name and then by what it says.

    At the top, Recent's matches are one row, and Return steps into Capture's Recent with the
    search kept: the screenshot named for the words first, then the one whose words hold them,
    the line that holds them below it. The screen-text daemon's stand-in is asked each text
    once, on one connection.
    """
    machine = desktop.machine
    shots = desktop.pictures
    named = f"{shots}/wombat plan.png"
    said = f"{shots}/2026-10-01 09-00-00.png"
    # Both older than the nine newest, so only a search past them finds them.
    newer = [f"{shots}/2026-10-2{day} 09-00-00.png" for day in range(10)]
    texts = {said: ["Standup", "The wombat review moved to Friday"], named: ["Wombat"]}
    # Each is a picture, one pixel, since the drawer shows the one selected and an empty
    # file would make it warn that it can't be read.
    pixel = (
        "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhf"
        "DwAChwGA60e6kgAAAABJRU5ErkJggg=="
    )
    # Each is taken on the day it's named for, since files touched together share their time
    # and Recent's nine of the ten would then be any of them.
    dated = "".join(
        f" && touch -d '2026-10-2{day} 09:00' {shlex.quote(path)}" for day, path in enumerate(newer)
    )
    desktop.as_account(
        f"mkdir -p {shlex.quote(shots)}"
        f" && printf %s {pixel} | base64 -d > {shlex.quote(named)}"
        f" && for each in {' '.join(shlex.quote(path) for path in [said, *newer])};"
        f' do cp {shlex.quote(named)} "$each"; done'
        f" && touch -d '2026-09-01' {shlex.quote(named)} {shlex.quote(said)}"
        f"{dated}"
        f" && printf %s {shlex.quote(json.dumps(texts))} > $XDG_RUNTIME_DIR/screen-text-texts.json"
        " && rm -f $XDG_RUNTIME_DIR/screen-text-asked"
    )
    # What was taken is read as the launcher opens, so Recent showing the newest says it's read.
    desktop.capture()
    desktop.type_in("recent", selecting="Recent")
    desktop.follow()
    desktop.launcher_until(lambda shown: "2026-10-29 09-00-00.png" in shown["rows"])
    desktop.as_account("qs ipc call launcher toggle")
    desktop.answer("wombat")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "wombat plan.png")
    machine.send_key("ret")
    desktop.launcher_until(
        lambda shown: (
            shown["held"][-1:] == ["Recent"]
            and shown["typed"] == "wombat"
            and shown["rows"] == ["wombat plan.png", "2026-10-01 09-00-00.png"]
        )
    )
    desktop.down_to("2026-10-01 09-00-00.png")
    desktop.launcher_until(
        lambda shown: shown["selected"]["note"] == "The wombat review moved to Friday"
    )
    asked = desktop.as_account("cat $XDG_RUNTIME_DIR/screen-text-asked").split("\n")
    searches = [line for line in asked if line.startswith("search ")]
    assert asked.count("connected") == 1, f"screen-text wasn't connected to once: {asked}"
    assert "search wombat" in searches, f"wombat was never asked: {searches}"
    assert len(searches) == len(set(searches)), f"a text was asked twice: {searches}"
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_until(lambda shown: not shown["open"])
    # Closed, the launcher holds no connection to it.
    machine.wait_until_succeeds(
        desktop.in_session("tail -1 $XDG_RUNTIME_DIR/screen-text-asked | grep -qx gone"),
        timeout=dt.timedelta(seconds=10),
    )
    desktop.as_account(
        f"rm -f {shlex.quote(named)} {shlex.quote(said)}"
        f" {' '.join(shlex.quote(path) for path in newer)}"
    )


# A binding that throws or assigns the wrong type only warns, and
# only once a surface draws it, which can be long after the start.


def test_quickshell_warned_of_nothing_it_drew_all_along(desktop: Desktop) -> None:
    """Quickshell warned of nothing it drew all along."""
    # Quickshell colours its levels and categories even in the journal,
    # so a colour code may stand between the two words.
    log = desktop.as_account(
        "journalctl --user -u quickshell --no-pager -o cat | grep -a -E 'WARN[^ ]* scene' || true"
    )
    assert log.strip() == "", f"Quickshell warned of a scene: {log}"
