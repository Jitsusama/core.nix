"""The desktop test's launcher answers area.

Here the launcher answers: everything Joel types lists each thing once, its answers are copied,
and emoji keep their own place. tests/desktop.nix runs it after desktop_prelude.py, the boot and
the helpers every area shares, so the names it takes from there are imported only for type
checkers.
"""

import datetime as dt
import json
import re
import time
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Callable

    from desktop_prelude import (
        LAUNCHER_OPEN,
        ROWS_LOOKED_AT,
        STARTING,
        Json,
        answer,
        as_account,
        in_session,
        launcher_ready,
        launcher_until,
        photograph,
        shell_said,
        showing,
        step_in,
        told,
        type_in,
    )
    from driver_names import machine, subtest, t

# Seeing one thing twice costs Joel more than any other mess in the
# list, so for everything he typed in a week of picks, every word of
# the launcher's photo survey and every case its audit found listed
# twice, no two rows may be the same thing, nor read the same. The
# launcher stays open through them all, the left arrow taking away
# what's typed whole between one and the next, as Joel would: opening
# and closing it for each, each a login as the account, took most of
# the time.
with subtest("the launcher lists each thing once, for everything Joel types"):

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

        shown = showing()
        while not holds(shown):
            launcher_until(holds)
            shown = showing()
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
        "bluetooth", "clipboard", "keyboard", "dim", "keyboard backlight", "camera",
        "quest", "terminal", "below", "zsh",
    ]  # fmt: skip
    listed_twice: list[str] = []
    looked_at = 0
    as_account("qs ipc call launcher toggle")
    launcher_ready()
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
            machine.wait_until_fails(in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10))
            as_account("qs ipc call launcher toggle")
            launcher_ready()
        else:
            machine.send_key("left")
    machine.send_key("esc")
    machine.wait_until_fails(in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10))
    t.assertEqual(listed_twice, [], "a thing was listed twice")
    # Most of those find something, so the rows were looked at.
    t.assertGreater(looked_at, ROWS_LOOKED_AT)

with subtest("the launcher answers what's asked, and copies the answer"):

    def copied(typed: str, expected: str) -> None:
        """Ask the launcher, take its answer with Return, and check that it copied this."""
        answer(typed)
        machine.send_key("ret")
        closed = in_session(LAUNCHER_OPEN)
        machine.wait_until_fails(closed, timeout=10)
        t.assertEqual(as_account("wl-paste --no-newline"), expected)

    copied("12*7", "84")
    # The vendor comes from the list built from hwdata, so this proves it's there.
    copied("3c:22:fb:12:34:56", "Apple, Inc.")
    copied("5 km in mi", "3.10686")
    # The battery's charge leads, as the kernel's stand-in has it.
    charge = machine.succeed("cat /sys/class/power_supply/test_battery/capacity").strip()
    answer("battery")
    photograph("launcher-battery")
    machine.send_key("ret")
    machine.wait_until_fails(in_session(LAUNCHER_OPEN), timeout=10)
    t.assertTrue(as_account("wl-paste --no-newline").startswith(f"{charge}%, "))
    # Then what feeds it: the stand-in's mains, plugged in, a step
    # into the answer's row.
    answer("charger")
    launcher_until(lambda shown: shown["kind"] == "answer")
    step_in()
    launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name") == "Charger"
            and shown["selected"]["side"] == "mains"
        )
    )
    # Its charge among them, drawn as a bar, and none of them a glyph.
    photograph("launcher-charger-parts")
    machine.send_key("esc")
    # A collection is found by what a person calls it, not only its
    # title, and Return goes straight into it.
    # Beside the field, it says what sort of thing each is.
    for typed, collection in [("audio", "Sound"), ("notifications", "Notifications")]:
        answer(typed)
        # The wait ends before the loop moves on, so it reads this pass's name.
        launcher_until(
            lambda shown: (
                (shown["selected"] or {}).get("name") == collection  # noqa: B023
                and shown["kind"] == "collection"
            )
        )
        machine.send_key("esc")
    # Half a word asks its question too: "batt" is the battery, and
    # Tab finishes the word.
    answer("batt")
    launcher_until(
        lambda shown: (
            shown["kind"] == "answer"
            and (shown["selected"] or {}).get("name", "").startswith(f"{charge}%")
        )
    )
    machine.send_key("tab")
    launcher_until(lambda shown: shown["typed"] == "battery")
    machine.send_key("ret")
    machine.wait_until_fails(in_session(LAUNCHER_OPEN), timeout=10)
    t.assertTrue(as_account("wl-paste --no-newline").startswith(f"{charge}%, "))
    # The words a poor memory reaches for ask the same answers.
    # "lag" answers from lag-watch's last ten seconds.
    for typed, said in [
        ("battery life", f"{charge}%"),
        ("how hot", ""),
        ("ram", ""),
        ("lag", ""),
    ]:
        answer(typed)
        # The wait ends before the loop moves on, so it reads this pass's answer.
        launcher_until(
            lambda shown: (
                shown["kind"] == "answer"
                and (shown["selected"] or {}).get("name", "").startswith(said)  # noqa: B023
            )
        )
        machine.send_key("esc")
    # A name starting the same way is chosen over an answer only
    # started: "pow" is Power Off, the battery's answer above it.
    answer("pow")
    launcher_until(
        lambda shown: (
            shown["kind"] not in ("", "answer")
            and (shown["selected"] or {}).get("name", "").lower().startswith("pow")
        )
    )
    machine.send_key("esc")
    # The hottest part, from the sensors the kernel lists, of which
    # the stand-in battery is one.
    print(machine.succeed("grep -H . /sys/class/hwmon/*/name"))
    answer("temperature")
    photograph("launcher-heat")
    machine.send_key("ret")
    machine.wait_until_fails(in_session(LAUNCHER_OPEN), timeout=10)
    t.assertRegex(as_account("wl-paste --no-newline"), r"^-?\d+ \u00b0C at the ")
    # Each part reads name first, a step in; Return opens one, its
    # history first, and so does the right arrow, once back out.
    answer("temperature")
    launcher_until(lambda shown: shown["kind"] == "answer")
    step_in()
    launcher_until(lambda shown: "Battery" in shown["rows"])
    type_in("battery", "Battery")
    t.assertRegex(showing()["selected"]["side"], r"^-?\d+ \u00b0C$")

    def in_battery(shown: Json) -> bool:
        """Say whether the launcher holds the battery's heat, its history selected."""
        return bool(
            shown["held"][-1:] == ["Battery"] and (shown["selected"] or {}).get("name") == "Lately"
        )

    step_in("ret")
    launcher_until(in_battery)
    photograph("launcher-heat-part")
    machine.send_key("left")
    launcher_until(
        lambda shown: len(shown["held"]) == 1 and (shown["selected"] or {}).get("name") == "Battery"
    )
    step_in()
    launcher_until(in_battery)
    machine.send_key("esc")
    machine.wait_until_fails(in_session(LAUNCHER_OPEN), timeout=10)
    # The date in full, as the machine's clock has it; the day may
    # turn while it's asked, so either will do.
    before = machine.succeed("date '+%A %-d %B %Y'").strip()
    answer("date")
    photograph("launcher-date")
    machine.send_key("ret")
    machine.wait_until_fails(in_session(LAUNCHER_OPEN), timeout=10)
    after = machine.succeed("date '+%A %-d %B %Y'").strip()
    t.assertIn(as_account("wl-paste --no-newline"), [before, after])
    # The stand-in backlight's share, as its files say.
    now, most = (
        int(machine.succeed(f"cat /sys/class/backlight/stand-in/{f}"))
        for f in ("brightness", "max_brightness")
    )
    # Rounded half up, as JavaScript's Math.round does.
    copied("brightness", f"{int(now * 100 / most + 0.5)}% bright")
    # With a number, Return sets it, as the brightness keys do.
    answer("brightness 30")
    launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "set the screen to 30%")
    machine.send_key("ret")
    machine.wait_until_succeeds(
        f"test $(cat /sys/class/backlight/stand-in/brightness) -eq {most * 30 // 100}",
        timeout=10,
    )
    machine.wait_until_fails(in_session(LAUNCHER_OPEN), timeout=10)
    # The session's actions answer to what people call them; Escape,
    # never Return, since Return would do them.
    for typed, action, verb in [
        ("shut down", "Power off", "Power off"),
        ("restart", "Reboot", "Reboot"),
        ("sign out", "Log out", "Log out"),
    ]:
        answer(typed)
        # Each wait ends before the loop moves on, so it reads this pass's action.
        launcher_until(
            lambda shown: (
                (shown["selected"] or {}).get("name") == action  # noqa: B023
                and shown["kind"] == "session"
            )
        )
        # Return asks rather than doing it: it holds the action, its
        # one verb saying what a second Return would do and what it
        # closes.
        machine.send_key("ret")
        launcher_until(
            lambda shown: (
                shown["held"] == [action]  # noqa: B023
                and re.fullmatch(
                    f"{verb} now(, closing (the window|\\d+ windows))?",  # noqa: B023
                    (shown["selected"] or {}).get("name", ""),
                )
                is not None
            )
        )
        machine.send_key("esc")
        machine.wait_until_fails(in_session(LAUNCHER_OPEN), timeout=10)
    # A key's action, by the other words its bind lists.
    answer("fullscreen")
    launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name")
            == "Fill the screen with the window, or put it back"
            and shown["kind"] == "shortcut"
        )
    )
    machine.send_key("esc")
    machine.wait_until_fails(in_session(LAUNCHER_OPEN), timeout=10)
    # Each disk's room, as a disk guard says it, the tightest first.
    status = {
        "version": 1,
        "heartbeat": time.time() + 3600,
        "filesystems": [
            # Less room than /nix, but healthy, so /nix still leads.
            {"path": "/", "band": "healthy", "free": 10 * 2**30, "total": 30 * 2**30},
            {"path": "/nix", "band": "pressure", "free": 20 * 2**30, "total": 900 * 2**30},
        ],
        "frozen": [],
    }
    machine.succeed(
        f"echo '{json.dumps(status)}' > /run/disk-guard/.status"
        " && mv /run/disk-guard/.status /run/disk-guard/status.json"
    )
    answer("disk")
    launcher_until(
        lambda shown: (shown["selected"] or {}).get("name", "").endswith(" free on /nix")
    )
    photograph("launcher-disks")
    # Its first part, a step in, is the space the headline's disk has,
    # and the disk's speed, how busy it is and its latency come after.
    step_in()
    launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name") == "Space"
            and shown["selected"]["side"] == "20.0 GiB free"
            and {"Speed", "Busy", "Latency"} <= set(shown["rows"])
        )
    )
    # A step further, each disk the guard watches, the headline's first.
    step_in()
    launcher_until(
        lambda shown: (
            shown["rows"][:1] == ["/nix"]
            and shown["selected"]["side"] == "20.0 GiB free \u00b7 pressure"
        )
    )
    machine.send_key("esc")
    machine.wait_until_fails(in_session(LAUNCHER_OPEN), timeout=10)
    # A part asked for alone, by its own words: how fast the disk
    # reads and writes, each a step in.
    answer("disk speed")
    launcher_until(
        lambda shown: (
            shown["kind"] == "answer"
            and re.fullmatch(r"[\d.,]+ (B|kB|MB|GB)/s", (shown["selected"] or {}).get("name", ""))
            is not None
        )
    )
    step_in()
    launcher_until(lambda shown: shown["rows"] == ["Reading", "Writing"])
    machine.send_key("esc")
    machine.wait_until_fails(in_session(LAUNCHER_OPEN), timeout=10)
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
    answer("network speed")
    launcher_until(
        lambda shown: (
            shown["kind"] == "answer"
            and re.fullmatch(
                f"{rate} in \u00b7 {rate} out", (shown["selected"] or {}).get("name", "")
            )
            is not None
        )
    )
    step_in()
    # Connections follows only where more than one interface is listed.
    launcher_until(
        lambda shown: shown["rows"][:5] == ["In", "Out", "Line", "Round trip", "Dropped"]
    )
    t.assertEqual(showing()["things"][2]["reads"], "Line \u00b7 40 Mb/s in \u00b7 12 Mb/s out")
    machine.send_key("esc")
    machine.wait_until_fails(in_session(LAUNCHER_OPEN), timeout=10)
    # "ping" asks the round trip alone, the middle of the servers, each
    # a step in, the quickest first.
    answer("ping")
    launcher_until(
        lambda shown: shown["kind"] == "answer" and (shown["selected"] or {}).get("name") == "21 ms"
    )
    step_in()
    launcher_until(lambda shown: shown["rows"] == ["8.8.8.8", "1.1.1.1", "9.9.9.9"])
    machine.send_key("esc")
    machine.wait_until_fails(in_session(LAUNCHER_OPEN), timeout=10)
    # A guard that stops writing is silent once its last heartbeat is
    # two minutes old, by a deadline that heartbeat set, with nothing
    # read again in between.
    status["heartbeat"] = float(machine.succeed("date +%s")) - 100
    machine.succeed(
        f"echo '{json.dumps(status)}' > /run/disk-guard/.status"
        " && mv /run/disk-guard/.status /run/disk-guard/status.json"
    )
    answer("disk")
    launcher_until(lambda shown: shown["footer"] == "from the disk guard")
    machine.send_key("esc")
    machine.wait_until_fails(in_session(LAUNCHER_OPEN), timeout=10)
    time.sleep(25)
    answer("disk")
    launcher_until(lambda shown: shown["footer"] == "the disk guard has stopped saying")
    machine.send_key("esc")
    machine.wait_until_fails(in_session(LAUNCHER_OPEN), timeout=10)
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
                "what": "Build the desktop",
                "since": queued_at,
                "slice": "jobs.slice",
            },
            {
                "id": "b",
                "class": "timed",
                "kind": "vm-test",
                "state": "waiting",
                "owner": {"session": "elsewhere", "pid": 1},
                "what": "Test the camera",
                "since": queued_at,
                "place": 1,
            },
        ],
        "wasted": {"today": 0},
    }
    as_account(
        f"echo '{json.dumps(queued)}' > $XDG_RUNTIME_DIR/jobs/.q"
        " && mv $XDG_RUNTIME_DIR/jobs/.q $XDG_RUNTIME_DIR/jobs/queue.json"
    )
    answer("what's running")
    launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name") == "1 running, 1 waiting"
            and shown["footer"] == "3 GiB to spare \u00b7 the processors 13% held up"
        )
    )
    # At a glance its place in line or how long it's run, a minute
    # turning over allowed for; since when below it. They're a step
    # into the answer, the first selected there.
    step_in()
    for index, (name, side, footer) in enumerate(
        [
            (
                "Build the desktop",
                r"build \u00b7 (just begun|for 1 min)",
                r"running since \d\d:\d\d",
            ),
            ("Test the camera", r"VM test \u00b7 1st in line", r"waiting since \d\d:\d\d"),
        ]
    ):
        if index > 0:
            machine.send_key("down")
        # The wait ends before the loop moves on, so it reads this pass's row.
        launcher_until(
            lambda shown: (
                (shown["selected"] or {}).get("name") == name  # noqa: B023
                and re.fullmatch(side, shown["selected"]["side"]) is not None  # noqa: B023
                and re.fullmatch(footer, shown["footer"]) is not None  # noqa: B023
            )
        )
    photograph("launcher-queue")
    machine.send_key("esc")
    machine.wait_until_fails(in_session(LAUNCHER_OPEN), timeout=10)
    as_account("rm $XDG_RUNTIME_DIR/jobs/queue.json")
    # How busy the processors are, read again while it's shown, so a
    # moment on it says just now; a step in, each core and the rest.
    answer("cpu")
    launcher_until(
        lambda shown: (
            re.fullmatch(r"\d+% busy", (shown["selected"] or {}).get("name", "")) is not None
            and shown["footer"] == "just now"
        )
    )
    step_in()
    launcher_until(lambda shown: {"Cores", "Maxed", "Load"} <= set(shown["rows"]))
    step_in()
    launcher_until(lambda shown: shown["rows"][0] == "Core 0")
    machine.send_key("esc")
    machine.wait_until_fails(in_session(LAUNCHER_OPEN), timeout=10)
    # The memory, and the programs holding most of it, read once it's
    # asked.
    answer("memory")
    launcher_until(lambda shown: "GiB used" in (shown["selected"] or {}).get("name", ""))
    time.sleep(2)
    step_in()
    launcher_until(lambda shown: {"Used", "Cached", "Using it"} <= set(shown["rows"]))
    machine.send_key("esc")
    machine.wait_until_fails(in_session(LAUNCHER_OPEN), timeout=10)
    # The keyboards QEMU gives it, however many, and caps lock.
    answer("keyboards")
    photograph("launcher-keyboards")
    machine.send_key("ret")
    machine.wait_until_fails(in_session(LAUNCHER_OPEN), timeout=10)
    t.assertRegex(
        as_account("wl-paste --no-newline"),
        r"^(one keyboard|[1-9]\d* keyboards), caps lock off$",
    )
    answer("350 f in c")
    photograph("launcher-units")
    machine.send_key("esc")
    # The zones come from the list built from tzdata, so this proves it's
    # there. The minute may turn while it's asked, so either will do.
    before = machine.succeed("date -u +%H:%M").strip()
    answer("time in utc")
    photograph("launcher-time")
    machine.send_key("ret")
    machine.wait_until_fails(in_session(LAUNCHER_OPEN), timeout=10)
    after = machine.succeed("date -u +%H:%M").strip()
    t.assertIn(as_account("wl-paste --no-newline"), [before, after])
    # A theme's colour is found by its role, as the theme exports it.
    theme = json.loads(as_account("cat ~/.config/jitsusama/theme.json"))
    copied("colour accent", theme["roles"]["accent"].lower())
    # Slack takes its theme only by hand, so the launcher copies it,
    # in the theme's own roles.
    slack = ["accent", "selection", "success", "warning"]
    copied("slack theme", ",".join(theme["roles"][role] for role in slack))
    answer("#f53")
    photograph("launcher-colour")
    machine.send_key("esc")
    answer("10.0.0.0/22")
    photograph("launcher-answer")
    machine.send_key("esc")
    # With no network, what has to ask one says it couldn't, and nothing
    # breaks. Each waits for its answer to settle, failed; asked again
    # while that failure stands, it says so at once. When a failure is
    # asked again is the launcher's answers check's to say.
    for typed in ["8.8.8.8", "weather tokyo", "100 usd in cad", "ip"]:
        answer(typed)
        machine.send_key("esc")
    answer("weather tokyo")
    photograph("launcher-offline")
    machine.send_key("esc")
    machine.wait_until_fails(in_session(LAUNCHER_OPEN), timeout=10)
    # The rain is the weather's to answer: here asks where here is, since
    # the VM has no time zone, and a place's rain asks the same forecast.
    # Built as a check, the VM has no network and says Open-Meteo didn't
    # answer; run by hand it may reach Open-Meteo itself, and then says the
    # next rain. That the next rain is said from the hours is the
    # launcher's rain check's to show.
    answer("rain")
    launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "where?")
    machine.send_key("esc")
    machine.wait_until_fails(in_session(LAUNCHER_OPEN), timeout=10)
    answer("when will it rain in tokyo")
    rained = r"rain (now|from \d\d:00|unlikely), .*|no rain in the next 12 hours"
    launcher_until(
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
    log = shell_said("ERROR", "Error:")
    assert "ERROR" not in log, "an answer raised an error"
    assert "Error:" not in log, "an answer raised an error"

with subtest("the launcher finds emoji in their own place, and only there"):
    closed = in_session(LAUNCHER_OPEN)
    # The main search lists no emoji, only the place they're in.
    answer("tada")
    t.assertNotIn("party popper", showing()["rows"])
    machine.send_key("esc")
    machine.wait_until_fails(closed, timeout=10)
    answer("emoji")
    launcher_until(lambda shown: shown["rows"] == ["Emoji"])
    machine.send_key("ret")
    launcher_until(lambda shown: shown["held"] == ["emoji"])
    type_in("tada", selecting="party popper")
    machine.send_key("ret")
    machine.wait_until_fails(closed, timeout=10)
    t.assertEqual(as_account("wl-paste --no-newline"), "\U0001f389")
    machine.send_key("meta_l-e")
    launcher_ready()
    type_in("smiling")
    photograph("launcher-emoji")
    type_in(" cat")
    photograph("launcher-emoji-all")
    machine.send_key("ret")
    machine.wait_until_fails(closed, timeout=10)
    # The grinning cat with smiling eyes, first among the smiling cats.
    t.assertEqual(as_account("wl-paste --no-newline"), "\U0001f638")
    # Super+E opens the emoji straight away, so "kitty" is a cat there,
    # where the main search would open the terminal.
    machine.send_key("meta_l-e")
    launcher_ready()
    type_in("kitty")
    machine.send_key("ret")
    machine.wait_until_fails(closed, timeout=10)
    t.assertEqual(as_account("wl-paste --no-newline"), "\U0001f431")
    # Over a window, Return pastes the emoji into it as well as copying
    # it, as Super+V's copies do, and a step in are Paste and Copy.
    as_account(
        "niri msg action spawn -- kitty --title emoji-here "
        "sh -c 'stty -icanon -echo; cat > /tmp/emoji-pasted'"
    )
    machine.wait_until_succeeds(
        in_session("niri msg focused-window | grep -q emoji-here"),
        timeout=dt.timedelta(seconds=STARTING),
    )
    machine.send_key("meta_l-e")
    launcher_ready()
    type_in("tada", selecting="party popper")
    launcher_until(lambda shown: (shown["selected"] or {}).get("verb") == "paste")
    step_in()
    launcher_until(lambda shown: shown["rows"] == ["Paste", "Copy"])
    with told("pastes into kitty"):
        machine.send_key("ret")
    machine.wait_until_succeeds(
        in_session("grep -q \U0001f389 /tmp/emoji-pasted"), timeout=dt.timedelta(seconds=10)
    )
    as_account("pkill -f 'cat > /tmp/emoji-pasted'")
    machine.wait_until_fails(
        in_session("niri msg windows | grep -q emoji-here"), timeout=dt.timedelta(seconds=30)
    )

with subtest("a category's word and a space step into it, and a command finishes by Tab"):
    closed = in_session(LAUNCHER_OPEN)
    # The word and a space become a chip, the field cleared, listing
    # what it holds; what's typed then searches only inside it.
    as_account("qs ipc call launcher toggle")
    launcher_ready()
    machine.send_chars("ai ")
    launcher_until(
        lambda shown: (
            shown["held"] == ["ai"]
            and shown["typed"] == ""
            and {"Agents", "Tasks", "Helpers"} <= set(shown["rows"])
        )
    )
    photograph("launcher-category")
    type_in("tas", selecting="Tasks")
    # The left arrow takes what's typed, then steps out with the word
    # put back.
    machine.send_key("left")
    launcher_until(lambda shown: shown["held"] == ["ai"] and shown["typed"] == "")
    machine.send_key("left")
    launcher_until(lambda shown: shown["held"] == [] and shown["typed"] == "ai")
    machine.send_key("esc")
    machine.wait_until_fails(closed, timeout=10)
    # A question that starts with a category's word is still a question.
    answer("time in utc")
    launcher_until(lambda shown: shown["held"] == [] and shown["kind"] == "answer")
    machine.send_key("esc")
    machine.wait_until_fails(closed, timeout=10)
    # Every command, its form shown; Tab puts its start in the field,
    # out at the top, where it's finished.
    as_account("qs ipc call launcher toggle")
    launcher_ready()
    machine.send_chars("commands ")
    launcher_until(
        lambda shown: (
            shown["held"] == ["commands"]
            and {"ip \u27e8address or network\u27e9", "time in \u27e8place\u27e9"}
            <= set(shown["rows"])
        )
    )
    photograph("launcher-commands")
    type_in("ip", selecting="ip \u27e8address or network\u27e9")
    machine.send_key("tab")
    launcher_until(lambda shown: shown["held"] == [] and shown["typed"] == "ip ")
    type_in("10.0.0.0/22")
    launcher_until(lambda shown: shown["kind"] == "answer")
    machine.send_key("esc")
    machine.wait_until_fails(closed, timeout=10)

with subtest("the session's acts are found from anywhere by Joel's own words"):
    closed = in_session(LAUNCHER_OPEN)
    # Short names, each found by the start Joel types, or a word of
    # his; Escape, never Return, since Return would do them.
    for typed, name, verb in [
        ("sus", "Suspend", "suspend"),
        ("caff", "Keep awake", "start"),
        ("reb", "Reboot", "reboot"),
        ("donot", "Do not disturb", "start"),
        # An old name, kept as a hidden word.
        ("suspend the machine", "Suspend", "suspend"),
    ]:
        answer(typed)
        # The wait ends before the loop moves on, so it reads this pass's name.
        launcher_until(
            lambda shown: (
                (shown["selected"] or {}).get("name") == name  # noqa: B023
                and shown["selected"]["verb"] == verb  # noqa: B023
            )
        )
        machine.send_key("esc")
        machine.wait_until_fails(closed, timeout=10)

with subtest("a search lists each thing once"):
    closed = in_session(LAUNCHER_OPEN)
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
        ("tuple", ["Calls"]),
        ("lock", ["Lock"]),
        ("kitty", ["kitty"]),
        ("do not disturb", ["Do not disturb"]),
        ("screenshot", ["Capture"]),
        ("", []),
    ]:
        answer(typed)
        launcher_until(once(leading))
        machine.send_key("esc")
        machine.wait_until_fails(closed, timeout=10)
    answer("zqxj")
    launcher_until(lambda shown: shown["rows"] == [])
    machine.send_key("esc")
    machine.wait_until_fails(closed, timeout=10)
    # The speakers' tuning plays through them, and is no program.
    answer("volume")
    launcher_until(lambda shown: "Turn the volume up" in shown["rows"])
    t.assertNotIn("pebble_x_plus_tuning_output", showing()["rows"])
    machine.send_key("esc")
    machine.wait_until_fails(closed, timeout=10)

# A binding that throws or assigns the wrong type only warns, and
# only once a surface draws it, which can be long after the start.
with subtest("Quickshell warned of nothing it drew all along"):
    # Quickshell colours its levels and categories even in the journal,
    # so a colour code may stand between the two words.
    log = as_account(
        "journalctl --user -u quickshell --no-pager -o cat | grep -a -E 'WARN[^ ]* scene' || true"
    )
    assert log.strip() == "", f"Quickshell warned of a scene: {log}"
