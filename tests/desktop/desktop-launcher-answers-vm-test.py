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


def yanked(desktop: Desktop) -> str:
    """Copy the answer shown by yy from normal, and give what was copied.

    An answer holding parts is gone into by Return, as every row holding more is, so y is
    what copies it (Joel, 2026-11-06: "i type memory in the launcher, hit enter and it
    copies and does not go into more details").
    """
    desktop.normal()
    since = desktop.mark()
    desktop.machine.send_chars("yy")
    desktop.reported("yanked 1", lambda rest: rest == "", since)
    taken = desktop.as_account("wl-paste --no-newline")
    desktop.close_launcher()
    return taken


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
    """The launcher copies a MAC's vendor and the charge by y, and steps into the charger's."""
    # What a sum or a measure says is tests/answers.qml's to hold; here,
    # only what needs the machine. The vendor comes from the list built
    # from hwdata, so this proves it's there; the answer holds its parts,
    # so Return goes in, and yy copies the vendor.
    desktop.answer("3c:22:fb:12:34:56")
    desktop.step_in()
    desktop.close_launcher()
    desktop.answer("3c:22:fb:12:34:56")
    assert yanked(desktop) == "Apple, Inc."
    # The battery's charge leads, as the kernel's stand-in has it.
    charge = charged(desktop)
    desktop.answer("battery")
    desktop.shoot("launcher-battery")
    assert yanked(desktop).startswith(f"{charge}% \u00b7 ")
    # Then what feeds it: the stand-in's charger, plugged in, a step
    # into the answer's row.
    desktop.answer("charger")
    desktop.launcher_until(lambda shown: shown["kind"] == "answer")
    # The copy is listed last, after the facts, since nothing else inside
    # does anything (Joel, 2026-10-28: never a copy at the top).
    desktop.follow()
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
    desktop.shoot("launcher-charger-parts")
    # Each part is named first, then what it says: the stand-in's health
    # against its design, and its temperature has a bar.
    desktop.launcher_until(
        lambda shown: (
            {"Charger", "Charge", "Health", "Temperature"} <= set(shown["rows"])
            and any(
                thing["reads"].startswith("Health \u00b7 ") and "% of design" in thing["reads"]
                for thing in shown["things"]
            )
        )
    )
    desktop.close_launcher()
    # A part is asked by its own words, its figure the headline.
    desktop.answer("battery health")
    desktop.launcher_until(
        lambda shown: (
            shown["kind"] == "answer" and "% of design" in (shown["selected"] or {}).get("name", "")
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
    desktop.shoot("launcher-heat")
    assert re.search("^\\u2212?[\\d.]+ \\u00b0C at the ", yanked(desktop))
    # Each part reads name first, a step in by Return; Return opens one,
    # its history first, and again once back out by -.
    desktop.answer("heat")
    desktop.launcher_until(lambda shown: shown["kind"] == "answer")
    desktop.step_in()
    desktop.launcher_until(lambda shown: "Battery" in shown["rows"])
    desktop.type_in("battery", "Battery")
    # Its heat first, then how far it is from its limit, where it has one.
    side = desktop.showing()["selected"]["side"]
    assert re.search("^\\u2212?[\\d.]+ \\u00b0C( \\u00b7 |$)", side), side

    def in_battery(shown: Json) -> bool:
        """Say whether the launcher holds the battery's heat, its Last Reads selected."""
        return bool(
            shown["held"][-1:] == ["Battery"]
            and (shown["selected"] or {}).get("name") == "Last Reads"
        )

    desktop.follow()
    desktop.launcher_until(in_battery)
    desktop.shoot("launcher-heat-part")
    desktop.step_out()
    desktop.launcher_until(
        lambda shown: len(shown["held"]) == 1 and (shown["selected"] or {}).get("name") == "Battery"
    )
    # A figure holding more steps in on Return,
    # and copies what it shows from the Copy listed last inside (Joel,
    # 2026-10-28: "hitting enter on the battery closes the window when
    # right opens it").
    machine.send_key("ret")
    desktop.launcher_until(lambda shown: in_battery(shown) and shown["rows"][-1:] == ["Copy"])
    desktop.down_to("Copy")
    machine.send_key("ret")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10))
    pasted = desktop.as_account("wl-paste --no-newline")
    assert re.search("^\\u2212?[\\d.]+ \\u00b0C$", pasted), pasted


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
    # With a number, Return sets it, as the brightness keys do: from the find
    # it lands on it, since it changes how things stand, and a second does it.
    desktop.answer("brightness 30")
    desktop.launcher_until(
        lambda shown: (shown["selected"] or {}).get("name") == "set the screen to 30%"
    )
    desktop.land()
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
    # Screens is held, with its screen, before Ctrl+] goes in, or the step in
    # would be counted from before Screens was.
    desktop.launcher_until(lambda shown: shown["held"] == ["screens"] and shown["rows"] != [])
    desktop.follow()
    desktop.launcher_until(lambda shown: shown["rows"][:3] == ["Focus", "Brightness", "Room Light"])
    # The screen's drawer and rows read the light outside the launcher's
    # bindings, so reading it writes nothing they read.
    log = desktop.shell_said("Binding loop")
    assert "Binding loop" not in log, f"the screen's light is read inside a binding: {log}"
    desktop.next_row()
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Brightness")
    desktop.step_in("ret")
    # Each level is said by its value alone, as every choice is (VKCF3Z, 2026-11-09).
    desktop.launcher_until(lambda shown: "40%" in shown["rows"])
    desktop.type_in("40", selecting="40%")
    # Return from the find lands on it, since it changes how things stand, and a second does it.
    desktop.land()
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
    desktop.follow()
    desktop.launcher_until(lambda shown: shown["rows"][:4] == [*KBD_LEVELS, "Copy"])
    for _ in range(KBD_LEVELS.index("Light to 2 of 2")):
        desktop.next_row()
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Light to 2 of 2")
    machine.send_key("ret")
    machine.wait_until_succeeds(f"grep -qx 2 {KBD_LIGHT}", timeout=dt.timedelta(seconds=10))
    # It changed something, so the launcher stays until it's closed.
    desktop.close_launcher()
    # The keyboards QEMU gives it, however many, and caps lock.
    desktop.answer("keyboards")
    desktop.shoot("launcher-keyboards")
    assert re.search("^(one keyboard|[1-9]\\d* keyboards), caps lock off$", yanked(desktop))
    # The keyboards are a step into the answer, and each holds its parts,
    # named first, a step further by Return. QEMU's
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
    # An answer asked at the top holds its parts, so Return goes into it.
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
    desktop.shoot("launcher-keyboard-parts")
    assert desktop.showing()["selected"]["side"] == "off"
    # In normal the field names no key, since - steps out rather than
    # clearing, so a letter typed on the find, then normal, brings none.
    desktop.on_find()
    machine.send_chars("l")
    desktop.launcher_until(lambda shown: shown["typed"] == "l")
    desktop.normal()
    field = json.loads(desktop.as_account("qs ipc call launcher laidOut"))["field"]
    desktop.shoot("launcher-keyboard-path-normal")
    assert not any("\u2190" in word["text"] for word in field), field
    assert not any(word["text"].endswith(" clear") for word in field), field
    desktop.on_find()
    machine.send_key("backspace")
    desktop.launcher_until(lambda shown: shown["typed"] == "")
    # Back in normal, where a changing row's Return does it rather than
    # landing, as main's clearing left arrow left it.
    desktop.normal()
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
    desktop.follow()
    desktop.launcher_until(lambda shown: board in shown["rows"])
    desktop.down_to(board)
    desktop.step_in("ret")
    desktop.launcher_until(lambda shown: "Repeat" in shown["rows"])
    desktop.step_out()
    desktop.launcher_until(
        lambda shown: len(shown["held"]) == 1 and (shown["selected"] or {}).get("name") == board,
    )
    desktop.follow()
    desktop.launcher_until(lambda shown: "Repeat" in shown["rows"])
    desktop.down_to("Repeat")
    desktop.launcher_until(
        lambda shown: (shown["selected"] or {}).get("side", "").startswith("after "),
    )
    machine.send_key("ret")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10))
    assert re.search("^after \\d+ ms \u00b7 \\d+ Hz$", desktop.as_account("wl-paste --no-newline"))


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
                "band": "filling",
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
    desktop.shoot("launcher-disks")
    # Its first part, a step in after Return's own Copy, is the space the
    # headline's disk has, and the disk's speed, how busy it is and its
    # latency come after.
    desktop.follow()
    desktop.launcher_until(lambda shown: shown["rows"][:1] == ["Space"])
    desktop.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name") == "Space"
            and shown["selected"]["side"] == "21.5\u00a0GB free"
            and {"Speed", "Busy", "Latency"} <= set(shown["rows"])
        )
    )
    # A step further, each disk the guard watches, the headline's first.
    # A disk is a thing, so the drawer is out for it: one that's short
    # says beside it its room and what the guard is doing, and in the
    # drawer its size and the guard's own words, its room not again
    # (docs/design.md, the drawer).
    desktop.follow()
    desktop.launcher_until(
        lambda shown: (
            shown["rows"][:1] == ["/nix"]
            and shown["selected"]["side"] == "21.5\u00a0GB free \u00b7 clearing old builds"
            and (shown.get("drawer") or {}).get("out") is True
            and shown["drawer"]["facts"] == ["Size"]
            and shown["drawer"]["body"] == "The disk is filling, with 20.0 GiB free."
        )
    )
    desktop.close_launcher()
    # After Space, what's filling the disks, the biggest beside it, and a
    # step in each holder, saying whether the guard clears any of it or
    # why not, its own sentence below.
    desktop.answer("disk")
    desktop.follow()
    desktop.launcher_until(lambda shown: shown["rows"][:2] == ["Space", "What's Filling It"])
    desktop.type_in("filling", selecting="What's Filling It")
    assert desktop.showing()["selected"]["side"] == "Containers \u00b7 12.9\u00a0GB"
    desktop.follow()
    desktop.launcher_until(
        lambda shown: (
            shown["rows"] == ["Containers", "Nix cache", "Copy"]
            and shown["selected"]["side"] == "12.9\u00a0GB"
            and str(shown["selected"]["reads"]).startswith("Container storage holds")
        )
    )
    desktop.close_launcher()
    # A part asked for alone, by its own words: how fast the disk
    # reads, writes, discards and flushes, each a step in, as the machine
    # daemon says them.
    desktop.answer("disk speed")
    desktop.launcher_until(
        lambda shown: (
            shown["kind"] == "answer"
            and re.fullmatch(r"[\d.,]+ (B|kB|MB|GB)/s", (shown["selected"] or {}).get("name", ""))
            is not None
        )
    )
    desktop.follow()
    desktop.launcher_until(
        lambda shown: shown["rows"] == ["Reading", "Writing", "Discarding", "Flushing", "Copy"]
    )
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
    desktop.follow()
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
    desktop.follow()
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
    desktop.follow()
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
            desktop.next_row()
        # The wait ends before the loop moves on, so it reads this pass's row.
        desktop.launcher_until(
            lambda shown: (
                (shown["selected"] or {}).get("name") == name  # noqa: B023
                and re.fullmatch(side, shown["selected"]["side"]) is not None  # noqa: B023
                and re.fullmatch(footer, shown["footer"]) is not None  # noqa: B023
            )
        )
    # With no session's window to go to, its act is copying its name, which
    # y does; Return goes in, to what it runs and the rest the queue knows
    # of it, as Return goes in wherever a row holds more.
    assert desktop.showing()["selected"]["verb"] == "copy", desktop.showing()["selected"]
    desktop.step_in()
    desktop.launcher_until(
        lambda shown: (
            {"Command", "Class", "Needs", "Since"} <= set(shown["rows"])
            and "Memory" not in shown["rows"]
        )
    )
    desktop.shoot("launcher-queue")
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
    # Return goes into an answer that holds parts, and into a part that
    # holds its own, rather than copying it (Joel, 2026-11-06: "i type
    # memory in the launcher, hit enter and it copies").
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
    desktop.launcher_until(lambda shown: "B used" in (shown["selected"] or {}).get("name", ""))
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
            {"path": "/", "band": "filling", "free": 5 * 2**30, "total": 30 * 2**30},
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
    desktop.follow()
    desktop.type_in("filling", selecting="What's Filling It")
    desktop.follow()
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
        assert "Grew \u00b7 2.15\u00a0GB in 20 h" in reads, reads
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
        # Letting it go ends the keeping, so Return writes its line, which asks first, in
        # the act's words and the folder's name, never the line's :snooze.
        desktop.line_answered("snooze", "Release ~/scratch? (y/n)")
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
        selected = desktop.showing()["selected"]
        assert selected["side"] == "1.07\u00a0GB", selected
        desktop.step_in("ret")
        desktop.launcher_until(lambda shown: shown["rows"] == ["Clear 1.07\u00a0GB of ~/scratch"])
        # The step asked, so its line is banged, and a fresh Return there clears it, asking
        # no more.
        desktop.line("delete!")
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
    assert desktop.here_kept() is None
    # The VM sets no time zone and no place, so here is nowhere until
    # whereabouts says where the laptop is. This has it say a position in
    # GeoClue's stead.
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
    desktop.found_at(said)
    # Here is the laptop's position now, heard over whereabouts' socket,
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
    # And once whereabouts says nowhere, here is nowhere again, and the
    # location is said to be unknown rather than guessed.
    desktop.found_at(None)
    asked(nowhere)
    desktop.answer("location")
    desktop.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name") == "Location"
            and (shown["selected"] or {}).get("side") == "unknown"
        )
    )
    desktop.close_launcher()


# What clock-follows keeps of its last move and a zone put back, forgotten for the tests after;
# named, since the account's shell refuses a pattern that matches nothing.
FORGET_MOVES = (
    "rm -f ~/.local/state/clock-follows/moved.json ~/.local/state/clock-follows/held.json"
)


def test_the_clock_follows_the_laptop_into_another_zone_and_is_put_back(desktop: Desktop) -> None:
    """The clock follows the laptop into another zone, says so in Time, and is put back."""
    machine = desktop.machine
    zone = "timedatectl show --property=Timezone --value"
    began = machine.succeed(zone).strip()
    # clock-follows says in its journal each time it has decided, never where.
    decided = "journalctl --user -u clock-follows -o cat --no-pager | grep -c 'was decided'; true"

    def found(where: str, accuracy: int, place: str) -> None:
        """Say where the laptop is, as whereabouts does, and wait for the clock to decide."""
        before = desktop.as_account(decided).strip()
        said = {"v": 2, "latitude": 52.34, "longitude": -115.67, "accuracy": accuracy,
                "when": int(time.time()), "place": place, "zone": where}  # fmt: skip
        desktop.found_at(said)
        machine.wait_until_succeeds(
            desktop.in_session(f'test "$({decided})" -gt {shlex.quote(before or "0")}'),
            timeout=dt.timedelta(seconds=30),
        )

    def keeps(where: str) -> None:
        """Wait until the clock keeps this zone."""
        machine.wait_until_succeeds(f'test "$({zone})" = {where}', timeout=dt.timedelta(seconds=30))

    try:
        # Found by Wi-Fi, to within 300 m, in another zone: the clock moves,
        # and the time here says so, the zone being the clock's.
        found("America/Edmonton", 300, "Kelpwater, Highreach")
        keeps("America/Edmonton")
        desktop.answer("time")
        desktop.launcher_until(
            lambda shown: (
                shown["footer"] == "clock moved to Edmonton time, as the laptop is in Kelpwater"
            )
        )
        desktop.follow()
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
        found("America/Edmonton", 300, "Kelpwater, Highreach")
        assert machine.succeed(zone).strip() == began
        # Somewhere else lets it go, and the clock follows again.
        found("America/Vancouver", 300, "Harbour, Farshore")
        keeps("America/Vancouver")
        # Found only to a country's width, wider than a zone needs, it never
        # moves the clock.
        found("Asia/Tokyo", 300000, "Saltmere, Farshore")
        assert machine.succeed(zone).strip() == "America/Vancouver"
    finally:
        machine.succeed(f"timedatectl set-timezone {shlex.quote(began)}")
        desktop.found_at(None)
        desktop.as_account(FORGET_MOVES)


def test_a_place_set_by_hand_holds_until_the_laptop_is_found_again(desktop: Desktop) -> None:
    """A place set by hand holds over GeoClue until the laptop is found again."""
    machine = desktop.machine
    kept = desktop.KEPT_HERE
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
        desktop.follow()
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
        said = {"v": 2, "latitude": 52.34, "longitude": -115.67, "accuracy": 1000,
                "when": int(time.time()), "place": "Kelpwater, Highreach",
                "zone": "America/Edmonton", "country": "Canada", "hand": True}  # fmt: skip
        # whereabouts hears the pin at once, without starting again.
        desktop.as_account(
            f"mkdir -p ~/.local/state/whereabouts && echo '{json.dumps(said)}' > {pin}"
        )
        machine.wait_until_succeeds(
            desktop.in_session(f"grep -q '\"hand\":true' {kept}"), timeout=dt.timedelta(seconds=30)
        )
        machine.wait_until_succeeds(
            "test $(timedatectl show --property=Timezone --value) = America/Edmonton",
            timeout=dt.timedelta(seconds=30),
        )
        location(lambda row: row["side"] == "Kelpwater, Highreach")
        desktop.follow()
        # Inside, in the design's order, saying it was set by hand, and in which country.
        inside = [
            "Town", "Country", "How It Knows", "Located", "Set It by Hand", "Find It Again", "Copy",
        ]  # fmt: skip
        desktop.launcher_until(lambda shown: shown["rows"] == inside)
        reads = [thing["reads"] for thing in desktop.showing()["things"]]
        assert "How It Knows \u00b7 set by hand" in reads, reads
        assert "Country \u00b7 Canada" in reads, reads
        # Found again, the pin goes, and here is what GeoClue says, which
        # in the VM is nothing yet.
        desktop.down_to("Find It Again")
        machine.send_key("ret")
        machine.wait_until_succeeds(
            desktop.in_session(f"test ! -e {pin} && test ! -e {kept}"),
            timeout=dt.timedelta(seconds=30),
        )
        close()
        location(lambda row: row["side"] == "unknown")
        close()
    finally:
        machine.succeed(f"timedatectl set-timezone {shlex.quote(began)}")
        desktop.as_account(f"rm -f {pin}")
        desktop.found_at(None)
        desktop.as_account(FORGET_MOVES)


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

    # Ctrl+] goes into what keeps the time, as Return does.
    desktop.answer("time")
    desktop.follow()
    # A step holds its rows still, so a row come after it was made goes last (Steady.js):
    # where chrony's reading is back only once the step is made, Servers follows Copy, and
    # stepping in afresh puts it after Clock.
    desktop.launcher_until(lambda shown: "Servers" in shown["rows"])
    if not kept_rows(desktop.showing()):
        desktop.step_out()
        desktop.follow()
    desktop.launcher_until(kept_rows)


def begins_at_sunset(drawer: Json) -> bool:
    """Whether a plate says, just after its Day, at what time today's sunset begins its day."""
    facts = cast("list[str]", drawer.get("drawnFacts") or [])
    values = cast("list[str | None]", drawer.get("values") or [])
    if "Day" not in facts:
        return False
    at = facts.index("Day") + 1
    return (
        at < len(facts)
        and facts[at] in {"Day Begins", "Day Began"}
        and re.fullmatch(r"\d{2}:\d{2}", values[at] or "") is not None
    )


def date_steps_into_its_calendars(desktop: Desktop) -> None:
    """Step Date into today in other calendars, once without the reader and once with it."""
    # Seen from Greenwich, as whereabouts would say it, with no zone to move the clock to,
    # so the Hebrew day says when the sun sets there to begin it. What was there before is
    # put back after, or nowhere.
    said = {"v": 2, "latitude": 51.4769, "longitude": -0.0005, "accuracy": 3000, "when": 1,
            "place": None, "zone": None}  # fmt: skip
    was = desktop.here_kept()
    desktop.found_at(said)
    try:
        calendars_asked(desktop)
    finally:
        desktop.found_at(was)


def calendars_from_now(desktop: Desktop) -> None:
    """Walk from home into Now, then Date, then Calendars, and find the four calendars there.

    That's the way Joel goes (2026-11-07: "Now -> Date -> Calendars does not work from the
    launcher"), each a step by Return rather than a search, so Date is held under Now.
    """
    desktop.answer("")
    desktop.normal()
    desktop.down_to("Now")
    desktop.step_in()
    desktop.launcher_until(lambda shown: "Date" in shown["rows"])
    desktop.down_to("Date")
    desktop.step_in()
    desktop.launcher_until(lambda shown: "Calendars" in shown["rows"])
    desktop.down_to("Calendars")
    desktop.step_in()
    calendars = ["Chinese", "Japanese", "Hebrew", "Islamic"]
    desktop.launcher_until(lambda shown: shown["rows"][: len(calendars)] == calendars)
    desktop.close_launcher()


def the_shell_restarted(desktop: Desktop, said: str) -> None:
    """Restart the shell and wait until its calendars say how the reader answered today."""
    since = desktop.mark()
    desktop.as_account("systemctl --user restart quickshell")
    desktop.reported(said, lambda _rest: True, since, dt.timedelta(seconds=60))


def calendars_asked(desktop: Desktop) -> None:
    """Ask Date's calendars without the reader, then with it, from where the laptop is."""
    # With the reader gone as the shell starts, Calendars says it isn't answering, and Return
    # closes rather than copying those words. The shell starts again for it, since today is
    # asked for as it starts and a day once answered is kept.
    try:
        desktop.as_account("systemctl --user stop timekeeping.socket timekeeping.service")
        the_shell_restarted(desktop, "calendars refused")
        desktop.answer("date")
        desktop.follow()
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
    # Shown again, Date asks again, the way Joel goes to it.
    calendars_from_now(desktop)
    # With the reader there as the shell starts, today's answer is held before the launcher
    # opens, so Date's first frame holds today in the Chinese, Japanese, Hebrew and Islamic
    # calendars, with nothing to wait for (Joel, 2026-11-07: "calendars takes way too long to
    # load"). Each is a thing whose plate is titled with its date in its own script.
    the_shell_restarted(desktop, "calendars held ")
    desktop.answer("date")
    desktop.follow()
    first = [str(thing["reads"]) for thing in desktop.showing()["things"]]
    assert any(it.startswith("Calendars \u00b7 4") for it in first), f"Date showed {first}"
    desktop.down_to("Calendars")
    desktop.follow()
    calendars = ["Chinese", "Japanese", "Hebrew", "Islamic"]
    desktop.launcher_until(lambda shown: shown["rows"][: len(calendars)] == calendars)
    desktop.down_to("Hebrew")
    desktop.launcher_until(
        lambda shown: (
            shown["drawer"]["out"] is True
            and any("\u0590" <= letter <= "\u05ff" for letter in shown["drawer"]["title"])
            and begins_at_sunset(shown["drawer"])
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
        "Offset", "RMS Offset", "Frequency", "Skew", "Stratum", "Root Delay",
        "Root Dispersion", "Leap Second", "Last Sync", "Update Interval", "Next Poll",
    ]  # fmt: skip
    desktop.down_to("Clock")
    desktop.follow()
    desktop.launcher_until(lambda shown: shown["rows"][: len(clock)] == clock)
    reads = [str(thing["reads"]) for thing in desktop.showing()["things"]]
    for read in ("Leap Second \u00b7 none announced", "Service \u00b7 chrony 4."):
        assert any(it.startswith(read) for it in reads), (read, reads)
    desktop.close_launcher()
    # Servers steps into each server chrony asks, the chosen one first, each a thing whose
    # note says where it stands, its drawer and its inside every figure chronyd has of it.
    time_inside(desktop)
    desktop.down_to("Servers")
    desktop.follow()
    servers = ["time.harbour.test", "far.harbour.test"]
    desktop.launcher_until(lambda shown: shown["rows"][: len(servers)] == servers)
    desktop.down_to("time.harbour.test")
    assert desktop.showing()["selected"]["note"] == "chosen", desktop.showing()["selected"]
    # Its identifiers, its address among them, come last (Y2YBTS's review of #2303), and it
    # has no Why, which its note "chosen" says (VKCF3Z, 2026-11-08).
    inside = ["Stratum", "Poll Interval", "Reach", "Last Sample", "Offset"]
    desktop.launcher_until(
        lambda shown: (
            shown["drawer"]["out"] is True
            and shown["drawer"]["title"] == "time.harbour.test"
            and shown["drawer"]["facts"][: len(inside)] == inside
        )
    )
    facts = desktop.showing()["drawer"]["facts"]
    assert facts.index("Address") > facts.index("Packets"), facts
    # One never heard says so on its side, and its note doesn't say it again.
    desktop.down_to("far.harbour.test")
    far = desktop.showing()["selected"]
    assert (far["side"], far["note"]) == ("not answering", ""), far
    desktop.down_to("time.harbour.test")
    desktop.follow()
    desktop.launcher_until(lambda shown: shown["rows"][: len(inside)] == inside)
    reads = [str(thing["reads"]) for thing in desktop.showing()["things"]]
    for read in (
        "Stratum \u00b7 2",
        "NTP Version \u00b7 4",
    ):
        assert any(it.startswith(read) for it in reads), (read, reads)
    desktop.close_launcher()
    # With chronyd stopped, nothing keeps the time, and Time says so and no more.
    try:
        desktop.machine.succeed("systemctl stop chronyd")
        desktop.answer("time")
        desktop.follow()
        desktop.launcher_until(
            lambda shown: any(
                str(thing["reads"]).startswith("Clock \u00b7 nothing keeps the time")
                for thing in shown["things"]
            )
        )
        assert "Servers" not in desktop.showing()["rows"], desktop.showing()["rows"]
        desktop.down_to("Clock")
        # Clock always steps in, saying inside that nothing keeps the time
        # (VKCF3Z, 2026-11-06), so Return never lands on nothing.
        assert desktop.showing()["selected"]["verb"] == "more", desktop.showing()["selected"]
        desktop.step_in("ret")
        desktop.launcher_until(lambda shown: shown["nothing"] == "Nothing keeps the time.")
        desktop.close_launcher()
    finally:
        desktop.machine.succeed("systemctl start chronyd")
    keeps_one_clock(desktop)


def keeps_one_clock(desktop: Desktop) -> None:
    """Step into Time before its reader answers, and see one Clock once it does.

    Time waits on a Clock, and the Clock the answer brings takes its place, never beside it.
    The reader is held back until the step is in, and Time asks it again each second.
    """
    try:
        desktop.as_account("systemctl --user stop timekeeping.socket timekeeping.service")
        desktop.answer("time")
        desktop.follow()
        desktop.launcher_until(lambda shown: "Clock" in shown["rows"])
    finally:
        desktop.as_account("systemctl --user start timekeeping.socket")
    desktop.launcher_until(lambda shown: "Servers" in shown["rows"])
    assert desktop.showing()["rows"].count("Clock") == 1, desktop.showing()["rows"]
    desktop.close_launcher()


def firmware_reads(shown: Json) -> str:
    """Give what Firmware's row reads where the launcher lists it, or nothing."""
    reads = [str(thing["reads"]) for thing in shown["things"]]
    return next((read for read in reads if read.startswith("Firmware \u00b7")), "")


def last_place(shown: Json) -> str:
    """Give the last of this machine's own places the launcher lists, or nothing.

    Its commands come after, and Machines, the realm's others, after its own.
    """
    places = [str(thing["is"]) for thing in shown["things"]]
    held = [
        place
        for place in places
        if not place.startswith("command:") and place != "collection:machines"
    ]
    return held[-1] if held else ""


def test_firmware_says_whats_offered_and_holds_each_devices_version(desktop: Desktop) -> None:
    """Firmware says whether an update is offered, last of Machine's own, and each version."""
    machine = desktop.machine
    said = "$XDG_RUNTIME_DIR/fwupd"
    # With nothing offered, as the laptop's fwupd says, it's up to date,
    # the last of this machine's own places, after Lag where Lag is shown
    # (only once something has lagged), and before Machines and its
    # commands.
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
        desktop.shoot("launcher-firmware-in-machine")
        desktop.follow()
        # A device is a thing, so the drawer is out for the BIOS, with the
        # release's own words; its versions are the row's side alone.
        desktop.launcher_until(
            lambda shown: (
                shown["rows"][:2] == ["System Firmware", "PVC10 SK hynix 1024GB"]
                and (shown.get("drawer") or {}).get("out") is True
                and shown["drawer"]["facts"] == []
                and "Fixes the keyboard's light." in str(shown["drawer"]["body"])
            )
        )
        reads = [str(thing["reads"]) for thing in desktop.showing()["things"]]
        assert reads[0].startswith("System Firmware \u00b7 1.13.0 ready \u00b7 from 1.12.0"), reads
        assert reads[1].startswith("PVC10 SK hynix 1024GB \u00b7 61001171"), reads
        desktop.shoot("launcher-firmware")
        desktop.follow()
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
        desktop.follow()
        desktop.launcher_until(lambda shown: shown["rows"][:1] == ["System Firmware"])
        desktop.follow()
        desktop.launcher_until(lambda shown: shown["rows"][:1] == ["Install 1.13.0"])
        # Return asks, its foot saying what it ends; nothing has run yet.
        machine.send_key("ret")
        desktop.launcher_until(
            lambda shown: (
                shown["rows"][:1] == [asked]
                and shown["footer"] == "it can't be undone, and finishes at a restart"
            )
        )
        desktop.shoot("launcher-firmware-install-asks")
        machine.fail(desktop.in_session(f"test -e {said}/installed"))
        # Return again writes its line, banged, since the step was the asking, and a fresh
        # Return there installs, keeping the launcher and saying so; a third Return while
        # it goes in installs nothing more, at most writing the line again, which Escape
        # lets go.
        desktop.line("install!")
        machine.fail(desktop.in_session(f"test -e {said}/installed"))
        machine.send_key("ret")
        desktop.launcher_until(lambda shown: shown["footer"] == "installing")
        machine.send_key("ret")
        desktop.held_still(dt.timedelta(seconds=2))
        if desktop.showing()["command"] is not None:
            machine.send_key("esc")
            desktop.launcher_until(lambda shown: shown["command"] is None)
        # Once in, Install waits for the restart, dimmed in place.
        desktop.step_out()
        desktop.launcher_until(
            lambda shown: (
                shown["rows"][:1] == ["Install 1.13.0"]
                and shown["things"][0]["gone"] is True
                and shown["footer"] == "restart to finish"
            )
        )
        desktop.shoot("launcher-firmware-install-waits")
        ran = desktop.as_account(f"cat {said}/installed").splitlines()
        assert ran == ["update bios --assume-yes --no-reboot-check"], ran
        desktop.close_launcher()
    finally:
        desktop.as_account(f"rm -rf {said}")


def test_the_screens_warmth_follows_brightness_and_return_switches_it(desktop: Desktop) -> None:
    """The screen's warmth follows its Brightness, says what holds it, and its switches switch.

    Inside are the Night Light and Follow the Room, each switched by Return, then the room's
    white and the night's; :set nightlight and :set warmth say and switch them too.
    """
    machine = desktop.machine
    note = "/run/user/1000/backlight-warmth"

    def warmth(said: dict[str, Any]) -> None:
        """Say how the warmth stands as the backlight daemon does, whole, by a rename."""
        text = json.dumps(said)
        desktop.as_account(f"echo '{text}' > {note}.writing && mv {note}.writing {note}")

    def selected(name: str, side: str, verb: str) -> Callable[[Json], bool]:
        """Give a test that a row is selected, standing as said."""
        return lambda shown: (
            (shown["selected"] or {}).get("name") == name
            and (shown["selected"]["side"], shown["selected"]["verb"]) == (side, verb)
        )

    # The backlight daemon warms nothing here, so its file is said for it: a night under way,
    # begun an hour ago and ending in ten, and a sensor that sees the room's colour.
    now = int(machine.succeed("date +%s").strip())
    sunrise = now + 10 * 3600
    clock = desktop.as_account(f"date -d @{sunrise} +%H:%M").strip()
    night = {
        "kelvin": 4000,
        "followsTheRoom": True,
        "seesColour": True,
        "roomWhite": 3200,
        "nightLight": True,
        "nightWhite": 4000,
        "sunset": (now - 3600) * 1000,
        "sunrise": sunrise * 1000,
    }
    warmth(night)
    desktop.as_account("qs ipc call launcher collection displays")
    desktop.launcher_ready()
    desktop.follow()
    desktop.launcher_until(lambda shown: shown["rows"][1:3] == ["Brightness", "Warmth"])
    desktop.down_to("Warmth")
    desktop.launcher_until(selected("Warmth", "4,000 K", "more"))
    assert desktop.showing()["selected"]["note"] == f"night light until {clock}", desktop.showing()[
        "selected"
    ]
    desktop.shoot("launcher-warmth")
    inside = ["Night Light", "Follow the Room", "Room's White", "Sunrise", "Sunset"]
    desktop.follow()
    desktop.launcher_until(lambda shown: shown["rows"][: len(inside)] == inside)
    # Return tells the daemon over the session's bus, which is heard here.
    desktop.as_account(
        "systemd-run --user --collect --quiet --unit=warmth-heard -- sh -c "
        "'busctl --user monitor io.jitsusama.Backlight > /tmp/warmth-heard'"
    )
    machine.wait_until_succeeds(
        desktop.in_session("systemctl --user is-active warmth-heard"),
        timeout=dt.timedelta(seconds=10),
    )
    desktop.down_to("Night Light")
    machine.send_key("ret")
    machine.wait_until_succeeds(
        "grep -q NightLight /tmp/warmth-heard && grep -q 'BOOLEAN false' /tmp/warmth-heard",
        timeout=dt.timedelta(seconds=10),
    )
    # It stays to show the change, which comes as the daemon says it.
    warmth(night | {"kelvin": 5000, "nightLight": False})
    desktop.launcher_until(selected("Night Light", "off", "on"))
    machine.send_chars(":set nightlight?\n")
    desktop.launcher_until(
        lambda shown: shown["command"] is None and shown["commandSaid"] == "nonightlight"
    )
    desktop.down_to("Follow the Room")
    machine.send_key("ret")
    machine.wait_until_succeeds(
        "grep -q FollowsTheRoom /tmp/warmth-heard", timeout=dt.timedelta(seconds=10)
    )
    warmth(night | {"kelvin": 6500, "followsTheRoom": False, "nightLight": False})
    desktop.launcher_until(selected("Follow the Room", "off", "on"))
    # :set warmth? says so as the row does, and :set warmth tells the daemon as Return does.
    machine.send_chars(":set warmth?\n")
    desktop.launcher_until(
        lambda shown: shown["command"] is None and shown["commandSaid"] == "nowarmth"
    )
    machine.send_chars(":set warmth\n")
    machine.wait_until_succeeds(
        "grep -q 'BOOLEAN true' /tmp/warmth-heard", timeout=dt.timedelta(seconds=10)
    )
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
    desktop.follow()
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
    side = desktop.showing()["selected"]["side"]
    assert side.split(" \u00b7 ")[0] == "62.0 \u00b0C", side
    desktop.close_launcher()
    # The Processor says what held it back, after its Clock, and each core
    # held, a step in; the core its cluster didn't hold isn't listed.
    desktop.answer("processor")
    desktop.launcher_until(lambda shown: shown["kind"] == "answer")
    desktop.follow()
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
    desktop.follow()
    desktop.launcher_until(lambda shown: shown["rows"] == ["Core 0", "Copy"])
    desktop.close_launcher()
    # What kept it from resting stands beside what held it back, both there
    # whenever the telemetry answers, saying the part that kept it most,
    # and each part that did a step in.
    desktop.answer("processor")
    desktop.launcher_until(lambda shown: shown["kind"] == "answer")
    desktop.follow()
    desktop.launcher_until(
        lambda shown: (
            "Held Back" in shown["rows"]
            and shown["rows"][shown["rows"].index("Held Back") + 1] == "Kept From Resting"
        )
    )
    desktop.type_in("kept", "Kept From Resting")
    desktop.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("side") == "12%"
            and shown["footer"] == "by the display engine"
        )
    )
    desktop.shoot("launcher-kept-from-resting")
    desktop.follow()
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
        desktop.shoot("launcher-kept-from-resting-on-battery")
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
