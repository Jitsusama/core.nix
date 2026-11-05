"""The desktop test's launcher answers area.

Here the launcher answers from the machine: the battery, heat, lights, disks, network, queue,
firmware and the processor, and the place and time it follows.

tests/desktop.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and its
opening comment says what the test proves. Every test asks for the desktop, started once a run
(desktop_session.py), and waits at its start for what it needs, so any test runs alone.
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
from desktop_session import KEY_PACE, LAUNCHER_OPEN, charged

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


# The keyboard's stand-in light, and the levels its answer lists.
KBD_LIGHT = "/sys/class/leds/stand-in::kbd_backlight/brightness"
KBD_LEVELS = ["Light Off", "Light to 1 of 2", "Light to 2 of 2"]


def copied(desktop: Desktop, typed: str, expected: str) -> None:
    """Ask the launcher, take its answer with Return, and check that it copied this."""
    desktop.answer(typed)
    # It says so as it closes.
    with desktop.told("parted copied"):
        desktop.machine.send_key("ret")
    closed = desktop.in_session(LAUNCHER_OPEN)
    desktop.machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
    assert desktop.as_account("wl-paste --no-newline") == expected


# In normal mode y copies an answer as Return does, and keeps the launcher
# open; d finds nothing to end in it, and says so in Return's place.


def test_yy_copies_an_answer_and_dd_has_nothing_to_end(desktop: Desktop) -> None:
    """Yy copies an answer with the launcher open, and dd says there's nothing to end."""
    machine = desktop.machine
    # A sum has no rows, only its headline, said in the field's ghost,
    # which flashes in its place, above the rows at -1.
    desktop.answer("12*7")
    desktop.normal()
    since = desktop.mark()
    machine.send_chars("yy")
    desktop.reported("showing ", lambda said: json.loads(said).get("flashing") == [-1], since)
    desktop.reported("yanked 1", lambda rest: rest == "", since)
    assert desktop.as_account("wl-paste --no-newline") == "84"
    # A MAC's answer has its vendor as its first row, which flashes.
    desktop.close_launcher()
    desktop.answer("3c:22:fb:12:34:56")
    desktop.normal()
    since = desktop.mark()
    machine.send_chars("yy")
    desktop.reported("showing ", lambda said: json.loads(said).get("flashing") == [0], since)
    desktop.reported("yanked 1", lambda rest: rest == "", since)
    assert desktop.as_account("wl-paste --no-newline") == "Apple, Inc."
    machine.send_chars("dd")
    desktop.launcher_until(
        lambda shown: shown["open"] and shown["operated"].endswith(" nothing to end")
    )


def test_the_launcher_copies_a_vendor_and_the_charge_and_steps_into_the_charger(
    desktop: Desktop,
) -> None:
    """The launcher copies a MAC's vendor and the charge, and steps into the charger's parts."""
    machine = desktop.machine
    # What a sum or a measure says is tests/answers.qml's to hold; here,
    # only what needs the machine. The vendor comes from the list built
    # from hwdata, so this proves it's there, and that Return copies.
    copied(desktop, "3c:22:fb:12:34:56", "Apple, Inc.")
    # The battery's charge leads, as the kernel's stand-in has it.
    charge = charged(desktop)
    desktop.answer("battery")
    desktop.photograph("launcher-battery")
    machine.send_key("ret")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10))
    assert desktop.as_account("wl-paste --no-newline").startswith(f"{charge}%, ")
    # Then what feeds it: the stand-in's charger, plugged in, a step
    # into the answer's row.
    desktop.answer("charger")
    desktop.launcher_until(lambda shown: shown["kind"] == "answer")
    # The copy is listed last, after the facts, since nothing else inside
    # does anything (Joel, 2026-10-28: never a copy at the top).
    desktop.step_in()
    desktop.launcher_until(
        lambda shown: shown["rows"][:1] == ["Charger"] and shown["rows"][-1:] == ["Copy"]
    )
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
    desktop.close_launcher()
    # A part is asked by its own words, its figure the headline.
    desktop.answer("battery health")
    desktop.launcher_until(
        lambda shown: (
            shown["kind"] == "answer"
            and (shown["selected"] or {}).get("name", "").startswith("Good, ")
        )
    )
    desktop.close_launcher()


def test_heat_copies_the_hottest_part_and_each_part_steps_into_its_history(
    desktop: Desktop,
) -> None:
    """Heat copies the hottest part, and each part steps into its history and copies it."""
    machine = desktop.machine
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
    # A figure holding more steps in on Return, as the right arrow does,
    # and copies what it shows from the Copy listed last inside (Joel,
    # 2026-10-28: "hitting enter on the battery closes the window when
    # right opens it").
    machine.send_key("ret")
    desktop.launcher_until(lambda shown: in_battery(shown) and shown["rows"][-1:] == ["Copy"])
    desktop.down_to("Copy")
    machine.send_key("ret")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10))
    pasted = desktop.as_account("wl-paste --no-newline")
    assert re.search("^-?\\d+ \\u00b0C$", pasted), pasted


def test_the_lights_are_asked_and_set_and_the_keyboards_hold_their_parts(  # noqa: PLR0915
    desktop: Desktop,
) -> None:
    """The screen's and keyboard's lights are asked and set, and keyboards hold their parts."""
    machine = desktop.machine
    # The stand-in backlight's share, as its files say.
    now, most = (
        int(machine.succeed(f"cat /sys/class/backlight/stand-in/{f}"))
        for f in ("brightness", "max_brightness")
    )
    # Rounded half up, as JavaScript's Math.round does.
    share = f"{int(now * 100 / most + 0.5)}%"
    desktop.answer("brightness")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == share)
    desktop.close_launcher()
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
    desktop.close_launcher()
    # The room's light, as the stand-in sensor hears it, is asked by its
    # word.
    desktop.answer("room light")
    desktop.launcher_until(lambda shown: str((shown["selected"] or {}).get("name")).endswith(" lx"))
    desktop.close_launcher()
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
    desktop.close_launcher()
    # "keyboard light" asks it alone, as the stand-in's
    # files say: its levels are the answer's rows, a step in by the right
    # arrow, and Return on one sets it, through logind as the screen's is.
    kbd_lit = int(machine.succeed(f"cat {KBD_LIGHT}"))
    kbd_said = "off" if kbd_lit == 0 else f"{kbd_lit} of 2"
    desktop.answer("keyboard light")
    desktop.launcher_until(
        lambda shown: (
            shown["kind"] == "answer"
            and (shown["selected"] or {}).get("name") == f"the keyboard's light is {kbd_said}"
        )
    )
    # The levels lead the answer's step, then the copy, after what can be
    # done and before what's said.
    desktop.step_in()
    desktop.launcher_until(lambda shown: shown["rows"][:4] == [*KBD_LEVELS, "Copy"])
    for _ in range(KBD_LEVELS.index("Light to 2 of 2")):
        machine.send_key("down")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Light to 2 of 2")
    machine.send_key("ret")
    machine.wait_until_succeeds(f"grep -qx 2 {KBD_LIGHT}", timeout=dt.timedelta(seconds=10))
    # It changed something, so the launcher stays until it's closed.
    desktop.close_launcher()
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
    # A path this long leaves no room for the field's words for its keys,
    # so a letter typed in insert, then normal, brings the left arrow's key
    # alone, and the path is never cut to make room for them.
    desktop.insert()
    machine.send_chars("l")
    desktop.launcher_until(lambda shown: shown["typed"] == "l")
    desktop.normal()
    field = json.loads(desktop.as_account("qs ipc call launcher laidOut"))["field"]
    assert not any(word["cut"] for word in field if board in word["text"]), field
    assert any("\u2190" in word["text"] for word in field), field
    assert not any(word["text"].endswith(" clear") for word in field), field
    machine.send_key("left")
    desktop.launcher_until(lambda shown: shown["typed"] == "")
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
    desktop.launcher_until(lambda shown: shown["rows"][:3] == KBD_LEVELS)
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Light Off")
    machine.send_key("ret")
    machine.wait_until_succeeds(f"grep -qx 0 {KBD_LIGHT}", timeout=dt.timedelta(seconds=10))
    # It changed something, so the launcher stays until it's closed.
    desktop.close_launcher()
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


def test_the_disks_and_the_network_say_their_room_speed_and_round_trip(
    desktop: Desktop,
) -> None:
    """The disks and the network say their room, their speed and their round trip."""
    machine = desktop.machine
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
    desktop.launcher_until(
        lambda shown: (shown["selected"] or {}).get("name", "").endswith(" free on /nix")
    )
    desktop.photograph("launcher-disks")
    # Its first part, a step in after Return's own Copy, is the space the
    # headline's disk has, and the disk's speed, how busy it is and its
    # latency come after.
    desktop.step_in()
    desktop.launcher_until(lambda shown: shown["rows"][:1] == ["Space"])
    desktop.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name") == "Space"
            and shown["selected"]["side"] == "20.0 GiB free"
            and {"Speed", "Busy", "Latency"} <= set(shown["rows"])
        )
    )
    # A step further, each disk the guard watches, the headline's first.
    # A disk is a thing, so the drawer is out for it: one that's short
    # says beside it what the guard is doing, and in the drawer its
    # figures and the guard's own words (docs/design.md, the drawer).
    desktop.step_in()
    desktop.launcher_until(
        lambda shown: (
            shown["rows"][:1] == ["/nix"]
            and shown["selected"]["side"] == "20.0 GiB free \u00b7 clearing old builds"
            and (shown.get("drawer") or {}).get("out") is True
            and shown["drawer"]["facts"] == ["Free", "Size"]
            and shown["drawer"]["body"] == "The disk is filling, with 20.0 GiB free."
        )
    )
    desktop.close_launcher()
    # After Space, what's filling the disks, the biggest beside it, and a
    # step in each holder, saying whether the guard clears any of it or
    # why not, its own sentence below.
    desktop.answer("disk")
    desktop.step_in()
    desktop.launcher_until(lambda shown: shown["rows"][:2] == ["Space", "What's Filling It"])
    desktop.type_in("filling", selecting="What's Filling It")
    assert desktop.showing()["selected"]["side"] == "Containers \u00b7 12.0 GiB"
    desktop.step_in()
    desktop.launcher_until(
        lambda shown: (
            shown["rows"] == ["Containers", "Nix cache", "Copy"]
            and shown["selected"]["side"] == "12.0 GiB"
            and str(shown["selected"]["reads"]).startswith("Container storage holds")
        )
    )
    desktop.close_launcher()
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
    desktop.launcher_until(lambda shown: shown["rows"] == ["Reading", "Writing", "Copy"])
    desktop.close_launcher()
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
        lambda shown: shown["rows"][:5] == ["In", "Out", "Connection", "Round Trip", "Dropped"]
    )
    assert (
        desktop.showing()["things"][2]["reads"] == "Connection \u00b7 40 Mb/s in \u00b7 12 Mb/s out"
    )
    desktop.close_launcher()
    # "round trip" asks it alone, the middle of the servers, each a step
    # in, the quickest first.
    desktop.answer("round trip")
    desktop.launcher_until(
        lambda shown: shown["kind"] == "answer" and (shown["selected"] or {}).get("name") == "21 ms"
    )
    desktop.step_in()
    desktop.launcher_until(lambda shown: shown["rows"] == ["8.8.8.8", "1.1.1.1", "9.9.9.9", "Copy"])
    desktop.close_launcher()
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
    desktop.close_launcher()
    machine.succeed("rm /run/disk-guard/status.json")


def test_the_queue_the_processors_and_the_memory_say_what_holds_them(desktop: Desktop) -> None:
    """The queue, the processors and the memory say what runs and what holds them."""
    machine = desktop.machine
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
    desktop.close_launcher()
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
    desktop.close_launcher()
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
    desktop.close_launcher()
    # The memory, and the programs holding most of it, read once it's
    # asked.
    desktop.answer("memory")
    desktop.launcher_until(lambda shown: "GiB used" in (shown["selected"] or {}).get("name", ""))
    desktop.step_in()
    desktop.launcher_until(lambda shown: {"Used", "Cached", "Using It"} <= set(shown["rows"]))
    desktop.close_launcher()


def guard_says(desktop: Desktop, holders: list[Json]) -> None:
    """Write what the account's guard says, holding these, as it writes it, whole."""
    status = desktop.as_account("echo $XDG_RUNTIME_DIR").strip() + "/disk-guard/status.json"
    whole = {
        "version": 1,
        "heartbeat": time.time() + 3600,
        "filesystems": [
            {"path": "/", "band": "pressure", "free": 5 * 2**30, "total": 30 * 2**30},
        ],
        "frozen": [],
        "holders": holders,
        "holders_at": time.time(),
    }
    desktop.as_account(
        f"mkdir -p $(dirname {status}) && echo '{json.dumps(whole)}' > {status}.new"
        f" && mv {status}.new {status}"
    )


def a_scratch_folder(desktop: Desktop) -> str:
    """Make ~/scratch, and have the guard say it holds 3 GiB, 1 GiB of it clearable now."""
    folder = desktop.as_account("mkdir -p ~/scratch && realpath ~/scratch").strip()
    guard_says(
        desktop,
        [
            {
                "path": folder,
                "name": "~/scratch",
                "bytes": 3 * 2**30,
                "clears": 2**30,
                "grew": {"bytes": 2 * 2**30, "over": 20 * 3600},
                "owner": {"kind": "other", "name": "scratch"},
                "clearable": True,
                "why": None,
                "kept": None,
                "says": "~/scratch holds 3.0 GiB, of which 1.0 GiB can be cleared now.",
            }
        ],
    )
    return folder


def into_whats_filling_it(desktop: Desktop) -> None:
    """Open Disk's What's Filling It, with the scratch folder alone in it."""
    desktop.answer("disk")
    desktop.step_in()
    desktop.type_in("filling", selecting="What's Filling It")
    desktop.step_in()
    # The folder, then the figure's own Copy, last as every step keeps it.
    desktop.launcher_until(lambda shown: shown["rows"] == ["~/scratch", "Copy"])


def test_a_folder_filling_the_disk_is_kept_from_the_guard_for_the_reason_joel_gives(
    desktop: Desktop,
) -> None:
    """A folder filling the disk is kept from the guard's clearing, for the reason Joel gives."""
    machine = desktop.machine
    folder = a_scratch_folder(desktop)

    def into_it() -> None:
        """Step into the folder."""
        into_whats_filling_it(desktop)
        desktop.step_in("ret")

    try:
        # Return steps into it: what can be done, then what it reads, how
        # much it grew among them, then Copy the Path.
        into_it()
        desktop.launcher_until(
            lambda shown: (
                shown["rows"]
                == ["Open Its Folder", "Keep", "Clear", "Size", "Grew", "Clearable",
                    "Copy the Path"]
            )
        )  # fmt: skip
        reads = [thing["reads"] for thing in desktop.showing()["things"]]
        assert "Grew \u00b7 2.0 GiB in 20 h" in reads, reads
        desktop.down_to("Keep")
        assert desktop.showing()["selected"]["side"] == "no"
        # Kept, it asks why, and the guard says so.
        machine.send_key("ret")
        desktop.launcher_until(lambda shown: shown["asking"])
        machine.send_chars("the thesis data")
        machine.send_key("ret")
        desktop.launcher_until(lambda shown: shown["footer"] == f"Kept {folder}: the thesis data.")
        desktop.close_launcher()
        # It stays kept, saying why, and Return lets it go.
        into_it()
        desktop.launcher_until(lambda shown: "Keep" in shown["rows"])
        desktop.down_to("Keep")
        desktop.launcher_until(
            lambda shown: (
                shown["selected"]["side"] == "yes, the thesis data"
                and shown["selected"]["verb"] == "release"
            )
        )
        machine.send_key("ret")
        desktop.launcher_until(lambda shown: shown["footer"] == f"No longer kept: {folder}.")
        desktop.close_launcher()
    finally:
        guard_says(desktop, [])
        desktop.as_account("rm -rf ~/scratch ~/.local/state/disk-guard")


def test_a_folder_filling_the_disk_is_cleared_once_joel_is_sure(desktop: Desktop) -> None:
    """A folder filling the disk is cleared of what the guard takes now, asked once."""
    machine = desktop.machine
    folder = a_scratch_folder(desktop)
    try:
        # Clear says what it takes, and asks once, since it can't be taken
        # back; the guard then says it starts, and the folder says it's
        # clearing, with nothing left to clear while it does.
        into_whats_filling_it(desktop)
        desktop.step_in("ret")
        desktop.launcher_until(lambda shown: "Clear" in shown["rows"])
        desktop.down_to("Clear")
        assert desktop.showing()["selected"]["side"] == "1.0 GiB", desktop.showing()["selected"]
        desktop.step_in("ret")
        desktop.launcher_until(lambda shown: shown["rows"] == ["Clear 1.0 GiB of ~/scratch"])
        machine.send_key("ret")
        desktop.launcher_until(
            lambda shown: (
                shown["footer"] == f"Clearing {folder}; the guard starts on its next step."
            )
        )
        desktop.close_launcher()
        into_whats_filling_it(desktop)
        desktop.launcher_until(lambda shown: shown["footer"] == "the guard is clearing it now")
        desktop.step_in("ret")
        desktop.launcher_until(lambda shown: "Copy the Path" in shown["rows"])
        assert "Clear" not in desktop.showing()["rows"], desktop.showing()["rows"]
        desktop.close_launcher()
    finally:
        guard_says(desktop, [])
        desktop.as_account("rm -rf ~/scratch ~/.local/state/disk-guard")


def test_the_weathers_here_follows_the_laptop(desktop: Desktop) -> None:
    """The weather's here follows the laptop."""
    machine = desktop.machine
    fix = "$XDG_RUNTIME_DIR/whereabouts.json"

    def asked(test: Callable[[Json], bool]) -> None:
        desktop.answer("rain")
        desktop.launcher_until(test)
        desktop.close_launcher()

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
                (shown["selected"] or {}).get("name") == "Open-Meteo didn't answer."
                and shown["footer"] == ""
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
    desktop.close_launcher()
    # And once the file says nowhere, here is nowhere again, and the
    # location is said to be unknown rather than guessed. It's never
    # removed, since the shell can't see a removed file come back.
    desktop.as_account(f"echo '{{}}' > {fix}.new && mv {fix}.new {fix}")
    asked(nowhere)
    desktop.answer("location")
    desktop.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name") == "Location"
            and (shown["selected"] or {}).get("side") == "unknown"
        )
    )
    desktop.close_launcher()


def test_the_clock_follows_the_laptop_into_another_zone_and_is_put_back(desktop: Desktop) -> None:
    """The clock follows the laptop into another zone, says so in Time, and is put back."""
    machine = desktop.machine
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
        # and the time here says so, the zone being the clock's.
        found("America/Edmonton", 300, "Lighthouse, Alberta")
        keeps("America/Edmonton")
        desktop.answer("time")
        desktop.launcher_until(
            lambda shown: (
                shown["footer"] == "clock moved to Edmonton time, as the laptop is in Lighthouse"
            )
        )
        desktop.step_in()
        # Put It Back leads what can be done; Copy, which isn't an act on
        # the clock, may stand before or after it.
        desktop.launcher_until(
            lambda shown: [row for row in shown["rows"] if row != "Copy"][:1] == ["Put It Back"]
        )
        desktop.down_to("Put It Back")
        assert desktop.showing()["selected"]["side"] == began.split("/")[-1].replace("_", " ")
        # Put back, it's held where it was while the laptop stays there.
        machine.send_key("ret")
        keeps(began)
        desktop.close_launcher()
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
        desktop.as_account(f"echo '{{}}' > {fix}.new && mv {fix}.new {fix}")
        desktop.as_account("rm -rf ~/.local/state/clock-follows")


def test_a_place_set_by_hand_holds_until_the_laptop_is_found_again(desktop: Desktop) -> None:
    """A place set by hand holds over GeoClue until the laptop is found again."""
    machine = desktop.machine
    fix = "$XDG_RUNTIME_DIR/whereabouts.json"
    pin = "~/.local/state/whereabouts/pin.json"
    began = machine.succeed("timedatectl show --property=Timezone --value").strip()

    def location(test: Callable[[Json], bool]) -> None:
        """Open Location from the top and wait until its row passes the test."""
        desktop.answer("location")
        desktop.launcher_until(
            lambda shown: (
                (shown["selected"] or {}).get("name") == "Location" and test(shown["selected"])
            )
        )

    def close() -> None:
        desktop.close_launcher()

    try:
        # A town no place is called sets nothing, and the foot says why:
        # built as a check, the VM has no network to ask; run by hand it
        # may reach Open-Meteo, which finds no such place.
        nowhere = "Qzxv Fjordholm"
        location(lambda _: True)
        desktop.step_in()
        desktop.down_to("Set It by Hand")
        machine.send_key("ret")
        desktop.launcher_until(lambda shown: shown["asking"])
        machine.send_chars(nowhere)
        desktop.launcher_until(lambda shown: shown["typed"] == nowhere)
        machine.send_key("ret")
        desktop.launcher_until(
            lambda shown: (
                shown["footer"]
                in {f"couldn't ask where {nowhere} is", f"no place is called {nowhere}"}
            ),
            timeout=dt.timedelta(seconds=30),
        )
        desktop.as_account(f"test ! -e {pin}")
        close()
        # A pin, as setting it writes one, is here from then on, whatever
        # GeoClue says, and the clock follows it as it would any fix.
        said = {"v": 2, "latitude": 51.05, "longitude": -114.07, "accuracy": 1000,
                "when": int(time.time()), "place": "Lighthouse, Alberta",
                "zone": "America/Edmonton", "hand": True}  # fmt: skip
        desktop.as_account(
            f"mkdir -p ~/.local/state/whereabouts && echo '{json.dumps(said)}' > {pin}"
        )
        desktop.as_account("systemctl --user restart whereabouts.service")
        machine.wait_until_succeeds(
            desktop.in_session(f"grep -q '\"hand\":true' {fix}"), timeout=dt.timedelta(seconds=30)
        )
        machine.wait_until_succeeds(
            "test $(timedatectl show --property=Timezone --value) = America/Edmonton",
            timeout=dt.timedelta(seconds=30),
        )
        location(lambda row: row["side"] == "Lighthouse, Alberta")
        desktop.step_in()
        # Inside, in the design's order, saying it was set by hand.
        desktop.launcher_until(
            lambda shown: (
                shown["rows"] == ["Town", "How It Knows", "Set It by Hand", "Find It Again", "Copy"]
            )
        )
        reads = [thing["reads"] for thing in desktop.showing()["things"]]
        assert "How It Knows \u00b7 set by hand" in reads, reads
        # Found again, the pin goes, and here is what GeoClue says, which
        # in the VM is nothing yet.
        desktop.down_to("Find It Again")
        machine.send_key("ret")
        machine.wait_until_succeeds(
            desktop.in_session(f"test ! -e {pin} && test \"$(cat {fix})\" = '{{}}'"),
            timeout=dt.timedelta(seconds=30),
        )
        close()
        location(lambda row: row["side"] == "unknown")
        close()
    finally:
        machine.succeed(f"timedatectl set-timezone {shlex.quote(began)}")
        desktop.as_account(f"echo '{{}}' > {fix}.new && mv {fix}.new {fix}")
        desktop.as_account(f"rm -rf {pin} ~/.local/state/clock-follows")


def kept_by_the_stand_in(desktop: Desktop) -> None:
    """Wait until chrony keeps the clock by the VM's own time server (tests/desktop.nix)."""
    desktop.machine.wait_until_succeeds(
        "chronyc -c -N sources | grep -q '^^,[*],time.harbour.test,'",
        timeout=dt.timedelta(seconds=90),
    )


def time_inside(desktop: Desktop) -> None:
    """Ask the time here and step in, once its Clock and Servers are there."""
    keeping = ["Clock", "Servers"]

    def kept_rows(shown: Json) -> bool:
        rows = cast("list[str]", shown["rows"])
        return "Clock" in rows and rows[rows.index("Clock") :][:2] == keeping

    desktop.answer("time")
    desktop.step_in()
    desktop.launcher_until(kept_rows)


def date_steps_into_its_calendars(desktop: Desktop) -> None:
    """Step Date into today in other calendars, once without the reader and once with it."""
    # With the reader gone, Calendars says it isn't answering, and Return closes rather than
    # copying those words. It's asked first, since a day once answered is kept.
    try:
        desktop.as_account("systemctl --user stop timekeeping.socket timekeeping.service")
        desktop.answer("date")
        desktop.step_in()
        desktop.launcher_until(
            lambda shown: any(
                str(thing["reads"]).startswith("Calendars \u00b7 not answering")
                for thing in shown["things"]
            )
        )
        desktop.down_to("Calendars")
        assert desktop.showing()["selected"]["verb"] == "close", desktop.showing()["selected"]
        desktop.close_launcher()
    finally:
        desktop.as_account("systemctl --user start timekeeping.socket")
    # Asked again, today in the Chinese, Japanese, Hebrew and Islamic calendars, each a
    # thing whose plate is titled with its date in its own script.
    desktop.answer("date")
    desktop.step_in()
    desktop.launcher_until(
        lambda shown: any(
            str(thing["reads"]).startswith("Calendars \u00b7 4") for thing in shown["things"]
        )
    )
    desktop.down_to("Calendars")
    desktop.step_in()
    calendars = ["Chinese", "Japanese", "Hebrew", "Islamic"]
    desktop.launcher_until(lambda shown: shown["rows"][: len(calendars)] == calendars)
    desktop.down_to("Hebrew")
    desktop.launcher_until(
        lambda shown: (
            shown["drawer"]["out"] is True
            and any("\u0590" <= letter <= "\u05ff" for letter in shown["drawer"]["title"])
        )
    )
    desktop.close_launcher()


def test_the_time_here_says_how_the_clock_is_kept(desktop: Desktop) -> None:
    """The time here says how chrony keeps the clock, each server a thing to dig into.

    The same reader reckons the date in other calendars, which Date steps into.
    """
    date_steps_into_its_calendars(desktop)
    kept_by_the_stand_in(desktop)
    time_inside(desktop)
    reads = [str(thing["reads"]) for thing in desktop.showing()["things"]]
    for read in ("Clock \u00b7 synchronised", "Servers \u00b7 2"):
        assert any(it.startswith(read) for it in reads), (read, reads)
    # The clock steps into its figures, in order, each read from chronyd.
    clock = [
        "Offset", "Frequency", "Skew", "Root Delay", "Root Dispersion", "Leap Second",
        "Last Sync", "Next Poll",
    ]  # fmt: skip
    desktop.down_to("Clock")
    desktop.step_in()
    desktop.launcher_until(lambda shown: shown["rows"][: len(clock)] == clock)
    reads = [str(thing["reads"]) for thing in desktop.showing()["things"]]
    for read in ("Leap Second \u00b7 none announced", "Service \u00b7 chrony 4."):
        assert any(it.startswith(read) for it in reads), (read, reads)
    desktop.close_launcher()
    # Servers steps into each server chrony asks, the chosen one first, each a thing whose
    # note says where it stands, its drawer and its inside every figure chronyd has of it.
    time_inside(desktop)
    desktop.down_to("Servers")
    desktop.step_in()
    servers = ["time.harbour.test", "far.harbour.test"]
    desktop.launcher_until(lambda shown: shown["rows"][: len(servers)] == servers)
    desktop.down_to("time.harbour.test")
    assert desktop.showing()["selected"]["note"] == "chosen", desktop.showing()["selected"]
    inside = ["Address", "Why", "Stratum", "Poll Interval", "Reach", "Last Sample", "Offset"]
    desktop.launcher_until(
        lambda shown: (
            shown["drawer"]["out"] is True
            and shown["drawer"]["title"] == "time.harbour.test"
            and shown["drawer"]["facts"][: len(inside)] == inside
        )
    )
    desktop.down_to("far.harbour.test")
    assert desktop.showing()["selected"]["note"] == "out of reach", desktop.showing()["selected"]
    desktop.down_to("time.harbour.test")
    desktop.step_in()
    desktop.launcher_until(lambda shown: shown["rows"][: len(inside)] == inside)
    reads = [str(thing["reads"]) for thing in desktop.showing()["things"]]
    for read in (
        "Address \u00b7 127.0.0.1",
        "Why \u00b7 the clock is kept by it",
        "Stratum \u00b7 2",
        "NTP Version \u00b7 4",
    ):
        assert any(it.startswith(read) for it in reads), (read, reads)
    desktop.close_launcher()
    # With chronyd stopped, nothing keeps the time, and Time says so and no more.
    try:
        desktop.machine.succeed("systemctl stop chronyd")
        desktop.answer("time")
        desktop.step_in()
        desktop.launcher_until(
            lambda shown: any(
                str(thing["reads"]).startswith("Clock \u00b7 nothing keeps the time")
                for thing in shown["things"]
            )
        )
        assert "Servers" not in desktop.showing()["rows"], desktop.showing()["rows"]
        desktop.down_to("Clock")
        assert desktop.showing()["selected"]["verb"] == "close", desktop.showing()["selected"]
        desktop.close_launcher()
    finally:
        desktop.machine.succeed("systemctl start chronyd")


def firmware_reads(shown: Json) -> str:
    """Give what Firmware's row reads where the launcher lists it, or nothing."""
    reads = [str(thing["reads"]) for thing in shown["things"]]
    return next((read for read in reads if read.startswith("Firmware \u00b7")), "")


def last_place(shown: Json) -> str:
    """Give the last row the launcher lists that isn't a command, or nothing."""
    places = [str(thing["is"]) for thing in shown["things"]]
    held = [place for place in places if not place.startswith("command:")]
    return held[-1] if held else ""


def test_firmware_says_whats_offered_and_holds_each_devices_version(desktop: Desktop) -> None:
    """Firmware says whether an update is offered, last in Machine, and each device's version."""
    machine = desktop.machine
    said = "$XDG_RUNTIME_DIR/fwupd"
    # With nothing offered, as the laptop's fwupd says, it's up to date,
    # Machine's last place, after Lag where Lag is shown (only once
    # something has lagged), and before its commands.
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    machine.send_chars("machine ", delay=KEY_PACE)
    desktop.launcher_until(
        lambda shown: (
            last_place(shown) == "answer:firmware"
            and firmware_reads(shown).startswith("Firmware \u00b7 up to date")
        )
    )
    desktop.close_launcher()
    # LVFS offers the BIOS an update: Firmware says so, and the BIOS is
    # listed first, its version and what's offered a step in and in its
    # drawer.
    offered = {
        "Devices": [
            {
                "DeviceId": "bios",
                "Releases": [
                    {"Version": "1.13.0", "Description": "<p>Fixes the keyboard's light.</p>"}
                ],
            }
        ]
    }
    # Quoted whole, since a release's words may hold an apostrophe.
    written = shlex.quote(json.dumps(offered))
    desktop.as_account(f"mkdir -p {said} && echo {written} > {said}/get-updates.json")
    try:
        desktop.as_account("qs ipc call launcher toggle")
        desktop.launcher_ready()
        machine.send_chars("machine ", delay=KEY_PACE)
        desktop.launcher_until(
            lambda shown: firmware_reads(shown).startswith("Firmware \u00b7 1 update ready")
        )
        desktop.type_in("firm", "Firmware")
        desktop.photograph("launcher-firmware-in-machine")
        desktop.step_in()
        # A device is a thing, so the drawer is out for the BIOS: who makes
        # it, its version, what's offered and the release's own words.
        desktop.launcher_until(
            lambda shown: (
                shown["rows"][:2] == ["System Firmware", "PVC10 SK hynix 1024GB"]
                and (shown.get("drawer") or {}).get("out") is True
                and shown["drawer"]["facts"] == ["Vendor", "Version", "Offered"]
                and "Fixes the keyboard's light." in str(shown["drawer"]["body"])
            )
        )
        reads = [str(thing["reads"]) for thing in desktop.showing()["things"]]
        assert reads[0].startswith("System Firmware \u00b7 1.12.0"), reads
        assert reads[1].startswith("PVC10 SK hynix 1024GB \u00b7 61001171"), reads
        desktop.photograph("launcher-firmware")
        desktop.step_in()
        desktop.launcher_until(lambda shown: shown["rows"] == ["Install 1.13.0", "Version"])
        desktop.close_launcher()
    finally:
        desktop.as_account(f"rm -rf {said}")


def test_firmware_installs_an_update_once_asked_and_waits_for_the_restart(
    desktop: Desktop,
) -> None:
    """Install asks once, installs once however often Return is pressed, then waits."""
    machine = desktop.machine
    said = "$XDG_RUNTIME_DIR/fwupd"
    offered = {"Devices": [{"DeviceId": "bios", "Releases": [{"Version": "1.13.0"}]}]}
    # Once in, fwupd's history says the BIOS's update waits for the
    # restart, as a capsule does; each run is counted, so a second Return
    # that started a second would show.
    pending = {
        "Devices": [{"DeviceId": "bios", "UpdateState": 1, "Releases": [{"Version": "1.13.0"}]}]
    }
    installs = "\n".join(
        [
            "#!/bin/sh",
            f'echo "$*" >> {said}/installed',
            "sleep 3",
            f"echo {shlex.quote(json.dumps(pending))} > {said}/get-history.json",
            f"rm {said}/get-updates.json",
        ]
    )
    desktop.as_account(
        f"mkdir -p {said} && echo {shlex.quote(json.dumps(offered))} > {said}/get-updates.json"
        f" && printf '%s\\n' {shlex.quote(installs)} > {said}/update && chmod +x {said}/update"
    )
    asked = "Install 1.13.0 on System Firmware Now"
    try:
        desktop.as_account("qs ipc call launcher toggle")
        desktop.launcher_ready()
        machine.send_chars("machine ", delay=KEY_PACE)
        desktop.launcher_until(
            lambda shown: firmware_reads(shown).startswith("Firmware \u00b7 1 update ready")
        )
        desktop.type_in("firm", "Firmware")
        desktop.step_in()
        desktop.launcher_until(lambda shown: shown["rows"][:1] == ["System Firmware"])
        desktop.step_in()
        desktop.launcher_until(lambda shown: shown["rows"][:1] == ["Install 1.13.0"])
        # Return asks, its foot saying what it ends; nothing has run yet.
        machine.send_key("ret")
        desktop.launcher_until(
            lambda shown: (
                shown["rows"][:1] == [asked]
                and shown["footer"] == "it can't be undone, and finishes at a restart"
            )
        )
        desktop.photograph("launcher-firmware-install-asks")
        machine.fail(desktop.in_session(f"test -e {said}/installed"))
        # Return again installs, keeping the launcher and saying so; a
        # third Return while it goes in does nothing more.
        machine.send_key("ret")
        desktop.launcher_until(lambda shown: shown["footer"] == "installing")
        machine.send_key("ret")
        # Once in, Install waits for the restart, dimmed in place.
        machine.send_key("left")
        desktop.launcher_until(
            lambda shown: (
                shown["rows"][:1] == ["Install 1.13.0"]
                and shown["things"][0]["gone"] is True
                and shown["footer"] == "restart to finish"
            )
        )
        desktop.photograph("launcher-firmware-install-waits")
        ran = desktop.as_account(f"cat {said}/installed").splitlines()
        assert ran == ["update bios --assume-yes --no-reboot-check"], ran
        desktop.close_launcher()
    finally:
        desktop.as_account(f"rm -rf {said}")


def test_the_screens_warmth_follows_brightness_and_return_switches_it(desktop: Desktop) -> None:
    """The screen's warmth follows its Brightness, says how it stands, and Return switches it."""
    machine = desktop.machine
    note = "/run/user/1000/backlight-warmth"

    def warmth(said: dict[str, Any]) -> None:
        """Say how the warmth stands as the backlight daemon does, whole, by a rename."""
        text = json.dumps(said)
        desktop.as_account(f"echo '{text}' > {note}.writing && mv {note}.writing {note}")

    def warmth_selected(side: str, verb: str, footer: str) -> Callable[[Json], bool]:
        """Give a test that the Warmth is selected, standing as said."""
        return lambda shown: (
            (shown["selected"] or {}).get("name") == "Warmth"
            and (
                shown["selected"]["side"],
                shown["selected"]["verb"],
                shown["footer"],
            )
            == (side, verb, footer)
        )

    # The backlight daemon warms nothing here, so its file is said for it.
    warmth({"kelvin": 5200, "followsTheRoom": True})
    desktop.as_account("qs ipc call launcher collection displays")
    desktop.launcher_ready()
    desktop.step_in()
    desktop.launcher_until(lambda shown: shown["rows"][1:3] == ["Brightness", "Warmth"])
    desktop.down_to("Warmth")
    desktop.launcher_until(warmth_selected("on", "off", "5200 K, following the room"))
    desktop.photograph("launcher-warmth")
    # Return tells the daemon over the session's bus, which is heard here.
    desktop.as_account(
        "systemd-run --user --collect --quiet --unit=warmth-heard -- sh -c "
        "'busctl --user monitor io.jitsusama.Backlight > /tmp/warmth-heard'"
    )
    machine.wait_until_succeeds(
        desktop.in_session("systemctl --user is-active warmth-heard"),
        timeout=dt.timedelta(seconds=10),
    )
    machine.send_key("ret")
    machine.wait_until_succeeds(
        "grep -q FollowsTheRoom /tmp/warmth-heard && grep -q 'BOOLEAN false' /tmp/warmth-heard",
        timeout=dt.timedelta(seconds=10),
    )
    # It stays to show the change, which comes as the daemon says it.
    warmth({"kelvin": 6500, "followsTheRoom": False})
    desktop.launcher_until(warmth_selected("off", "on", "6500 K, the screen's own white"))
    # Where nothing warms the screen, the daemon leaves {}, and the Warmth
    # goes: kept in its place while stepped in, as any part that goes is.
    warmth({})
    desktop.launcher_until(
        lambda shown: all(
            thing["gone"] for thing in shown["things"] if thing["is"].endswith(":warmth")
        )
    )
    desktop.as_account("systemctl --user stop warmth-heard")
    desktop.close_launcher()


# A binding that throws or assigns the wrong type only warns, and
# only once a surface draws it, which can be long after the start.


# An answer that has to be asked for holds its row from the keystroke, so
# the rows below it never move down when it comes, or when it doesn't.


# The processor telemetry reader's reading, as it gives one on the laptop,
# with a part Heat doesn't list among them; a stand-in says it each second,
# as the reader does.
TELEMETRY_READING: Json = {
    "temperatures": [
        {"part": "npu", "celsius": 62},
        {"part": "imageProcessor", "celsius": 63},
        {"part": "media", "celsius": 59},
        {"part": "display", "celsius": 60},
        {"part": "systemAgent", "celsius": 65},
    ],
    "heldBack": [{"by": "pl1", "share": 1.0}],
    "clusters": [
        {"cpus": "0", "heldBack": [{"by": "power", "share": 0.98}]},
        {"cpus": "1", "heldBack": []},
    ],
    "npu": [],
    "keptFromResting": {"kept": 0.12, "by": [{"by": "display", "share": 0.12}]},
}
TELEMETRY_HELD = "/run/processor-telemetry"


def stand_in_for_the_telemetry(desktop: Desktop) -> None:
    """Serve the reader's reading on its socket, each second, as the reader does."""
    socket = f"{TELEMETRY_HELD}/io.jitsusama.ProcessorTelemetry"
    desktop.machine.succeed(f"install -d -o 1000 {TELEMETRY_HELD}")
    the_telemetry_says(desktop, TELEMETRY_READING)
    desktop.as_account(
        "systemd-run --user --collect --quiet --unit=telemetry-stand-in -- "
        f"systemd-socket-activate --inetd --accept --listen={socket} "
        "sh -c 'while cat /tmp/telemetry-said; do sleep 1; done'"
    )
    desktop.machine.wait_until_succeeds(f"test -S {socket}", timeout=dt.timedelta(seconds=10))


def the_telemetry_says(desktop: Desktop, reading: Json) -> None:
    """Have the reader's stand-in serve this reading from its next second on."""
    told = json.dumps({"parameters": reading})
    desktop.as_account(
        f"printf '%s\\0' '{told}' > /tmp/telemetry-said.new"
        " && mv /tmp/telemetry-said.new /tmp/telemetry-said"
    )


def the_telemetry_is_let_go(desktop: Desktop) -> None:
    """Wait until nothing watches the reader, then take its stand-in away."""
    # With the launcher shut nothing watches, so the reader is let go: the
    # stand-in's one shell for the connection ends, leaving it listening alone.
    desktop.machine.wait_until_succeeds(
        desktop.in_session(
            'test "$(systemctl --user show -P TasksCurrent telemetry-stand-in)" = 1'
        ),
        timeout=dt.timedelta(seconds=10),
    )
    desktop.as_account("systemctl --user stop telemetry-stand-in")
    desktop.machine.succeed(f"rm -rf {TELEMETRY_HELD}")


def test_the_processors_telemetry_shows_in_heat_and_in_what_held_it_back(
    desktop: Desktop,
) -> None:
    """The processor's telemetry shows its own parts in Heat, what held it back and kept it up."""
    own = ["NPU", "Image Processor", "Media Engine", "Display Engine"]
    # Heat is looked at before the reader is there, so its connection is
    # refused first, as when the reader's socket is missing on a machine.
    desktop.answer("heat")
    desktop.launcher_until(lambda shown: shown["kind"] == "answer")
    desktop.step_in()
    stand_in_for_the_telemetry(desktop)
    # Listed once the first reading comes, stepped in already, in order:
    # the refused connection is made afresh while Heat is still shown.
    desktop.launcher_until(
        lambda shown: (
            [row for row in shown["rows"] if row in own] == own
            and "System Agent" not in shown["rows"]
        )
    )
    desktop.type_in("npu", "NPU")
    assert desktop.showing()["selected"]["side"] == "62 \u00b0C"
    desktop.close_launcher()
    # The Processor says what held it back, after its Clock, and each core
    # held, a step in; the core its cluster didn't hold isn't listed.
    desktop.answer("processor")
    desktop.launcher_until(lambda shown: shown["kind"] == "answer")
    desktop.step_in()
    desktop.launcher_until(lambda shown: "Held Back" in shown["rows"])
    # Clock is there only where the kernel says the cores' clocks.
    rows = desktop.showing()["rows"]
    assert rows[rows.index("Held Back") - 1] in ("Clock", "Maxed"), rows
    desktop.type_in("held", "Held Back")
    desktop.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("side") == "100%"
            and shown["footer"] == "by its power limit, PL1, over the last second"
        )
    )
    desktop.step_in()
    desktop.launcher_until(lambda shown: shown["rows"] == ["Core 0", "Copy"])
    desktop.close_launcher()
    # What kept it from resting comes and goes, so it's the last part, before
    # the step's Copy, saying the part that kept it most, and each part that
    # did a step in.
    desktop.answer("processor")
    desktop.launcher_until(lambda shown: shown["kind"] == "answer")
    desktop.step_in()
    desktop.launcher_until(lambda shown: shown["rows"][-2:] == ["Kept From Resting", "Copy"])
    desktop.type_in("kept", "Kept From Resting")
    desktop.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("side") == "12%"
            and shown["footer"] == "by the display engine"
        )
    )
    desktop.photograph("launcher-kept-from-resting")
    desktop.step_in()
    desktop.launcher_until(lambda shown: shown["rows"] == ["Display Engine", "Copy"])
    desktop.close_launcher()
    the_telemetry_is_let_go(desktop)


def open_power(desktop: Desktop) -> None:
    """Open Power where it lives, in Machine, found by its name from the top.

    Its word and a space don't step into it, since "power off" goes on from them.
    """
    desktop.answer("power")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Power")
    desktop.step_in("ret")


def test_power_says_what_kept_the_processor_from_resting_on_battery(desktop: Desktop) -> None:
    """Power says what kept the processor from resting while on battery, and not on mains."""
    stand_in_for_the_telemetry(desktop)
    # On battery, Power holds it too, right after Battery, where Joel looks
    # when the battery drains; nothing else shown asks for the reading, so
    # Power's holding it is what has it read. It's held whole, with no
    # parts, until that reading comes: were it dropped then, it would come
    # later at the end, after the commands, not second. On mains it isn't
    # there.
    desktop.power("off")
    try:
        open_power(desktop)
        desktop.launcher_until(
            lambda shown: (
                shown["path"] == ["Machine", "Power"]
                and shown["rows"][:2] == ["Battery", "Kept From Resting"]
            )
        )
        desktop.down_to("Kept From Resting")
        desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("side") == "12%")
        desktop.photograph("launcher-kept-from-resting-on-battery")
        # The charger going in while it's shown doesn't move it: it stays
        # where it was, dimmed, saying why, until Power is opened again.
        desktop.power("on")

        def kept_from_resting(shown: Json) -> Json:
            return next(
                (
                    thing
                    for thing in shown["things"]
                    if str(thing["reads"]).startswith("Kept From Resting")
                ),
                {},
            )

        desktop.launcher_until(
            lambda shown: (
                cast("list[str]", shown["rows"])[:2] == ["Battery", "Kept From Resting"]
                and kept_from_resting(shown).get("gone") is True
                and (shown["selected"] or {}).get("note") == "on the charger"
            )
        )
        # It stays so as readings go on coming, rather than lighting again
        # with each: a new one is said on the dimmed row, and every report
        # until then has it dimmed and second.
        lit: list[str] = []

        def said_dimmed(shown: Json) -> bool:
            rows = cast("list[str]", shown["rows"])
            kept = kept_from_resting(shown)
            if rows[:2] != ["Battery", "Kept From Resting"] or kept.get("gone") is not True:
                lit.append(str(shown["things"]))
            return str(kept.get("reads", "")).startswith("Kept From Resting \u00b7 20%")

        more = dict(TELEMETRY_READING)
        more["keptFromResting"] = {"kept": 0.2, "by": [{"by": "display", "share": 0.2}]}
        the_telemetry_says(desktop, more)
        desktop.launcher_until(said_dimmed)
        assert lit == [], lit
        desktop.close_launcher()
    finally:
        desktop.power("on")
    open_power(desktop)
    desktop.launcher_until(
        lambda shown: shown["path"] == ["Machine", "Power"] and "Battery" in shown["rows"]
    )
    assert "Kept From Resting" not in desktop.showing()["rows"], desktop.showing()["rows"]
    desktop.close_launcher()
    the_telemetry_is_let_go(desktop)


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
