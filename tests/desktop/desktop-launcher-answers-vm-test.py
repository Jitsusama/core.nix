"""The desktop test's launcher answers area.

Here the launcher answers: everything Joel types lists each thing once, its answers are copied,
and emoji keep their own place. tests/desktop.nix runs this through pytest inside the driver
(tests/lib/vm_pytest.py), and its opening comment says what the test proves. Every test asks for
the desktop, started once a run (desktop_session.py), and waits at its start for what it needs,
so any test runs alone.
"""

from __future__ import annotations

import datetime as dt
import json
import re
import shlex
import time
from typing import TYPE_CHECKING, Any, cast

import desktop_session
import pytest
from desktop_session import LAUNCHER_OPEN, ROWS_LOOKED_AT, STARTING

if TYPE_CHECKING:
    from collections.abc import Callable

    from desktop_session import Desktop, Json
    from test_driver.machine import BaseMachine


@pytest.fixture(scope="module")
def desktop(machine: BaseMachine, values: dict[str, Any]) -> Desktop:
    """Give the desktop, started once a run."""
    return desktop_session.started(machine, cast("desktop_session.Values", values))


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


# One walk, as its subtest was: each step starts where the last left it.
def test_the_launcher_answers_whats_asked_and_copies_the_answer(  # noqa: PLR0915
    desktop: Desktop,
) -> None:
    """The launcher answers what's asked, and copies the answer."""
    machine = desktop.machine

    def copied(typed: str, expected: str) -> None:
        """Ask the launcher, take its answer with Return, and check that it copied this."""
        desktop.answer(typed)
        # It says so as it closes.
        with desktop.told("parted copied"):
            machine.send_key("ret")
        closed = desktop.in_session(LAUNCHER_OPEN)
        machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
        assert desktop.as_account("wl-paste --no-newline") == expected

    # What a sum or a measure says is tests/answers.qml's to hold; here,
    # only what needs the machine. The vendor comes from the list built
    # from hwdata, so this proves it's there, and that Return copies.
    copied("3c:22:fb:12:34:56", "Apple, Inc.")
    # The battery's charge leads, as the kernel's stand-in has it.
    charge = machine.succeed("cat /sys/class/power_supply/test_battery/capacity").strip()
    desktop.answer("battery")
    desktop.photograph("launcher-battery")
    machine.send_key("ret")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10))
    assert desktop.as_account("wl-paste --no-newline").startswith(f"{charge}%, ")
    # Then what feeds it: the stand-in's charger, plugged in, a step
    # into the answer's row.
    desktop.answer("charger")
    desktop.launcher_until(lambda shown: shown["kind"] == "answer")
    desktop.step_in()
    # Return's own Copy leads, then the charger.
    desktop.launcher_until(lambda shown: shown["rows"][:2] == ["Copy", "Charger"])
    machine.send_key("down")
    desktop.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name") == "Charger"
            and shown["selected"]["side"] == "plugged in"
        )
    )
    # Its charge among them, drawn as a bar, and none of them a glyph.
    desktop.photograph("launcher-charger-parts")
    # Each part is named first, then what it says: the stand-in's health
    # is good, and its temperature has a bar.
    desktop.launcher_until(
        lambda shown: (
            {"Charger", "Charge", "Health", "Temperature"} <= set(shown["rows"])
            and any(thing["reads"].startswith("Health \u00b7 Good, ") for thing in shown["things"])
        )
    )
    machine.send_key("esc")
    # A part is asked by its own words, its figure the headline.
    desktop.answer("battery health")
    desktop.launcher_until(
        lambda shown: (
            shown["kind"] == "answer"
            and (shown["selected"] or {}).get("name", "").startswith("Good, ")
        )
    )
    machine.send_key("esc")
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
    # Tab finishes the word, its key after the completion in the field.
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
    assert any("\u21e5" in word["text"] for word in field), field
    # The left arrow takes what's typed away only where the field says
    # so, its mark after the completion's, and the right arrow right
    # after puts it back, what it took standing in grey with the right
    # arrow's mark meanwhile. With Ctrl it moves the caret, as in any
    # field: a letter typed then lands at the start.
    assert any("\u2190" in word["text"] for word in field), field
    machine.send_key("ctrl-left")
    machine.send_chars("x")
    desktop.launcher_until(lambda shown: shown["typed"] == "xbatt")
    machine.send_key("backspace")
    desktop.launcher_until(lambda shown: shown["typed"] == "batt")
    machine.send_key("left")
    desktop.launcher_until(lambda shown: shown["typed"] == "")
    field = json.loads(desktop.as_account("qs ipc call launcher laidOut"))["field"]
    assert any(word["text"].startswith("batt ") and "\u2192" in word["text"] for word in field), (
        field
    )
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
    # The hottest part, from the sensors the kernel lists, of which
    # the stand-in battery is one.
    print(machine.succeed("grep -H . /sys/class/hwmon/*/name"))
    desktop.answer("heat")
    desktop.photograph("launcher-heat")
    machine.send_key("ret")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10))
    assert re.search("^-?\\d+ \\u00b0C at the ", desktop.as_account("wl-paste --no-newline"))
    # Each part reads name first, a step in; Return opens one, its
    # history first, and so does the right arrow, once back out.
    desktop.answer("heat")
    desktop.launcher_until(lambda shown: shown["kind"] == "answer")
    desktop.step_in()
    desktop.launcher_until(lambda shown: "Battery" in shown["rows"])
    desktop.type_in("battery", "Battery")
    assert re.search("^-?\\d+ \\u00b0C$", desktop.showing()["selected"]["side"])

    def in_battery(shown: Json) -> bool:
        """Say whether the launcher holds the battery's heat, its history selected."""
        return bool(
            shown["held"][-1:] == ["Battery"] and (shown["selected"] or {}).get("name") == "Lately"
        )

    desktop.step_in()
    desktop.launcher_until(in_battery)
    desktop.photograph("launcher-heat-part")
    machine.send_key("left")
    desktop.launcher_until(
        lambda shown: len(shown["held"]) == 1 and (shown["selected"] or {}).get("name") == "Battery"
    )
    # A figure copies what it shows on Return, even one holding more
    # (Joel, 2026-10-26: a row showing a value copies it, everywhere).
    machine.send_key("ret")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10))
    pasted = desktop.as_account("wl-paste --no-newline")
    assert re.search("^-?\\d+ \\u00b0C$", pasted), pasted
    # The date in full, as the machine's clock has it; the day may
    # turn while it's asked, so either will do.
    before = machine.succeed("date '+%A %-d %B %Y'").strip()
    desktop.answer("date")
    desktop.photograph("launcher-date")
    machine.send_key("ret")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10))
    after = machine.succeed("date '+%A %-d %B %Y'").strip()
    assert desktop.as_account("wl-paste --no-newline") in [before, after]
    # The stand-in backlight's share, as its files say.
    now, most = (
        int(machine.succeed(f"cat /sys/class/backlight/stand-in/{f}"))
        for f in ("brightness", "max_brightness")
    )
    # Rounded half up, as JavaScript's Math.round does.
    share = f"{int(now * 100 / most + 0.5)}%"
    desktop.answer("brightness")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == share)
    machine.send_key("esc")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10))
    # With a number, Return sets it, as the brightness keys do.
    desktop.answer("brightness 30")
    desktop.launcher_until(
        lambda shown: (shown["selected"] or {}).get("name") == "set the screen to 30%"
    )
    machine.send_key("ret")
    machine.wait_until_succeeds(
        f"test $(cat /sys/class/backlight/stand-in/brightness) -eq {most * 30 // 100}",
        timeout=dt.timedelta(seconds=10),
    )
    # It changed something, so the launcher stays until it's closed.
    machine.send_key("esc")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10))
    # The room's light, as the stand-in sensor hears it, is asked by its
    # word.
    desktop.answer("room light")
    desktop.launcher_until(lambda shown: str((shown["selected"] or {}).get("name")).endswith(" lx"))
    machine.send_key("esc")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10))
    # The screen the backlight lights holds its Brightness after Return's
    # own Focus, a setting whose levels are a step in by Return, and the
    # Room light beside it.
    desktop.as_account("qs ipc call launcher collection displays")
    desktop.launcher_ready()
    desktop.step_in()
    desktop.launcher_until(lambda shown: shown["rows"][:3] == ["Focus", "Brightness", "Room Light"])
    machine.send_key("down")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Brightness")
    desktop.step_in("ret")
    desktop.launcher_until(lambda shown: "Brightness to 40%" in shown["rows"])
    desktop.type_in("40", selecting="Brightness to 40%")
    machine.send_key("ret")
    machine.wait_until_succeeds(
        f"test $(cat /sys/class/backlight/stand-in/brightness) -eq {most * 40 // 100}",
        timeout=dt.timedelta(seconds=10),
    )
    # It changed something, so the launcher stays until it's closed.
    machine.send_key("esc")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10))
    # "keyboard light" asks it alone, as the stand-in's
    # files say: its levels are the answer's rows, a step in by the right
    # arrow, and Return on one sets it, through logind as the screen's is.
    kbd_light = "/sys/class/leds/stand-in::kbd_backlight/brightness"
    kbd_lit = int(machine.succeed(f"cat {kbd_light}"))
    kbd_said = "off" if kbd_lit == 0 else f"{kbd_lit} of 2"
    kbd_levels = ["Light Off", "Light to 1 of 2", "Light to 2 of 2"]
    desktop.answer("keyboard light")
    desktop.launcher_until(
        lambda shown: (
            shown["kind"] == "answer"
            and (shown["selected"] or {}).get("name") == f"the keyboard's light is {kbd_said}"
        )
    )
    # Return's own Copy leads the answer's step, then the levels.
    desktop.step_in()
    desktop.launcher_until(lambda shown: shown["rows"][:4] == ["Copy", *kbd_levels])
    for _ in range(kbd_levels.index("Light to 2 of 2") + 1):
        machine.send_key("down")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Light to 2 of 2")
    machine.send_key("ret")
    machine.wait_until_succeeds(f"grep -qx 2 {kbd_light}", timeout=dt.timedelta(seconds=10))
    # It changed something, so the launcher stays until it's closed.
    machine.send_key("esc")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10))
    # The power actions answer to their names, and only those; Escape,
    # never Return, since Return would do them.
    for typed, action, verb in [
        ("power off", "Power Off", "Power Off"),
        ("reboot", "Reboot", "Reboot"),
        ("log out", "Log Out", "Log Out"),
    ]:
        desktop.answer(typed)
        # Each wait ends before the loop moves on, so it reads this pass's action.
        desktop.launcher_until(
            lambda shown: (
                (shown["selected"] or {}).get("name") == action  # noqa: B023
                and shown["kind"] == "session"
            )
        )
        # Return asks rather than doing it: it holds the action, its
        # one verb saying what a second Return would do and what it
        # closes.
        machine.send_key("ret")
        desktop.launcher_until(
            lambda shown: (
                shown["held"] == [action]  # noqa: B023
                and re.fullmatch(
                    f"{verb} Now(, Closing (the Window|\\d+ Windows))?",  # noqa: B023
                    (shown["selected"] or {}).get("name", ""),
                )
                is not None
            )
        )
        machine.send_key("esc")
        machine.wait_until_fails(
            desktop.in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10)
        )
    # Asked to be sure, the foot says Log Out's verb whole, its first
    # word alone saying nothing.
    desktop.answer("log out")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Log Out")
    machine.send_key("ret")
    desktop.launcher_until(
        lambda shown: "\u21b5 log out" in (shown["selected"] or {}).get("told", [])
    )
    machine.send_key("esc")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10))
    # A key's action, by the other words its bind lists.
    desktop.answer("fullscreen")
    desktop.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name")
            == "Fill the screen with the window, or put it back"
            and shown["kind"] == "shortcut"
        )
    )
    machine.send_key("esc")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10))
    # Each disk's room, as a disk guard says it, the tightest first.
    status = {
        "version": 1,
        "heartbeat": time.time() + 3600,
        "filesystems": [
            # Less room than /nix, but healthy, so /nix still leads.
            {"path": "/", "band": "healthy", "free": 10 * 2**30, "total": 30 * 2**30},
            {
                "path": "/nix",
                "band": "pressure",
                "free": 20 * 2**30,
                "total": 900 * 2**30,
                "says": "The disk is filling, with 20.0 GiB free.",
            },
        ],
        "frozen": [],
        # What the guard measured filling the disks, biggest first.
        "holders": [
            {
                "path": "/var/lib/containers",
                "bytes": 12 * 2**30,
                "owner": {"kind": "containers", "name": "podman"},
                "clearable": False,
                "why": "podman prunes only images no container uses",
                "says": "Container storage holds 12.0 GiB; podman prunes only images no"
                " container uses.",
            },
            {
                "path": "/nix/var/cache",
                "bytes": 6 * 2**30,
                "owner": {"kind": "cache", "name": "nix"},
                "clearable": True,
                "why": None,
                "says": "The nix cache holds 6.0 GiB.",
            },
        ],
        "holders_at": time.time(),
    }
    machine.succeed(
        f"echo '{json.dumps(status)}' > /run/disk-guard/.status"
        " && mv /run/disk-guard/.status /run/disk-guard/status.json"
    )
    desktop.answer("disk")
    # The guard's own words are read below the rows.
    desktop.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name", "").endswith(" free on /nix")
            and shown["selected"]["reads"] == "The disk is filling, with 20.0 GiB free."
        )
    )
    desktop.photograph("launcher-disks")
    # Its first part, a step in after Return's own Copy, is the space the
    # headline's disk has, and the disk's speed, how busy it is and its
    # latency come after.
    desktop.step_in()
    desktop.launcher_until(lambda shown: shown["rows"][:2] == ["Copy", "Space"])
    machine.send_key("down")
    desktop.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name") == "Space"
            and shown["selected"]["side"] == "20.0 GiB free"
            and {"Speed", "Busy", "Latency"} <= set(shown["rows"])
        )
    )
    # A step further, each disk the guard watches, the headline's first.
    desktop.step_in()
    desktop.launcher_until(
        lambda shown: (
            shown["rows"][:1] == ["/nix"]
            and shown["selected"]["side"] == "20.0 GiB free \u00b7 filling"
        )
    )
    machine.send_key("esc")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10))
    # After Space, what's filling the disks, the biggest beside it, and a
    # step in each holder, saying whether the guard clears any of it or
    # why not, its own sentence below.
    desktop.answer("disk")
    desktop.step_in()
    desktop.launcher_until(
        lambda shown: shown["rows"][:3] == ["Copy", "Space", "What's Filling It"]
    )
    desktop.type_in("filling", selecting="What's Filling It")
    assert desktop.showing()["selected"]["side"] == "Containers \u00b7 12.0 GiB"
    desktop.step_in()
    desktop.launcher_until(
        lambda shown: (
            shown["rows"] == ["Containers", "Nix cache"]
            and shown["selected"]["side"] == "12.0 GiB"
            and str(shown["selected"]["reads"]).startswith("Container storage holds")
        )
    )
    machine.send_key("esc")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10))
    # A part asked for alone, by its own words: how fast the disk
    # reads and writes, each a step in.
    desktop.answer("disk speed")
    desktop.launcher_until(
        lambda shown: (
            shown["kind"] == "answer"
            and re.fullmatch(r"[\d.,]+ (B|kB|MB|GB)/s", (shown["selected"] or {}).get("name", ""))
            is not None
        )
    )
    desktop.step_in()
    desktop.launcher_until(lambda shown: shown["rows"] == ["Copy", "Reading", "Writing"])
    machine.send_key("esc")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10))
    # What the network carries now, in and out, from the kernel's
    # counters, and what the line can and how quickly it answers, from
    # what the shaper last wrote, here a stand-in for the machine's card.
    card = machine.succeed(
        "for n in /sys/class/net/*; do if [ -e $n/device ]; then basename $n; fi; done"
    ).split()[0]
    shaped = [
        {
            "name": card,
            "ifb": "shaper9",
            "up": 11830,
            "down": 40000,
            "state": "climbing",
            "baselines": [["9.9.9.9", 51.1], ["1.1.1.1", 21.1], ["8.8.8.8", 20.6]],
            "follows": None,
        }
    ]
    machine.succeed(
        "systemctl stop shaper.service; mkdir -p /run/shaper"
        f" && echo '{json.dumps(shaped)}' > /run/shaper/status.json"
    )
    rate = r"[\d.,]+ (b|kb|Mb|Gb)/s"
    desktop.answer("speed")
    desktop.launcher_until(
        lambda shown: (
            shown["kind"] == "answer"
            and re.fullmatch(
                f"{rate} in \u00b7 {rate} out", (shown["selected"] or {}).get("name", "")
            )
            is not None
        )
    )
    desktop.step_in()
    # Connections follows only where more than one interface is listed.
    desktop.launcher_until(
        lambda shown: (
            shown["rows"][:6] == ["Copy", "In", "Out", "Connection", "Round Trip", "Dropped"]
        )
    )
    assert (
        desktop.showing()["things"][3]["reads"] == "Connection \u00b7 40 Mb/s in \u00b7 12 Mb/s out"
    )
    machine.send_key("esc")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10))
    # "round trip" asks it alone, the middle of the servers, each a step
    # in, the quickest first.
    desktop.answer("round trip")
    desktop.launcher_until(
        lambda shown: shown["kind"] == "answer" and (shown["selected"] or {}).get("name") == "21 ms"
    )
    desktop.step_in()
    desktop.launcher_until(lambda shown: shown["rows"] == ["Copy", "8.8.8.8", "1.1.1.1", "9.9.9.9"])
    machine.send_key("esc")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10))
    # A guard that stops writing is asked about once its last heartbeat
    # is two minutes old, by a deadline that heartbeat set, with nothing
    # read again in between, and this machine runs no guard's unit.
    status["heartbeat"] = float(machine.succeed("date +%s")) - 100
    machine.succeed(
        f"echo '{json.dumps(status)}' > /run/disk-guard/.status"
        " && mv /run/disk-guard/.status /run/disk-guard/status.json"
    )
    desktop.answer("disk")
    # The band /nix is in, while the guard still writes.
    desktop.launcher_until(lambda shown: shown["footer"] == "filling")
    # Held open, the answer is asked again each second, so it says so as
    # the deadline passes, about twenty seconds on.
    desktop.launcher_until(
        lambda shown: shown["footer"] == "the machine's disk guard stopped; it was turned off",
        timeout=dt.timedelta(seconds=60),
    )
    machine.send_key("esc")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10))
    machine.succeed("rm /run/disk-guard/status.json")
    # The machine's queue as pi's harness writes it, replaced whole:
    # what runs, then what waits in turn, and the room to spare.
    queued_at = machine.succeed("date -u +%Y-%m-%dT%H:%M:%SZ").strip()
    queued = {
        "v": 1,
        "heartbeat": float(machine.succeed("date +%s")),
        "machine": {
            "cores": 4,
            "availableGib": 3,
            "pressure": {"cpu": 12.5, "memory": 0, "io": 0},
        },
        "quiet": None,
        "jobs": [
            {
                "id": "a",
                "class": "bulk",
                "kind": "build",
                "state": "running",
                "owner": {"session": "elsewhere", "pid": 1},
                "what": "nix build .#lighthouse",
                "name": "lighthouse",
                "lastRanS": 1500,
                "memoryGib": 4,
                "since": queued_at,
                # A slice the machine has, so what it holds is read.
                "slice": "/user.slice",
            },
            {
                "id": "b",
                "class": "timed",
                "kind": "vm-test",
                "state": "waiting",
                "owner": {"session": "elsewhere", "pid": 1},
                "what": "nix build .#checks.x86_64-linux.vm-zathura-opens-a-pdf",
                "name": "vm-zathura-opens-a-pdf",
                "memoryGib": 6,
                "waitingOn": "memory",
                "quick": True,
                "since": queued_at,
                "place": 1,
            },
        ],
        "wasted": {"today": 0},
    }
    desktop.as_account(
        f"echo '{json.dumps(queued)}' > $XDG_RUNTIME_DIR/jobs/.q"
        " && mv $XDG_RUNTIME_DIR/jobs/.q $XDG_RUNTIME_DIR/jobs/queue.json"
    )
    # The queue is Agents' now, found by its name, saying how many run and
    # wait, and the room the machine has below.
    # In Agents it says how many run and wait, after Quests.
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("agents", selecting="Agents")
    desktop.step_in("ret")
    desktop.launcher_until(
        lambda shown: (
            shown["held"] == ["agents"]
            and any(
                thing["reads"].startswith("Queue \u00b7 1 running, 1 waiting")
                for thing in shown["things"]
            )
        )
    )
    machine.send_key("esc")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10))
    # Found from the top by name, which counts nothing as it's typed, the
    # room below it says the same.
    desktop.answer("queue")
    desktop.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name") == "Queue"
            and shown["footer"] == "3 GiB to spare \u00b7 the processors 13% held up"
        )
    )
    # Each job by what it is in words, never a check's name: how long it's
    # run against the last time, a minute turning over allowed for, or what
    # it waits for; below it, its kind, the memory it holds or needs, and
    # how it's marked.
    desktop.step_in()
    # Past Return's own act, where the step leads with it.
    desktop.launcher_until(lambda shown: "Lighthouse" in shown["rows"])
    desktop.down_to("Lighthouse")
    for index, (name, side, footer) in enumerate(
        [
            ("Lighthouse", r"[12] of 25 min", r"build \u00b7 holds [\d.]+ GiB"),
            (
                "Zathura Opens a PDF",
                "waits for memory",
                "VM test \u00b7 needs 6 GiB \u00b7 quick",
            ),
        ]
    ):
        if index > 0:
            machine.send_key("down")
        # The wait ends before the loop moves on, so it reads this pass's row.
        desktop.launcher_until(
            lambda shown: (
                (shown["selected"] or {}).get("name") == name  # noqa: B023
                and re.fullmatch(side, shown["selected"]["side"]) is not None  # noqa: B023
                and re.fullmatch(footer, shown["footer"]) is not None  # noqa: B023
            )
        )
    # With no session's window to go to, Return copies its name; a step in
    # says what it runs and the rest the queue knows of it.
    assert desktop.showing()["selected"]["verb"] == "copy", desktop.showing()["selected"]
    desktop.step_in()
    desktop.launcher_until(
        lambda shown: (
            {"Command", "Class", "Needs", "Since"} <= set(shown["rows"])
            and "Holds" not in shown["rows"]
        )
    )
    desktop.photograph("launcher-queue")
    machine.send_key("esc")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10))
    desktop.as_account("rm $XDG_RUNTIME_DIR/jobs/queue.json")
    # How busy the processors are, read again while it's shown, so a
    # moment on it says just now; a step in, each core and the rest.
    desktop.answer("processor")
    desktop.launcher_until(
        lambda shown: (
            re.fullmatch(r"\d+% busy", (shown["selected"] or {}).get("name", "")) is not None
            and shown["footer"] == "just now"
        )
    )
    desktop.step_in()
    desktop.launcher_until(lambda shown: {"Cores", "Maxed", "Load"} <= set(shown["rows"]))
    # Past the answer's own Copy, which leads, to Cores.
    desktop.down_to("Cores")
    desktop.step_in()
    desktop.launcher_until(lambda shown: shown["rows"][0] == "Core 0")
    machine.send_key("esc")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10))
    # The memory, and the programs holding most of it, read once it's
    # asked.
    desktop.answer("memory")
    desktop.launcher_until(lambda shown: "GiB used" in (shown["selected"] or {}).get("name", ""))
    desktop.step_in()
    desktop.launcher_until(lambda shown: {"Used", "Cached", "Using It"} <= set(shown["rows"]))
    machine.send_key("esc")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10))
    # The keyboards QEMU gives it, however many, and caps lock.
    desktop.answer("keyboards")
    desktop.photograph("launcher-keyboards")
    machine.send_key("ret")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10))
    assert re.search(
        "^(one keyboard|[1-9]\\d* keyboards), caps lock off$",
        desktop.as_account("wl-paste --no-newline"),
    )
    # The keyboards are a step into the answer, and each holds its parts,
    # named first, a step further by Return as by the right arrow. QEMU's
    # own is the laptop's kind, with all three lock lights and a PS/2 port.
    # desk-facts doesn't run here, so what it would say of it is written
    # for it, and reads as a part.
    board_facts = {"AT Translated Set 2 keyboard": [{"name": "keys", "value": "105"}]}
    desktop.as_account(
        f"mkdir -p $XDG_RUNTIME_DIR/desk"
        f" && echo '{json.dumps(board_facts)}' > $XDG_RUNTIME_DIR/desk/.k"
        " && mv $XDG_RUNTIME_DIR/desk/.k $XDG_RUNTIME_DIR/desk/keyboards.json"
    )
    board = "The Laptop's Keyboard"
    desktop.answer("keyboards")
    desktop.step_in()
    desktop.launcher_until(lambda shown: board in shown["rows"])
    desktop.down_to(board)
    desktop.step_in("ret")
    desktop.launcher_until(
        lambda shown: (
            shown["rows"][:3] == ["Caps Lock", "Num Lock", "Scroll Lock"]
            and {"Repeat", "Light", "Connection", "Keys"} <= set(shown["rows"])
        ),
    )
    desktop.photograph("launcher-keyboard-parts")
    assert desktop.showing()["selected"]["side"] == "off"
    # Its Light is a setting saying how lit it is, the level set
    # above; Return steps into its levels, and on one, sets it.
    desktop.down_to("Light")
    desktop.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name") == "Light"
            and shown["selected"]["side"] == "2 of 2"
        ),
    )
    desktop.step_in("ret")
    desktop.launcher_until(lambda shown: shown["rows"][:3] == kbd_levels)
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Light Off")
    machine.send_key("ret")
    machine.wait_until_succeeds(f"grep -qx 0 {kbd_light}", timeout=dt.timedelta(seconds=10))
    # It changed something, so the launcher stays until it's closed.
    machine.send_key("esc")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10))
    desktop.answer("keyboards")
    desktop.step_in()
    desktop.launcher_until(lambda shown: board in shown["rows"])
    desktop.down_to(board)
    desktop.step_in("ret")
    desktop.launcher_until(lambda shown: "Repeat" in shown["rows"])
    machine.send_key("left")
    desktop.launcher_until(
        lambda shown: len(shown["held"]) == 1 and (shown["selected"] or {}).get("name") == board,
    )
    desktop.step_in()
    desktop.launcher_until(lambda shown: "Repeat" in shown["rows"])
    desktop.down_to("Repeat")
    desktop.launcher_until(
        lambda shown: (shown["selected"] or {}).get("side", "").startswith("after "),
    )
    machine.send_key("ret")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10))
    assert re.search("^after \\d+ ms, \\d+ Hz$", desktop.as_account("wl-paste --no-newline"))
    desktop.answer("350 f in c")
    desktop.photograph("launcher-units")
    machine.send_key("esc")
    # The zones come from the list built from tzdata, so this proves it's
    # there. The minute may turn while it's asked, so either will do.
    before = machine.succeed("date -u +%H:%M").strip()
    desktop.answer("time in utc")
    desktop.photograph("launcher-time")
    machine.send_key("ret")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10))
    after = machine.succeed("date -u +%H:%M").strip()
    assert desktop.as_account("wl-paste --no-newline") in [before, after]
    # A theme's colour is found by its name, as the theme exports it.
    theme = json.loads(desktop.as_account("cat ~/.config/jitsusama/theme.json"))
    copied("colour accent", theme["colours"]["accent"].lower())
    # Slack takes its theme only by hand, so the launcher copies it,
    # in the theme's own colours.
    slack = ["accent", "selection", "success", "warning"]
    copied("slack theme", ",".join(theme["colours"][colour] for colour in slack))
    desktop.answer("#f53")
    desktop.photograph("launcher-colour")
    machine.send_key("esc")
    desktop.answer("10.0.0.0/22")
    desktop.photograph("launcher-answer")
    machine.send_key("esc")
    # With no network, what has to ask one says it couldn't, and nothing
    # breaks. Each waits for its answer to settle, failed; asked again
    # while that failure stands, it says so at once. When a failure is
    # asked again is the launcher's answers check's to say.
    for typed in ["8.8.8.8", "weather tokyo", "100 usd in cad", "address"]:
        desktop.answer(typed)
        machine.send_key("esc")
    desktop.answer("weather tokyo")
    desktop.photograph("launcher-offline")
    machine.send_key("esc")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10))
    # The rain is the weather's to answer: here asks where here is, since
    # the VM has no time zone, and a place's rain asks the same forecast.
    # Built as a check, the VM has no network and says Open-Meteo didn't
    # answer; run by hand it may reach Open-Meteo itself, and then says the
    # next rain. That the next rain is said from the hours is the
    # launcher's rain check's to show.
    desktop.answer("rain")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "where?")
    machine.send_key("esc")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10))
    desktop.answer("rain in tokyo")
    rained = r"rain (now|from \d\d:00|unlikely), .*|no rain in the next 12 hours"
    desktop.launcher_until(
        lambda shown: (
            (
                (shown["selected"] or {}).get("name") == "no forecast"
                and shown["footer"] == "Open-Meteo didn't answer"
            )
            or (
                re.fullmatch(rained, (shown["selected"] or {}).get("name", "")) is not None
                and shown["footer"] == "Tokyo, Japan \u00b7 Open-Meteo"
            )
        )
    )
    machine.send_key("esc")
    log = desktop.shell_said("ERROR", "Error:")
    assert "ERROR" not in log, "an answer raised an error"
    assert "Error:" not in log, "an answer raised an error"


def test_the_weathers_here_follows_the_laptop(desktop: Desktop) -> None:
    """The weather's here follows the laptop."""
    machine = desktop.machine
    closed = desktop.in_session(LAUNCHER_OPEN)
    fix = "$XDG_RUNTIME_DIR/whereabouts.json"

    def asked(test: Callable[[Json], bool]) -> None:
        desktop.answer("rain")
        desktop.launcher_until(test)
        machine.send_key("esc")
        machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))

    def nowhere(shown: Json) -> bool:
        return (shown["selected"] or {}).get("name") == "where?"

    # whereabouts runs, GeoClue having taken it as a client without a
    # word against it, and with no geolocation service to ask, it has found
    # nothing.
    machine.wait_until_succeeds(desktop.in_session("systemctl --user is-active whereabouts"))
    refused = desktop.as_account("journalctl --user -u whereabouts -p err --no-pager -o cat")
    assert refused.strip() == "", f"whereabouts was refused: {refused}"
    desktop.as_account(f"test ! -e {fix}")
    # The VM sets no time zone and no place, so here is nowhere until
    # whereabouts says where the laptop is. This writes the file in its
    # stead, as it does: whole, under another name, then renamed.
    asked(nowhere)
    said = {
        "v": 2,
        "latitude": 10.123,
        "longitude": -20.456,
        "accuracy": 3000,
        "when": 1,
        "place": None,
        "zone": None,
    }
    desktop.as_account(f"echo '{json.dumps(said)}' > {fix}.new && mv {fix}.new {fix}")
    # Here is the laptop's position now, heard as the file changes,
    # and its forecast is asked by it. Built as a check, the VM has no
    # network, so Open-Meteo didn't answer; run by hand it may, and then
    # the footer says how here was found.
    found = "10.12\u00b0 N 20.46\u00b0 W, from where the laptop is \u00b7 Open-Meteo"
    asked(
        lambda shown: (
            (
                (shown["selected"] or {}).get("name") == "no forecast"
                and shown["footer"] == "Open-Meteo didn't answer"
            )
            or shown["footer"] == found
        )
    )
    # Now's Location says the same position, as the one place it's kept.
    desktop.answer("location")
    desktop.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name") == "Location"
            and (shown["selected"] or {}).get("side") == "10.12, -20.46"
        )
    )
    machine.send_key("esc")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
    # And once the file goes, here is nowhere again, and the location is
    # said to be unknown rather than guessed.
    desktop.as_account(f"rm {fix}")
    asked(nowhere)
    desktop.answer("location")
    desktop.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name") == "Location"
            and (shown["selected"] or {}).get("side") == "unknown"
        )
    )
    machine.send_key("esc")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))


def test_the_clock_follows_the_laptop_into_another_zone_and_is_put_back(desktop: Desktop) -> None:
    """The clock follows the laptop into another zone, says so in Now, and is put back."""
    machine = desktop.machine
    closed = desktop.in_session(LAUNCHER_OPEN)
    fix = "$XDG_RUNTIME_DIR/whereabouts.json"
    zone = "timedatectl show --property=Timezone --value"
    began = machine.succeed(zone).strip()
    ran = "systemctl --user show clock-follows.service"
    ran += " --property=InactiveEnterTimestampMonotonic --value"

    def found(where: str, accuracy: int, place: str) -> None:
        """Write where the laptop is, as whereabouts does, and wait for the clock to read it."""
        before = desktop.as_account(ran).strip()
        said = {"v": 2, "latitude": 51.05, "longitude": -114.07, "accuracy": accuracy,
                "when": int(time.time()), "place": place, "zone": where}  # fmt: skip
        desktop.as_account(f"echo '{json.dumps(said)}' > {fix}.new && mv {fix}.new {fix}")
        machine.wait_until_succeeds(
            desktop.in_session(f'test "$({ran})" != {shlex.quote(before)}'),
            timeout=dt.timedelta(seconds=30),
        )

    def keeps(where: str) -> None:
        """Wait until the clock keeps this zone."""
        machine.wait_until_succeeds(f'test "$({zone})" = {where}', timeout=dt.timedelta(seconds=30))

    try:
        # Found by Wi-Fi, to within 300 m, in another zone: the clock moves,
        # and Now's Location says so.
        found("America/Edmonton", 300, "Lighthouse, Alberta")
        keeps("America/Edmonton")
        desktop.answer("location")
        desktop.launcher_until(
            lambda shown: (shown["selected"] or {}).get("name") == "Location"
            and "clock moved to Edmonton time, from Lighthouse" in json.dumps(shown["selected"])
        )
        desktop.step_in()
        desktop.launcher_until(lambda shown: shown["rows"][:1] == ["Put It Back"])
        # Put back, it's held where it was while the laptop stays there.
        machine.send_key("ret")
        keeps(began)
        machine.send_key("esc")
        machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
        found("America/Edmonton", 300, "Lighthouse, Alberta")
        assert machine.succeed(zone).strip() == began
        # Somewhere else lets it go, and the clock follows again.
        found("America/Vancouver", 300, "Harbour, British Columbia")
        keeps("America/Vancouver")
        # Found only from the network's address, tens of kilometres wide,
        # as through WARP, it never moves the clock.
        found("Asia/Tokyo", 26000, "Shibuya, Tokyo")
        assert machine.succeed(zone).strip() == "America/Vancouver"
    finally:
        machine.succeed(f"timedatectl set-timezone {shlex.quote(began)}")
        desktop.as_account(f"rm -rf {fix} ~/.local/state/clock-follows")


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


def test_the_power_acts_are_found_by_their_names_and_by_the_names_they_had(
    desktop: Desktop,
) -> None:
    """The power acts are found by their names, and by the names they had."""
    machine = desktop.machine
    closed = desktop.in_session(LAUNCHER_OPEN)
    # Short names, each found by the start Joel types; Escape, never
    # Return, since Return would do them.
    for typed, name, verb in [
        ("sle", "Sleep", "sleep"),
        ("keep", "Keep Awake", "start"),
        ("reb", "Reboot", "reboot"),
        ("do not", "Do Not Disturb", "start"),
    ]:
        desktop.answer(typed)
        # The wait ends before the loop moves on, so it reads this pass's name.
        desktop.launcher_until(
            lambda shown: (
                (shown["selected"] or {}).get("name") == name  # noqa: B023
                and shown["selected"]["verb"] == verb  # noqa: B023
            )
        )
        machine.send_key("esc")
        machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
    # A rename loses nothing: the old name still finds Sleep, never shown,
    # and says once what it's called now.
    desktop.answer("suspend")
    desktop.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name") == "Sleep"
            and shown["footer"] == "Once called Suspend."
            and "Suspend" not in shown["rows"]
        )
    )
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
    # Tuple's place stands alone for its rows and its people; the
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
        ("calls", ["Calls"]),
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
    # At rest not every core is busy, so every core saying it is below
    # is each read again.
    cores_resting = desktop.showing()
    assert not all(busy.fullmatch(thing["reads"]) for thing in cores_resting["things"]), (
        "the cores were busy before anything made them so"
    )
    # A loop for each core, as the shell's own builtins, since a
    # transient unit's path has no coreutils.
    spinning = "(while :; do :; done) & " * int(machine.succeed("nproc"))
    machine.succeed(f"systemd-run --unit=busy-cores -p Nice=19 sh -c '{spinning}wait'")
    desktop.launcher_until(
        lambda shown: all(busy.fullmatch(thing["reads"]) for thing in shown["things"]),
        timeout=dt.timedelta(seconds=60),
    )
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


# A binding that throws or assigns the wrong type only warns, and
# only once a surface draws it, which can be long after the start.


# An answer that has to be asked for holds its row from the keystroke, so
# the rows below it never move down when it comes, or when it doesn't.


def test_an_answer_asked_for_holds_its_row_from_the_keystroke(desktop: Desktop) -> None:
    """An answer asked for holds its row from the keystroke."""
    machine = desktop.machine
    typed = "weather lima"
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    since = desktop.mark()
    machine.send_chars(typed)
    seen: list[Json] = []

    def settled(said: str) -> bool:
        shown = json.loads(said)
        if shown["typed"] != typed:
            return False
        seen.append(shown)
        return bool((shown["selected"] or {}).get("settled"))

    desktop.reported("showing ", settled, since)
    # Each time it's shown whole, still asking or answered, the answer is
    # the first row and the rows below it are the same.
    firsts = [shown["things"][0]["is"] if shown["things"] else "" for shown in seen]
    assert set(firsts) == {"answer"}, f"the answer's row came and went: {firsts}"
    below = {tuple(shown["rows"][1:]) for shown in seen}
    assert len(below) == 1, f"the rows below the answer moved: {below}"
    machine.send_key("esc")
    desktop.launcher_until(lambda shown: not shown["open"])


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


def test_quickshell_warned_of_nothing_it_drew_all_along(desktop: Desktop) -> None:
    """Quickshell warned of nothing it drew all along."""
    # Quickshell colours its levels and categories even in the journal,
    # so a colour code may stand between the two words.
    log = desktop.as_account(
        "journalctl --user -u quickshell --no-pager -o cat | grep -a -E 'WARN[^ ]* scene' || true"
    )
    assert log.strip() == "", f"Quickshell warned of a scene: {log}"
