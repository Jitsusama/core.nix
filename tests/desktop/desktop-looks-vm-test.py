"""The desktop test's looks area.

Here the launcher is laid out at Joel's two scales, on a screen as large as the laptop's, which the
other areas' screen is too small to hold the card and its drawer at: 1.5, as the HP draws it, and 2,
as the laptop does. A session sits a step in under its orchestrator; a meter stands over dictation
rather than on it; AI spend's budgets stand behind one row; and what's found by its meaning sits
below what typing found, each shot for a person to judge. An ErgoDox unplugged goes from the
keyboard's layout and the launcher, and comes back plugged in. The Home Network lays out the eero
daemon's record, and Bluetooth's Nearby hears a device of BlueZ's emulator while it's open, and only
then. The launcher's rows, vi modes, plates and file picker are looks areas of their own
(desktop-looks-*-vm-test.py), so the looks areas run side by side rather than one after another;
what they share is in looks_session.py. tests/desktop.nix runs this through pytest inside the driver
(tests/lib/vm_pytest.py), and its opening comment says what the test proves. Every test asks for the
desktop, started once a run (desktop_session.py), and leaves it as it found it.
"""

from __future__ import annotations

import json
import shlex
import time
from datetime import UTC, datetime, timedelta
from functools import partial
from typing import TYPE_CHECKING, Any, cast

import desktop_session

# pytest finds a fixture by its name in the test's module, so the looks areas' desktop and
# scales are brought here.
from looks_session import (  # noqa: F401
    at_each_scale,
    at_scale,
    changed_from,
    desktop,
    drawn,
    plate_shown,
    pressed,
    things_apart,
)
from PIL import Image

if TYPE_CHECKING:
    from collections.abc import Callable

    from desktop_session import Desktop, Json
    from PIL import Image

# More pages than the internet's drawer takes at either of Joel's scales, so a paging that
# never reaches its end fails rather than going on.
INTERNET_PAGES = 20
# What warp asks warp-cli, each answered by the contract's file of the same name.
WARP_QUESTIONS = ("status", "tunnel-stats", "settings", "dns-stats")


def home_network_plate(desktop: Desktop, scale: str, title: str) -> None:
    """Wait for a thing of the home network's plate, and hold its facts whole and one way."""
    desktop.launcher_until(
        lambda shown: (
            shown["drawer"]["out"] is True
            and shown["drawer"]["title"] == title
            and len(shown["drawer"]["lines"]["facts"]) > 1
        )
    )
    drawer = plate_shown(desktop, scale)
    assert drawer["labelsWhole"], f"at {scale} a fact's name is broken: {drawer}"
    desktop.launcher_until(lambda shown: any(v is not None for v in shown["drawer"]["values"]))
    drawer = desktop.showing()["drawer"]
    assert drawer["valuesWhole"], f"at {scale} a fact's value breaks a word: {drawer}"
    assert drawer["laidOneWay"], f"at {scale} its facts mix beside and under: {drawer}"


def home_network_laid_out(desktop: Desktop, at_each_scale: list[str]) -> None:
    """Hold the Home Network to the eero daemon's record, and each thing's plate whole.

    The record the stand-in serves (tests/desktop.nix) reaches the place: the internet first,
    then the gateway node with its devices under it, and each node after the one it hangs from.
    A node's, a device's and a profile's plates lay their facts out one way, every name and value
    whole, at both of Joel's scales. WARP, beside it in Network, is shot at each scale too.
    """
    desktop.machine.wait_until_succeeds(
        desktop.in_session('test -S "$XDG_RUNTIME_DIR/eero/socket"'),
        timeout=timedelta(seconds=30),
    )
    for scale in at_each_scale:
        at_scale(desktop, scale)
        at = scale.replace(".", "-")
        desktop.answer("home network")
        desktop.launcher_until(
            lambda shown: (shown["selected"] or {}).get("name") == "Home Network"
        )
        desktop.step_in()
        desktop.launcher_until(
            lambda shown: (
                shown["rows"][:4] == ["Internet", "Hall", "generic 3", "phone 1"]
                and "Den" in shown["rows"]
                and shown["rows"].index("Den") < shown["rows"].index("Study")
            )
        )
        home_network_plate(desktop, scale, "Internet")
        desktop.shoot(f"launcher-home-network-at-{at}")
        the_internet_paged_through(desktop, at)
        desktop.down_to("Study")
        home_network_plate(desktop, scale, "Study")
        desktop.shoot(f"launcher-home-network-node-at-{at}")
        desktop.down_to("unknown computer 1")
        home_network_plate(desktop, scale, "unknown computer 1")
        desktop.shoot(f"launcher-home-network-device-at-{at}")
        desktop.down_to("profile 1")
        home_network_plate(desktop, scale, "profile 1")
        desktop.shoot(f"launcher-home-network-profile-at-{at}")
        desktop.close_launcher()
        devices_plates_lie_on_the_lines(desktop, scale)
        the_desk_lights_plate_lies_on_the_lines(desktop, scale)
        the_screens_plate_says_what_sets_it(desktop, scale)
        warp_stands_in_network(desktop, scale)


def the_internet_paged_through(desktop: Desktop, at: str) -> None:
    """Page through the internet's drawer to its last line, a shot a page, and come back.

    It holds every figure eero gives of the network, far more than one view, so each page is
    shot for a person to judge how all of it is laid out (Joel, 2026-11-07: "every figure there
    is, laid out to the design bar"). No page's top cuts a line, and the pages reach its end.
    """
    machine = desktop.machine
    pressed(desktop, "ctrl-w l")
    desktop.launcher_until(lambda shown: shown["drawer"]["keys"])
    page = 1
    while True:
        drawer = desktop.showing()["drawer"]
        assert not drawer["cut"], f"page {page} of the internet's drawer cuts a line: {drawer}"
        cursor = drawer["cursor"]
        if cursor["line"] == cursor["lines"] - 1:
            break
        assert page < INTERNET_PAGES, f"the internet's drawer runs past {page} pages: {drawer}"
        machine.send_key("ctrl-f")
        desktop.launcher_until(partial(paged_on, drawer["scrolled"]))
        page += 1
        desktop.shoot(f"launcher-home-network-internet-{page}-at-{at}")
    pressed(desktop, "ctrl-w h")
    desktop.launcher_until(lambda shown: not shown["drawer"]["keys"])


def paged_on(scrolled: int, shown: Json) -> bool:
    """Say whether the drawer has paged on from where it was scrolled, or stands at its end."""
    drawer = shown["drawer"]
    return bool(
        drawer["scrolled"] != scrolled or drawer["cursor"]["line"] == drawer["cursor"]["lines"] - 1
    )


def devices_plates_lie_on_the_lines(desktop: Desktop, scale: str) -> None:
    """Hold the speakers', the microphone's and the screen's plates to the lines, as a server's.

    Each says which way it goes or what it is over its name, then its parts, what moves most
    first, laid one way, every name whole (VKCF3Z, 2026-11-07). A sound device's State, as
    pipewire says it, leads its level (VKCF3Z, 2026-11-08); the stand-in backlight lights the
    one screen, so its Brightness leads. The speakers are played silence, so their node runs
    and the sound daemon says how it keeps time, from PipeWire's profiler: Dropouts, Load and
    Cycle, in that order (VKCF3Z, 2026-11-09).
    """
    for collection, typed, kind, leading, keeping in [
        ("sound", "speakers", "output", ["State", "Volume"], ["Dropouts", "Load", "Cycle"]),
        ("sound", "microphone", "input", ["State", "Gain"], []),
        ("displays", "", "screen", ["Brightness"], []),
    ]:
        first = leading[-1]
        desktop.launcher_until(lambda shown: not shown["open"])
        if keeping:
            # Silence, as fast as the node plays it, which pw-cat paces.
            desktop.as_account(
                "systemd-run --user --collect --quiet --unit=plays -- sh -c "
                + shlex.quote(
                    "pw-cat --playback --raw --format s16 --rate 48000 --channels 2"
                    f" --target stand-in-{typed} - < /dev/zero"
                )
            )
        desktop.as_account(f"qs ipc call launcher collection {collection}")
        desktop.launcher_ready()
        if typed:
            desktop.type_in(typed, selecting=typed)

        def plated(  # As launcher_until calls it.
            shown: Json, first: str = first, keeping: list[str] = keeping
        ) -> bool:
            """Say whether the drawer is out, holding this plate's facts, laid.

            A busy VM has reported a plate out before its facts were placed, all on one line,
            and a fact that arrives late, as Curve does when the backlight daemon writes its
            curve, lays them out again; so the plate is waited for laid, on the pitch, every
            name whole and one way, rather than held to it in a later snapshot. A playing
            device's timing comes a second after its drawer is out, once the profiler's first second
            of cycles stands.
            """
            drawer = shown["drawer"]
            return (
                drawer["out"] is True
                and first in drawer["facts"]
                and all(fact in drawer["facts"] for fact in keeping)
                and things_apart(drawer["lines"]["facts"], wrapped=drawer["factsWrapped"] > 0)
                and drawer["labelsWhole"] is True
                and drawer["laidOneWay"] is True
            )

        desktop.launcher_until(plated)
        drawer = plate_shown(desktop, scale)
        assert str(drawer["eyebrow"]).lower().endswith(kind), f"at {scale}: {drawer}"
        assert drawer["facts"][: len(leading)] == leading, (
            f"at {scale} {typed or kind} leads wrong: {drawer}"
        )
        kept = [fact for fact in drawer["facts"] if fact in keeping]
        assert kept == keeping, f"at {scale} {typed}'s timing is out of order: {drawer}"
        desktop.shoot(f"launcher-plate-{typed or 'screen'}-at-{scale.replace('.', '-')}")
        if keeping:
            a_programs_plate_lies_on_the_lines(desktop, scale, keeping)
        desktop.close_launcher()
        if keeping:
            desktop.as_account("systemctl --user stop plays")


def a_programs_plate_lies_on_the_lines(desktop: Desktop, scale: str, keeping: list[str]) -> None:
    """Hold the plate of the program playing through the speakers to the lines.

    pw-cat plays them silence, so its stream runs in their cycle and says how it keeps time as
    theirs does, and what identifies it ends its plate. It's named as it names itself, and its
    eyebrow says where it plays and that it's a stream (VKCF3Z, 2026-11-09).
    """
    desktop.follow()
    desktop.down_to("Playing")
    desktop.follow()
    desktop.down_to("pw-cat")

    def plated(shown: Json) -> bool:  # As launcher_until calls it.
        """Say whether pw-cat's plate is out, its timing heard, laid on the pitch."""
        drawer = shown["drawer"]
        return (
            drawer["out"] is True
            and drawer["title"] == "pw-cat"
            and all(fact in drawer["facts"] for fact in keeping)
            and things_apart(drawer["lines"]["facts"], wrapped=drawer["factsWrapped"] > 0)
            and drawer["labelsWhole"] is True
            and drawer["laidOneWay"] is True
        )

    desktop.launcher_until(plated)
    drawer = plate_shown(desktop, scale)
    eyebrow = str(drawer["eyebrow"])
    assert eyebrow.startswith("on "), f"at {scale} it doesn't say where it plays: {drawer}"
    assert eyebrow.endswith(" \u00b7 stream"), f"at {scale} it doesn't say it's a stream: {drawer}"
    kept = [fact for fact in drawer["facts"] if fact in keeping]
    assert kept == keeping, f"at {scale} pw-cat's timing is out of order: {drawer}"
    assert "Graph ID" in drawer["facts"], f"at {scale} pw-cat isn't identified: {drawer}"
    at = scale.replace(".", "-")
    desktop.shoot(f"launcher-plate-program-at-{at}")
    # Its part says where it plays, and steps in to where else it could:
    # the default first, then each output, the one it's on chosen (VKCF3Z,
    # 2026-11-09).
    desktop.follow()
    desktop.launcher_until(lambda shown: "Plays On" in shown["rows"])
    desktop.down_to("Plays On")
    desktop.shoot(f"launcher-program-plays-on-at-{at}")
    desktop.step_in("ret")
    desktop.launcher_until(
        lambda shown: (
            shown["rows"][:1] == ["the default"] and "stand-in-headphones" in shown["rows"]
        )
    )
    desktop.shoot(f"launcher-program-plays-on-choices-at-{at}")


def the_desk_lights_plate_lies_on_the_lines(desktop: Desktop, scale: str) -> None:
    """Hold the desk light's plate to the lines, as a sound device's.

    A stand-in Litra Glow (desk-light-stand-in.py) is plugged in, lit as a Glow first is, and
    the desk light answer's drawer says where it is and what it is over the light's own name,
    then its output in lumens of its range, its HID device and its serial (VKCF3Z, 2026-11-08).
    It's unplugged again after, so no later shot holds a light that test order put there.
    systemd's hwdb opens a Litra to the seat by its USB parent, which a uhid stand-in hasn't, so
    its node is given to the account here, as udev gives a real one; `litra watch` tries again
    when it is.
    """
    machine = desktop.machine
    light = desktop.values["desk_light"]
    machine.succeed(
        "systemctl is-active --quiet desk-light"
        f" || systemd-run --unit=desk-light {light} litra /run/desk-light.log"
    )
    try:
        found = machine.wait_until_succeeds(
            "grep -l HID_ID=0003:0000046D:0000C900 /sys/class/hidraw/*/device/uevent",
            timeout=timedelta(seconds=30),
        )
        node = found.split("/")[4]
        machine.succeed(f"chown {desktop.account} /dev/{node}")
        desktop.answer("desk light")
        desktop.launcher_until(
            lambda shown: (
                shown["drawer"]["out"] is True
                and "Output" in shown["drawer"]["facts"]
                and things_apart(
                    shown["drawer"]["lines"]["facts"], wrapped=shown["drawer"]["factsWrapped"] > 0
                )
            )
        )
        drawer = plate_shown(desktop, scale)
        assert drawer["eyebrow"] == "on USB \u00b7 desk light", f"at {scale}: {drawer}"
        assert drawer["facts"] == ["Output", "HID Device", "Serial"], f"at {scale}: {drawer}"
        assert things_apart(drawer["lines"]["facts"], wrapped=drawer["factsWrapped"] > 0), (
            f"at {scale} the desk light's facts are off the pitch: {drawer}"
        )
        assert drawer["labelsWhole"], f"at {scale} a fact's name is broken: {drawer}"
        assert drawer["laidOneWay"], f"at {scale} its facts mix beside and under: {drawer}"
        desktop.shoot(f"launcher-plate-desk-light-at-{scale.replace('.', '-')}")
        desktop.close_launcher()
    finally:
        machine.succeed("systemctl stop desk-light")


def the_screens_plate_says_what_sets_it(desktop: Desktop, scale: str) -> None:
    """Hold the screen's plate to the lines not following the room, then following it.

    The backlight daemon says what it settled, whole by a rename, in backlight-now, and keeps
    its curve in learned; the VM's stand-in backlight and sensor give it no real figures, so
    both are written here as it writes them, the daemon stopped so it writes neither over
    them, and started again after. The stand-in has no lid to shut, so the shot is named for
    what it shows: not following, Follows is gone and the Room Light still says the
    stand-in sensor's live reading; following the room on its way to
    what it asks, with a curve, it says both, the Curve last of the light, every point
    dimmest first (VKCF3Z, 2026-11-08).
    """
    now = "/run/user/1000/backlight-now"
    learned = "~/.local/state/backlight/learned"

    def settled(said: str) -> None:
        """Say what the daemon settled, as it does."""
        desktop.as_account(f"printf '{said}' > {now}.writing && mv {now}.writing {now}")

    def shown_with(labels: list[str], missing: str) -> Callable[[Json], bool]:
        """Give a test that the plate is out and laid, saying these and not the one missing."""

        def laid(shown: Json) -> bool:  # As launcher_until calls it.
            """Say whether the plate says these facts, laid on the pitch, whole, one way."""
            drawer = shown["drawer"]
            return (
                drawer["out"] is True
                and all(label in drawer["facts"] for label in labels)
                and missing not in drawer["facts"]
                and things_apart(drawer["lines"]["facts"], wrapped=drawer["factsWrapped"] > 0)
                and drawer["labelsWhole"] is True
                and drawer["laidOneWay"] is True
            )

        return laid

    at = scale.replace(".", "-")
    desktop.as_account("systemctl --user stop backlight.service")
    try:
        desktop.as_account(f"rm -f {learned}")
        settled("level\\t0.600\\nlux\\t28\\nasks\\t0.950\\nfollowing\\tno\\n")
        desktop.launcher_until(lambda shown: not shown["open"])
        desktop.as_account("qs ipc call launcher collection displays")
        desktop.launcher_ready()
        desktop.launcher_until(shown_with(["Brightness", "Room Light"], "Follows"))
        drawer = plate_shown(desktop, scale)
        room = drawer["values"][drawer["facts"].index("Room Light")]
        assert room.endswith(" lx"), f"at {scale}: {drawer}"
        desktop.shoot(f"launcher-plate-screen-not-following-at-{at}")
        curve = "0 0.150\\n6 0.377\\n28 0.950\\n10000 1.000\\n"
        desktop.as_account(
            f"mkdir -p $(dirname {learned}) && printf '{curve}'"
            f" > {learned}.writing && mv {learned}.writing {learned}"
        )
        settled("level\\t0.600\\nlux\\t28\\nasks\\t0.950\\nfollowing\\tyes\\n")
        # The machine daemon's say of the stand-ins too: what drives the
        # backlight, the sensor's count, and both their names (VKCF3Z,
        # 2026-11-09).
        light = ["Driven By", "Raw", "Per Count", "Device", "Sensor"]
        desktop.launcher_until(shown_with(["Brightness", "Follows", "Curve", *light], ""))
        drawer = plate_shown(desktop, scale)
        facts = drawer["facts"]
        assert facts.index("Follows") < facts.index("Curve"), f"at {scale}: {drawer}"
        said = dict(zip(facts, drawer["values"], strict=False))
        assert (said["Driven By"], said["Follows"]) == (
            "the graphics chip",
            "the room \u00b7 on its way to 95%",
        ), f"at {scale}: {drawer}"
        desktop.shoot(f"launcher-plate-screen-following-at-{at}")
        # Its settings' choices, each by its value alone: Rotation's four
        # turns then four mirrored, and Scale's multiples (VKCF3Z, 2026-11-09).
        desktop.follow()
        for setting, choices in [
            (
                "Rotation",
                [
                    "upright",
                    "a quarter turn left",
                    "upside down",
                    "a quarter turn right",
                    "mirrored",
                    "mirrored, a quarter turn left",
                    "mirrored, upside down",
                    "mirrored, a quarter turn right",
                ],
            ),
            ("Scale", ["1\u00d7", "1.25\u00d7", "1.5\u00d7"]),
        ]:
            desktop.down_to(setting)
            desktop.step_in("ret")

            def offered(shown: Json, choices: list[str] = choices) -> bool:  # As called.
                """Say whether the setting offers these choices in order, Return saying set."""
                told = (shown["selected"] or {}).get("told", [])
                return [row for row in shown["rows"] if row in choices] == choices and (
                    "\u21b5 set" in told
                )

            desktop.launcher_until(offered)
            desktop.shoot(f"launcher-screen-{setting.lower()}-at-{at}")
            desktop.step_out()
        desktop.close_launcher()
    finally:
        desktop.as_account(f"rm -f {now} {learned}")
        desktop.as_account("systemctl --user start backlight.service")


def reading(shown: Json, start: str) -> str:
    """Give how the first row starting so reads whole, or nothing."""
    reads = [str(thing["reads"]) for thing in shown["things"]]
    return next((read for read in reads if read.startswith(start)), "")


def warp_error_drawn(shown: Json) -> bool:
    """Whether WARP's drawer is out, warp-cli's error as written its last fact."""
    drawer = shown["drawer"]
    return (
        bool(drawer["shown"])
        and drawer["title"] == "Work Network"
        and drawer["facts"][-1:] == ["Error"]
    )


def warp_answers(desktop: Desktop, *answers: tuple[str, str]) -> None:
    """Have the stand-in warp-cli answer as the contract's files say, each (file, question).

    It answers from what's written in ~/warp-cli (tests/warp-cli-stand-in.sh); a question
    named with .error is said as warp-cli says an error. Each lands whole, by a rename, so
    warp never reads half of one, and writable, since mv and rm on a terminal ask before
    replacing a file that isn't, as the store's are.
    """
    contract = desktop.values["warp_contract"]
    desktop.as_account(
        "mkdir -p ~/warp-cli && "
        + " && ".join(
            f"install -m 644 {contract}/{file} ~/warp-cli/.next"
            f" && mv -f ~/warp-cli/.next ~/warp-cli/{asked}"
            for file, asked in answers
        )
    )


def warp_opened_again(desktop: Desktop, state: str) -> None:
    """Step out of the Work Network to Network, wait until its row says this, and step in."""
    desktop.step_out()
    desktop.launcher_until(
        lambda shown: reading(shown, "Work Network ").startswith(f"Work Network \u00b7 {state}")
    )
    desktop.down_to("Work Network")
    desktop.step_in()
    desktop.launcher_until(lambda shown: "State" in shown["rows"])


def warp_stands_in_network(desktop: Desktop, scale: str) -> None:
    """Shoot WARP in Network, a step in with a drawer open, held, down, degraded and silent.

    The shell runs the real warp against the stand-in warp-cli, answering with the contract's
    made-up files (modules/warp/contract), which warp's own tests read too. What each part says
    is tests/answers.qml's to prove; these are for the design's look at Joel's scales (VKCF3Z,
    2026-11-08).
    """
    at = scale.replace(".", "-")
    connected = [(f"{asked}.json", f"{asked}.json") for asked in WARP_QUESTIONS]
    try:
        warp_answers(desktop, *connected)
        desktop.answer("network")
        desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Network")
        desktop.step_in()
        desktop.launcher_until(
            lambda shown: reading(shown, "Work Network ").startswith(
                "Work Network \u00b7 connected"
            )
        )
        desktop.down_to("Work Network")
        desktop.shoot(f"launcher-warp-in-network-at-{at}")
        desktop.step_in()
        desktop.launcher_until(lambda shown: "Round Trip" in shown["rows"])
        desktop.shoot(f"launcher-warp-at-{at}")
        # Read again while shown, Round Trip sparks, its drawer saying once whose
        # figures they are.
        desktop.down_to("Round Trip")
        desktop.launcher_until(
            lambda shown: (
                shown["drawer"]["out"] is True
                and shown["drawer"]["title"] == "Round Trip"
                and shown["drawer"]["eyebrow"] == "via WARP \u00b7 tunnel"
            )
        )
        desktop.shoot(f"launcher-warp-round-trip-at-{at}")
        # Held by the organisation, its switch says who holds it, selected.
        desktop.as_account(
            'sed -i \'s/"switch_locked": false/"switch_locked": true/\' ~/warp-cli/settings.json'
        )
        desktop.down_to("Disconnect WARP")
        desktop.launcher_until(lambda shown: "holds the switch" in shown["footer"])
        desktop.shoot(f"launcher-warp-switch-held-at-{at}")
        # Taken down, as the stand-in says it after a disconnect, its switch brings
        # it up, and the tunnel it no longer has says nothing.
        warp_answers(desktop, ("settings.json", "settings.json"))
        desktop.as_account(
            'rm -f ~/warp-cli/tunnel-stats.json && echo \'{"status": "Disconnected",'
            ' "reason": "ManualDisconnection"}\' > ~/warp-cli/.next'
            " && mv -f ~/warp-cli/.next ~/warp-cli/status.json"
        )
        desktop.launcher_until(lambda shown: "Connect WARP" in shown["rows"])
        # Rows WARP no longer gives stay while the place is shown, so it's opened
        # again to show only what WARP says once down.
        warp_opened_again(desktop, "disconnected")
        desktop.launcher_until(lambda shown: "Round Trip" not in shown["rows"])
        desktop.shoot(f"launcher-warp-disconnected-at-{at}")
        warp_answers(desktop, *connected)
        warp_opened_again(desktop, "connected")
        desktop.launcher_until(lambda shown: "Round Trip" in shown["rows"])
        # WARP finds the network degraded, which its Health says a step in, and its
        # row in Network, in the warning colour.
        warp_answers(desktop, ("status.degraded.json", "status.json"))
        desktop.launcher_until(
            lambda shown: reading(shown, "Health ").startswith("Health \u00b7 degraded")
        )
        # Selected, so its row says what WARP measured.
        desktop.down_to("Health")
        desktop.shoot(f"launcher-warp-degraded-at-{at}")
        desktop.step_out()
        desktop.launcher_until(
            lambda shown: reading(shown, "Work Network ").startswith("Work Network \u00b7 degraded")
        )
        desktop.shoot(f"launcher-warp-degraded-in-network-at-{at}")
        # Its service gone, WARP says so in its place, and why beside it, its drawer
        # out with warp-cli's error as written once the row has rested (VKCF3Z,
        # 2026-11-09).
        desktop.as_account("rm -f ~/warp-cli/status.json")
        warp_answers(desktop, ("daemon-down.json", "status.error.json"))
        desktop.launcher_until(
            lambda shown: (
                reading(shown, "Work Network ").startswith("Work Network \u00b7 not answering")
                and warp_error_drawn(shown)
            )
        )
        desktop.shoot(f"launcher-warp-not-answering-in-network-at-{at}")
        desktop.close_launcher()
        desktop.answer("vpn")
        desktop.launcher_until(
            lambda shown: (
                (shown["selected"] or {}).get("name") == "not answering" and warp_error_drawn(shown)
            )
        )
        desktop.shoot(f"launcher-warp-not-answering-at-{at}")
        desktop.close_launcher()
    finally:
        desktop.as_account("rm -rf ~/warp-cli")


# A session sits under the one orchestrating it, a step in, its glyph where its orchestrator's
# name starts, with no line, box or fold; typed, every row is at no depth, whom it's under
# leading its note; and an orchestrator says what its whole orchestration spent today
# (QEST-20261026-15J7N3's agents-tree-look.md). The orchestrator listens while an owner under it
# runs a builder that works, so the orchestration stands with those working: the owner, its
# builder, and one paused beside the builder. The two others under the orchestrator only listen,
# so they fold into one row after its own (VKCF3Z, 2026-11-06), while the owner's one paused
# stays a row, since folding one saves none.
ORCHESTRATION = [
    # address, title, whom it's under, how it stands
    ("TESTQA", "A quest of its own & whole", None, "listening"),
    ("TESTQB", "The Other One Under It", "TESTQA", "listening"),
    ("TESTQC", "An Owner Under It", "TESTQA", "listening"),
    ("TESTQD", "A Builder Under the Owner", "TESTQC", "working"),
    ("TESTQE", "One Put Down Under It", "TESTQC", "paused"),
    ("TESTQF", "One More Under It", "TESTQA", "listening"),
]
# The fold's row, by what it says, and the sessions it holds.
FOLD = "2 listening"
FOLDED = ["The Other One Under It", "One More Under It"]
TREE = {"TESTQA": 0, "TESTQC": 1, "TESTQD": 2, "TESTQE": 2, FOLD: 1}
SPENT = {
    "v": 2,
    "sessions": {"01a1face-test": "$3.10", "01a1face-qd": "$5.10"},
    # How the builder's day went, its hours from midnight on the day's largest one.
    "sessionsToday": {
        "01a1face-qd": {
            "graph": {
                "shape": "spark",
                "values": [0, 0, 0, 0, 0, 0, 0, 0, 0.1, 0.4, 0.2, 0.7, 1, 0.5, 0.3],
            },
            "rebuilt": "$1.20",
        }
    },
    "helpers": {},
    "families": {
        "QEST-20261022-TESTQA": {"all": "$14.20", "own": "$3.10", "under": "$11.10"},
        "QEST-20261022-LAMPLT": {"all": "$2.40", "own": "$2.40", "under": "$0.00"},
    },
}
# Orchestrators no session runs, as the harness lists them, each with when it was last live:
# they stand dimmed at the tree's end, the one live last first, and only one that spent today
# says what all of it spent (VKCF3Z, 2026-11-07 [9a4b1dd0]).
CLOSED = [
    # address, its short name, its quest's title, when it was last live
    ("BEACON", "Beacon", "Light the beacon", "2026-11-06T22:00:00Z"),
    ("LAMPLT", "Lamplight", "Keep the lamp lit", "2026-11-07T04:00:00Z"),
]
# What the owner under the orchestrator landed, newest first, as the harness keeps it: a change
# with its page, a plan item ticked alone, which Landed leaves out, and a change saying what to
# try, which waits in For You until it's read (VKCF3Z, 2026-11-10).
STOPPED = "Stop Waits for a Second Return"
CLOSING = "A Session Closes From the Launcher"
LANDINGS = [
    {"at": "2026-11-07T11:31:00Z", "ref": "core.nix#2140", "what": STOPPED,
     "url": "https://example.org/core.nix/pull/2140"},
    {"at": "2026-11-07T11:25:00Z", "ref": "PLAN-20261022-TESTQC#2", "what": "Ticked alone"},
    {"at": "2026-11-07T11:19:00Z", "ref": "core.nix#2138", "what": CLOSING,
     "person": "Press dd on a session to close it"},
]  # fmt: skip
# How full the builder's context is and what its turns took, as the harness's usage keeps them
# by gen_ai's names; today's counts show only on the day they're of.
FULL = {
    "gen_ai.response.model": "claude-opus-5-5",
    "last": {"gen_ai.usage.output_tokens": 4100},
    "context": {"used": 842000, "limit": 1000000},
    "turns": {"last": {"duration_ms": 38000, "first_token_ms": 900}},
}
# What the builder's record says of its work, as pi says it: its step, its quest, and what it
# works on, the quest's title and then a summary as long as every real session's.
BUILDER = {
    "stepDetail": {"kind": "tool", "name": "edit", "detail": "/home/jitsusama/tree/Drawer.qml"},
    "questInfo": {"id": "QEST-20261022-TESTQD", "title": "A Builder Under the Owner",
                  "short": "TESTQD", "status": "active"},
    "worksOn": "A Builder Under the Owner: A builder under the owner's plan, which keeps the"
    " launcher's agents quiet, told and easy to read: every session says how it stands, what it"
    " does, what it spent and what it needs from Joel, from the node, with nothing guessed and"
    " nothing said twice.",
}  # fmt: skip
# A quest nothing orchestrates and no session has, whose step in offers to orchestrate it.
UNSTARTED = "Chart the harbour"
WORKFLOW = "~/.local/state/pi/agentic-harness.pi/quest-workflow"


def window_of(address: str, title: str) -> str:
    """Give the title of a session's window, as its label: its quest's name and its folder."""
    return f"\u03c0 - {title} - {address.lower()}-tree"


def orchestrated(desktop: Desktop) -> None:
    """Start the orchestration's sessions, each in a window of its own, saying whom it's under."""
    desktop.pi_session()
    for address, title, under, stands in ORCHESTRATION:
        if address != "TESTQA":
            desktop.pi_window(address, window_of(address, title))
        desktop.presence_says(record_of(address, under, stands, LANDINGS), address)
    desktop.as_account(
        f"mkdir -p $XDG_RUNTIME_DIR/spend-watch && echo '{json.dumps(SPENT)}'"
        " > $XDG_RUNTIME_DIR/spend-watch/.c"
        " && mv $XDG_RUNTIME_DIR/spend-watch/.c $XDG_RUNTIME_DIR/spend-watch/collection.json"
    )
    closed_listed(desktop)


def record_of(address: str, under: str | None, stands: str, landings: list[Json]) -> Json:
    """Give an orchestration's session as its record says it, the owner's with its landings."""
    return {
        "session": ("01a1face-test" if address == "TESTQA" else f"01a1face-{address[-2:].lower()}"),
        "addresses": [address],
        "busy": stands == "working",
        "state": "busy" if stands == "working" else stands,
        "owns": [f"QEST-20261022-{address}"],
        "orchestratedBy": under,
        # The orchestrator's record names its repository, which its drawer's eyebrow says as
        # it's written.
        **({"repo": "core.nix"} if address == "TESTQA" else {}),
        **({"landings": landings} if address == "TESTQC" else {}),
        **({"usage": FULL, **BUILDER} if address == "TESTQD" else {}),
    }


def closed_listed(desktop: Desktop) -> None:
    """List the closed orchestrators, as the harness does, and a quest nothing has started."""
    listed = {
        "orchestrators": [
            {"quest": f"QEST-20261022-{address}", "address": address, "short": short,
             "title": title}
            for address, short, title, _ in CLOSED
        ]
    }  # fmt: skip
    quests = [
        desktop_session.listed_quest(
            f"QEST-20261022-{address}",
            title,
            priority="active",
            resume={"id": f"01a1-{address.lower()}", "cwd": "/home/jitsusama", "at": at},
        )
        for address, _, title, at in CLOSED
    ]
    unstarted = desktop_session.listed_quest("QEST-20261022-HARBOR", UNSTARTED, priority="queued")
    desktop_session.quests_listed(desktop, [*quests, unstarted])
    desktop.as_account(
        f"echo '{json.dumps(listed)}' > {WORKFLOW}/.o"
        f" && mv {WORKFLOW}/.o {WORKFLOW}/orchestrators.json"
    )


def unorchestrated(desktop: Desktop) -> None:
    """Close the sessions the orchestration started, and forget what they spent."""
    for address, title, _, _ in ORCHESTRATION[1:]:
        window = desktop.window_titled(window_of(address, title))
        desktop.as_account(f"niri msg action close-window --id {window['id']}")
        desktop.as_account(f"rm -f {desktop_session.PI_PRESENCE}/{address.lower()}.json")
    desktop.as_account(
        "rm -f $XDG_RUNTIME_DIR/spend-watch/collection.json"
        f" {WORKFLOW}/orchestrators.json {WORKFLOW}/quests.json {WORKFLOW}/ended.json"
    )
    desktop.presence_says({"addresses": ["TESTQA"], "busy": True})


def address_of(row: Json) -> str:
    """Say which of the orchestration's sessions a drawn row is, by its title.

    A session's row says its title, how it stands and what it spent; its address is its
    drawer's. The fold's row is named by what it says.
    """
    if FOLD in row["reads"]:
        return FOLD
    return next(address for address, title, _, _ in ORCHESTRATION if title in row["reads"])


def in_orchestration(row: Json) -> bool:
    """Say whether a drawn row is one of the orchestration's sessions, or its fold."""
    reads = row["reads"]
    return FOLD in reads or any(title in reads for _, title, _, _ in ORCHESTRATION)


def orchestration_drawn(desktop: Desktop) -> list[Json]:
    """Give the orchestration's rows as they're drawn, once two reads in a row agree."""
    was = None
    for _ in range(20):
        laid = json.loads(desktop.as_account("qs ipc call launcher laidOut"))
        now = [row for row in laid["rows"] if in_orchestration(row)]
        if now == was:
            return now
        was = now
    return cast("list[Json]", was)


def side_gives_way_whole(row: Json, title: str, said: str, scale: str) -> None:
    """Hold a row's side to whole items, its name whole while the side says more than one."""
    name = next(word for word in row["words"] if word["text"] == title)
    # What a row says at its right is the word drawn furthest right.
    side = max(row["words"], key=lambda word: word["x"])
    assert not side["cut"], f"at {scale} {title}'s side is cut: {side}"
    assert said in side["text"], f"at {scale} {title}'s side dropped {said!r}: {side}"
    assert not name["cut"] or " \u00b7 " not in side["text"], (
        f"at {scale} {title}'s name is cut while its side says {side['text']!r}"
    )


def into_agents(desktop: Desktop) -> None:
    """Open the launcher into Agents, once it lists every session of the orchestration."""
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("agents")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Agents")
    desktop.machine.send_key("ret")
    desktop.launcher_until(
        lambda shown: (
            shown["held"] == ["agents"]
            and sum(in_orchestration(thing) for thing in shown["things"]) == len(TREE)
        )
    )


def orchestrator_says_whose_spend_is_whose(desktop: Desktop) -> None:
    """Inside the orchestrator, what all of it spent, then its own and its sessions'."""
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("testqa", selecting="A quest of its own & whole")
    desktop.follow()
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Go To")
    for name, side in [
        ("Spent Today", "$14.20 in all"),
        ("Its Own", "$3.10"),
        ("Its Sessions", "$11.10"),
    ]:
        desktop.down_to(name)
        assert desktop.showing()["selected"]["side"] == side, desktop.showing()["selected"]
    desktop.close_launcher()


def orchestrators_drawer_names_its_repo(desktop: Desktop, title: str, shot: str) -> None:
    """Shoot the orchestrator's drawer, its eyebrow naming its repository as written.

    The eyebrow says its address, its repository and when it started, set as they're written,
    never in capitals, since they're names from the world (VKCF3Z, 2026-11-06). Short of room,
    as the drawer is at its least width, it gives way by whole items from its end, never cut
    mid-fact (VKCF3Z, 2026-11-07).
    """
    desktop.down_to(title)
    desktop.launcher_until(
        lambda shown: (
            shown["drawer"]["shown"]
            and shown["drawer"]["eyebrow"].startswith("TESTQA \u00b7 core.nix \u00b7 ")
            and shown["drawer"]["eyebrowNames"]
        )
    )
    drawer = desktop.showing()["drawer"]
    whole = drawer["eyebrow"]
    assert drawer["eyebrowDrawn"] in {whole, "TESTQA \u00b7 core.nix", "TESTQA"}, (
        f"at {shot} the eyebrow is cut mid-fact: {drawer}"
    )
    desktop.shoot(f"launcher-agents-drawer-at-{shot}")


def figures_in_the_drawer(desktop: Desktop, title: str, shot: str) -> None:
    """Shoot a session past 80% full, its drawer giving its figures by gen_ai's names."""
    desktop.down_to(title)
    # Its step leads, its figures follow what it spent, then its quest and machine, and what
    # its quest is about, long, comes last, so the first screen holds the rest (Joel and
    # VKCF3Z, 2026-11-07).
    first = ["Doing", "Spent Today", "Context", "Last Turn", "Model"]
    later = ["Context Rebuilt", "Quest", "Machine", "Working On"]
    desktop.launcher_until(
        lambda shown: (
            shown["drawer"]["shown"]
            and shown["drawer"]["facts"][: len(first)] == first
            and shown["drawer"]["facts"][-len(later) :] == later
        )
    )
    side = desktop.showing()["selected"]["side"]
    assert side.startswith("84% full \u00b7 working"), f"at {shot}: {side}"
    # Its note says what its quest is about, never its title, the row's own name, again.
    note = desktop.showing()["selected"]["note"]
    assert note.startswith("A builder under"), f"at {shot}: {note}"
    # Context draws Meter's stroke after its value, at Joel's scales both.
    drawer = desktop.showing()["drawer"]
    drawn = dict(zip(drawer["drawnFacts"], drawer["figures"], strict=True))
    assert drawn["Context"] == "stroke", f"at {shot}: {drawer}"
    # Its spend today carries its hours as Graph's spark.
    assert drawn["Spent Today"] == "spark", f"at {shot}: {drawer}"
    assert drawer["laidOneWay"], f"at {shot} its facts mix beside and under: {drawer}"
    # How it stands is its row's, so Doing says its step alone, and its quest, active, its id.
    assert drawer["values"][0] == "edit Drawer.qml", f"at {shot}: {drawer}"
    assert drawer["values"][-3] == "QEST-20261022-TESTQD", f"at {shot}: {drawer}"
    # What it's doing, its spend and how full it is are on its first screen, even this
    # short one's, before any it says are more.
    said = str(drawer.get("moreSays") or "")
    hidden = int(said.split(maxsplit=1)[0]) if said else 0
    led = first[: first.index("Context") + 1]
    assert len(drawer["facts"]) - hidden >= len(led), f"at {shot} it hides {said}: {drawer}"
    desktop.shoot(f"launcher-agents-figures-at-{shot}")


def returns_said_in_the_foot(desktop: Desktop, titles: dict[str, str], shot: str) -> None:
    """Shoot the foot on a working, a listening and a paused row, then one's acts.

    Return does each state's harmless act, which the foot says before it's pressed
    (VKCF3Z, 2026-11-06): talk to one listening, resume one paused. A step in holds every
    act in its fixed order, whatever Return does. From the find, a Return that changes
    something only lands first, and its foot names the act it leads to all the same
    (VKCF3Z, 2026-11-07), so the paused row's foot is kept there too.
    """
    desktop.down_to(titles["TESTQE"])
    desktop.launcher_until(
        lambda shown: (shown["selected"] or {}).get("told", [])[:1] == ["\u21b5 resume"]
    )
    desktop.shoot(f"launcher-agents-returns-paused-found-at-{shot}")
    desktop.normal()
    feet = [("TESTQD", "working", "\u21b5 go to"), ("TESTQC", "listening", "\u21b5 talk")]
    for address, stands, said in [*feet, ("TESTQE", "paused", "\u21b5 resume")]:
        desktop.down_to(titles[address])

        def foot_says(shown: Json, said: str = said) -> bool:
            """Say whether the selected row's foot says Return's act."""
            told = (shown["selected"] or {}).get("told", [])
            return any(str(one).startswith(said) for one in told)

        desktop.launcher_until(foot_says)
        desktop.shoot(f"launcher-agents-returns-{stands}-at-{shot}")
    desktop.down_to(titles["TESTQC"])
    desktop.follow()
    desktop.launcher_until(lambda shown: "Talk" in shown["rows"][:2])
    desktop.shoot(f"launcher-agents-acts-at-{shot}")
    desktop.step_out()


def fold_steps_in(desktop: Desktop, shot: str) -> None:
    """Shoot the fold's row, then what Return steps in to: each session it holds."""
    desktop.down_to(FOLD)
    desktop.shoot(f"launcher-agents-fold-at-{shot}")
    desktop.machine.send_key("ret")
    desktop.launcher_until(
        lambda shown: (
            shown["held"][-1:] == [FOLD] and all(title in shown["rows"] for title in FOLDED)
        )
    )
    desktop.shoot(f"launcher-agents-fold-inside-at-{shot}")
    desktop.step_out()


def closed_stand_last(desktop: Desktop, shot: str) -> None:
    """Shoot the closed orchestrators at the tree's end, one on the cursor, one at rest.

    Each is dimmed, says it's closed, and what all of it spent only where it spent today;
    Return reopens it.
    """
    desktop.down_to("Lamplight")
    desktop.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("side") == "closed \u00b7 $2.40 in all"
            and shown["selected"]["hue"] == "muted"
        )
    )
    rows = desktop.showing()["rows"]
    assert rows.index("Lamplight") + 1 == rows.index("Beacon"), f"at {shot}: {rows}"
    desktop.shoot(f"launcher-agents-closed-at-{shot}")


def close_asks_what_stays(desktop: Desktop, title: str, address: str, shot: str) -> None:
    """Shoot a session's Close on the cursor, then its ask, which says what stays.

    Close ends the session, so Return puts it on the command line and a fresh Return asks,
    naming the session by its address and what of it stays (VKCF3Z, 2026-11-06); n leaves
    it running.
    """
    desktop.down_to(title)
    desktop.follow()
    desktop.launcher_until(lambda shown: "Close" in shown["rows"])
    desktop.down_to("Close")
    desktop.shoot(f"launcher-agents-close-at-{shot}")
    # The second Return waits out the first's, which the launcher takes for the same press
    # doubled while it's fresh, and so runs nothing.
    desktop.line("close")
    asked = f"Close {address}? Its folder stays. (y/n)"
    with desktop.told("ask ready"), desktop.told(f"asked {asked}"):
        desktop.machine.send_key("ret")
    # The ask takes the whole head, so what it closes is read, its path given way.
    shown = desktop.showing()
    drawn = shown["questionShown"]
    assert drawn.startswith(f"Close {address}?"), f"at {shot} the ask lost its subject: {drawn}"
    assert drawn.endswith("(y/n)"), f"at {shot} the ask lost its keys: {drawn}"
    assert shown["questionWhole"], f"at {shot} the ask was cut: {drawn}"
    desktop.shoot(f"launcher-agents-close-asks-at-{shot}")
    with desktop.told("answered no"):
        desktop.machine.send_chars("n")
    desktop.launcher_until(lambda shown: shown["question"] == "")
    desktop.step_out()


def landed_lists_what_sessions_landed(desktop: Desktop, shot: str) -> None:
    """Shoot For You's one landing to try, Landed, then a landing's drawer, then History.

    Each says what landed as its name and when it landed at its right; its drawer says who
    landed it, when, its ref last, and what to try, a sentence the row has no room for. Landed says
    how many landed today at its side; what's to read is For You's alone to count. The one
    saying what to try lands afresh at each scale, so each finds it unread.
    """
    # What to try waits a day from when it landed by the machine's clock, so a landing dated
    # once goes stale the next day. This one lands in the last minute of yesterday, in the
    # machine's time zone, UTC: within a day, yet not today, so Landed still has none today.
    # Only from 23:59 is it a day old. A second for the scale's figures, :15 at 1.5, :02 at 2.
    yesterday = (datetime.now(UTC) - timedelta(days=1)).date().isoformat()
    fresh = {**LANDINGS[2], "at": f"{yesterday}T23:59:{int(shot.replace('-', '')):02}Z"}
    landings = [*LANDINGS[:2], fresh]
    desktop.presence_says(record_of("TESTQC", "TESTQA", "listening", landings), "TESTQC")
    desktop.close_launcher()
    desktop.machine.send_key("meta_l-o")
    desktop.in_now("For You")
    desktop.step_in()
    desktop.launcher_until(lambda shown: "To Try" in shown["rows"])
    desktop.down_to("To Try")
    desktop.shoot(f"launcher-for-you-to-try-at-{shot}")
    desktop.step_in()
    desktop.launcher_until(lambda shown: shown["rows"] == [CLOSING])
    desktop.launcher_until(
        lambda shown: (
            shown["drawer"]["out"] is True and shown["drawer"]["body"] == LANDINGS[2]["person"]
        )
    )
    desktop.shoot(f"launcher-for-you-landing-at-{shot}")
    # Return steps into it where it lives, Agents \u203a Landed, and reads it.
    desktop.machine.send_key("ret")
    desktop.launcher_until(lambda shown: shown["held"] == ["agents", "Landed", CLOSING])
    desktop.step_out()
    desktop.step_out()
    desktop.down_to("Landed")
    assert desktop.showing()["selected"]["side"] == "none today", f"at {shot}: {desktop.showing()}"
    desktop.shoot(f"launcher-agents-landed-at-{shot}")
    desktop.follow()
    # Newest first: the one landed yesterday, then the one from 7 Nov.
    desktop.launcher_until(lambda shown: shown["rows"] == [CLOSING, STOPPED])
    desktop.down_to(CLOSING)
    selected = desktop.showing()["selected"]
    # A day before today, as a row says one, never its ref, which its drawer says last.
    assert selected["side"] not in {"", "core.nix#2138"}, f"at {shot}: {selected}"
    desktop.launcher_until(
        lambda shown: (
            shown["drawer"]["out"] is True and shown["drawer"]["body"] == LANDINGS[2]["person"]
        )
    )
    desktop.shoot(f"launcher-agents-landed-in-at-{shot}")
    desktop.close_launcher()
    # Read, it's in For You's History, saying when.
    desktop.machine.send_key("meta_l-o")
    desktop.in_now("For You")
    desktop.step_in()
    desktop.launcher_until(lambda shown: "To Try" not in shown["rows"])
    desktop.down_to("History")
    desktop.step_in()
    desktop.launcher_until(lambda shown: CLOSING in shown["rows"])
    desktop.down_to(CLOSING)
    side = desktop.showing()["selected"]["side"]
    assert side.startswith("read "), f"at {shot} a landing read says {side!r} in History"
    desktop.shoot(f"launcher-for-you-history-at-{shot}")
    desktop.close_launcher()
    into_agents(desktop)


def after_the_sessions(desktop: Desktop, shot: str) -> None:
    """Shoot what Agents holds after its sessions: the closed, Landed, Recent and Start."""
    closed_stand_last(desktop, shot)
    landed_lists_what_sessions_landed(desktop, shot)
    desktop.down_to("Recent")
    desktop.shoot(f"launcher-agents-recent-at-{shot}")
    a_quest_offers_to_orchestrate(desktop, shot)


def a_quest_offers_to_orchestrate(desktop: Desktop, shot: str) -> None:
    """Shoot a quest's step in from Start a Session: Start Here, then Orchestrate.

    Normal's foot keeps : and Space ?, since the mode's word holds room only for what it
    says in normal, never for visual's count (VKCF3Z, 2026-11-07: held for "N chosen", it
    cost normal its :); visual says its count where normal's word stood.
    """
    desktop.down_to("Start a Session")
    desktop.machine.send_key("ret")
    desktop.launcher_until(lambda shown: UNSTARTED in shown["rows"])
    desktop.down_to(UNSTARTED)
    desktop.follow()
    desktop.launcher_until(lambda shown: shown["rows"][:2] == ["Start Here", "Orchestrate"])
    hints = desktop.showing()["hints"]
    assert {": commands", "Space ? keys"} <= set(hints), f"at {shot}: {hints}"
    desktop.shoot(f"launcher-agents-start-quest-at-{shot}")
    desktop.machine.send_chars("v")
    desktop.launcher_until(lambda shown: shown["modeWord"] == "1 chosen")
    desktop.shoot(f"launcher-agents-start-quest-visual-at-{shot}")
    desktop.machine.send_key("esc")
    desktop.launcher_until(lambda shown: shown["modeWord"] == "")
    desktop.step_out()
    desktop.step_out()


def glyphs_stand_on_names(rows: list[Json], titles: dict[str, str], scale: str) -> None:
    """Hold each row's glyph where the name above it starts, the first word a row draws."""
    words = {address_of(row): row["words"] for row in rows}
    under = [("TESTQC", "TESTQA"), ("TESTQD", "TESTQC"), ("TESTQE", "TESTQC"), (FOLD, "TESTQA")]
    for one, above in under:
        glyph = words[one][0]["starts"]
        name = next(w["starts"] for w in words[above] if w["text"] == titles[above])
        assert abs(glyph - name) <= 1, (
            f"at {scale} {one}'s glyph is at {glyph}, not on {above}'s name at {name}"
        )


def test_a_session_sits_a_step_in_under_its_orchestrator_which_says_what_all_of_it_spent(
    desktop: Desktop, at_each_scale: list[str]
) -> None:
    """A session sits a step in under its orchestrator, its glyph on its name; flat once typed."""
    orchestrated(desktop)
    try:
        for scale in at_each_scale:
            at_scale(desktop, scale)
            into_agents(desktop)
            rows = orchestration_drawn(desktop)
            order = [address_of(row) for row in rows]
            assert order == list(TREE), f"at {scale} the orchestration reads {order}"
            depths = {address_of(row): row["depth"] for row in rows}
            assert depths == TREE, f"at {scale} the steps in are {depths}"
            titles = {address: title for address, title, _, _ in ORCHESTRATION}
            titles[FOLD] = FOLD
            glyphs_stand_on_names(rows, titles, scale)
            # Short of room, a side gives way whole, the state first, before a name gives up
            # a letter, and what it cost stays, never slid from its left. Beside a tier 2
            # figure, as the builder's 84% full, how it stands outlasts the money instead.
            costs = {"TESTQA": "$14.20 in all", "TESTQD": "84% full \u00b7 working"}
            for row in rows:
                one = address_of(row)
                side_gives_way_whole(row, titles[one], costs.get(one, ""), scale)
            # The orchestrator says what all of it spent; one under it, its own. One idle
            # is listening, never waiting.
            reads = {address_of(row): row["reads"] for row in rows}
            assert "$14.20 in all" in reads["TESTQA"], reads
            assert "$5.10" in reads["TESTQD"], reads
            assert "in all" not in reads["TESTQD"], reads
            assert "listening" in reads["TESTQC"], reads
            assert "paused" in reads["TESTQE"], reads
            # And its glyph is muted, never the live rows' colour (#2052), its name in text.
            inks = {address_of(row): row["words"][0]["ink"] for row in rows}
            assert inks["TESTQE"] != inks["TESTQC"], f"at {scale} paused looks live: {inks}"
            assert "waiting" not in " ".join(reads.values()), reads
            shot = scale.replace(".", "-")
            desktop.shoot(f"launcher-agents-tree-at-{shot}")
            orchestrators_drawer_names_its_repo(desktop, titles["TESTQA"], shot)
            figures_in_the_drawer(desktop, titles["TESTQD"], shot)
            returns_said_in_the_foot(desktop, titles, shot)
            fold_steps_in(desktop, shot)
            close_asks_what_stays(desktop, titles["TESTQC"], "TESTQC", shot)
            after_the_sessions(desktop, shot)
            desktop.close_launcher()
        orchestrator_says_whose_spend_is_whose(desktop)
        # Typed, the builder ranks flat, at no depth, whom it's under leading its note,
        # which the foot says for the row selected.
        desktop.as_account("qs ipc call launcher toggle")
        desktop.launcher_ready()
        desktop.type_in("testqd", selecting="A Builder Under the Owner")
        # Found from the top, it reads as the thing it is, its window, beside its kind.
        laid = json.loads(desktop.as_account("qs ipc call launcher laidOut"))
        row = next(row for row in laid["rows"] if row["selected"])
        assert row["depth"] == 0, row
        note = desktop.showing()["selected"]["note"]
        assert note.startswith("under TESTQC"), note
        # And it leads the words drawn after its name, since the note is only in the foot.
        drawn = [word["text"] for word in row["words"]]
        assert any(text.startswith("under TESTQC") for text in drawn), drawn
        desktop.shoot("launcher-agents-typed-flat")
        desktop.close_launcher()
    finally:
        unorchestrated(desktop)


def test_a_meter_stands_over_dictation_rather_than_on_it_at_each_scale(
    desktop: Desktop, at_each_scale: list[str]
) -> None:
    """Shoot dictation, a meter above it, and a meter alone at its least and most.

    Both are one frame at the bottom centre, the frame's inset from the ink on every side
    (docs/design.md, The Ladder). A meter that shows while dictation listens stands the space
    between frames above it rather than over it, and a shorter figure keeps its room after it,
    so the bar's gap to it never grows. Each is kept for a person to measure.
    """
    machine = desktop.machine
    dictation = desktop.in_session("niri msg layers | grep -q dictation")
    volume = desktop.in_session("niri msg layers | grep -q 'Namespace: \"volume\"'")
    was = desktop.as_account("wpctl get-volume @DEFAULT_AUDIO_SINK@").split()[1]
    # A change is heard only once the shell has bound the speaker, which it says.
    desktop.reported("hears volume ", bool, "", timeout=timedelta(seconds=60))

    # Where on the screen, in fractions of its height, dictation's frame and
    # a meter above it fall at both scales, the middle half across.
    under, over = (0.87, 0.94), (0.78, 0.84)

    def band(rows: tuple[float, float]) -> bytes:
        """Give the screen's middle half across between two fractions of its height."""
        screen = desktop.screen_now()
        width, height = screen.size
        box = (width // 4, int(height * rows[0]), 3 * width // 4, int(height * rows[1]))
        return screen.crop(box).tobytes()

    def shot_once_in(name: str, rows: tuple[float, float], before: bytes) -> None:
        """Shoot once the band shows something it didn't, or after three seconds.

        While dictation listens, its clock and wave move, so the screen never holds still,
        and a meter goes again in a second and a half; and niri can show dictation's first
        frame a while after it's drawn. So the picture is taken once what it's of is on the
        screen. One that never comes, as a meter hidden under dictation, is kept as it is.
        """
        deadline = time.monotonic() + 3
        while band(rows) == before and time.monotonic() < deadline:
            pass
        if desktop.values["shots"]:
            machine.screenshot(name)

    def metered(level: str, name: str) -> None:
        machine.wait_until_fails(volume, timeout=timedelta(seconds=10))
        with desktop.told("drawn volume"):
            desktop.as_account(f"wpctl set-volume @DEFAULT_AUDIO_SINK@ {level}")
        desktop.shoot(name)

    try:
        for scale in at_each_scale:
            named = scale.replace(".", "-")
            at_scale(desktop, scale)
            machine.wait_until_fails(dictation, timeout=timedelta(seconds=15))
            since = desktop.mark()
            before = band(under)
            with desktop.told("drawn dictation"):
                desktop.as_account("dictation toggle")
            desktop.reported("says dictation listening", lambda rest: rest == "", since)
            shot_once_in(f"dictation-listening-at-{named}", under, before)
            machine.wait_until_fails(volume, timeout=timedelta(seconds=10))
            before = band(over)
            with desktop.told("drawn volume"):
                desktop.as_account("wpctl set-volume @DEFAULT_AUDIO_SINK@ 0.5")
            shot_once_in(f"meter-over-dictation-at-{named}", over, before)
            # Stopped, the stand-in types a moment and says it heard nothing.
            desktop.as_account("dictation toggle")
            desktop.reported("says dictation heard nothing", lambda rest: rest == "", since)
            machine.wait_until_fails(dictation, timeout=timedelta(seconds=10))
            metered("0.05", f"meter-least-at-{named}")
            metered("1", f"meter-most-at-{named}")
    finally:
        desktop.as_account(f"wpctl set-volume @DEFAULT_AUDIO_SINK@ {was}")


def spend_line(key: str, name: str, side: str, note: str = "", role: str = "text") -> Json:
    """Give one of AI spend's rows as spend-watch writes it (its collection.rs's Line)."""
    line: Json = {"key": key, "name": name, "side": side, "copy": f"{name}: {side}", "role": role}
    if note:
        line["note"] = note
    # A budget's row and a window's name what opens their drawer in budgets.json.
    if key.startswith("budget:"):
        line["budget"] = key
    return line


def marked(line: Json, mark: str, rows: list[Json] | None = None) -> Json:
    """Give a row the theme's mark its role colours, and what it holds."""
    line = {**line, "mark": mark}
    if rows:
        line["rows"] = rows
    return line


# How things stand with three budgets, as spend-watch writes it: today, with its drawer and the
# day's cap behind it, opening its window's drawer; then Budgets, the month's whole in billed
# spend, its mark its nearest budget's colour; behind it today's whole, then each budget nearest
# its allowance first, the plan stepping into its two windows.
BUDGETED = {
    "v": 2,
    "today": "$412 today",
    "sessions": {},
    "helpers": {},
    "families": {},
    "now": [
        marked(
            spend_line("today", "Today", "$412")
            | {
                "drawn": {
                    "eyebrow": "Sat 7 Nov \u00b7 day",
                    "facts": [
                        {"id": "ends", "value": "$520 \u00b7 likely $470 to $610"},
                        {"id": "from", "value": "the bill \u00b7 pi's logs where it hasn't come"}
                        | {"words": True},
                        {"id": "rebuilding", "value": "$25 \u00b7 6%"},
                        {"id": "most-quest", "value": "Build the shell \u00b7 31%"}
                        | {"words": True},
                        {"id": "most-model", "value": "claude-opus-5-5 \u00b7 all"},
                    ],
                }
            },
            "date",
            rows=[
                marked(
                    spend_line("cap:premium", "Premium Day", "83%", "$250 spent", "warning")
                    | {"budget": "budget:proxy:premium"},
                    "clock",
                )
            ],
        ),
        marked(
            spend_line("budgets", "Budgets", "$21,564 this month", "billed spend", "warning"),
            "agent",
            rows=[
                marked(
                    spend_line(
                        "budgets/all",
                        "All AI Spend",
                        "$412 today",
                        "billed spend \u00b7 $3,163 in 7 days",
                    ),
                    "agent",
                ),
                marked(
                    spend_line("budget:claude", "Claude Pro", "85% of 5 h", role="warning"),
                    "agent",
                    rows=[
                        marked(
                            spend_line(
                                "budget:claude:five-hours",
                                "Five Hours",
                                "85%",
                                role="warning",
                            ),
                            "clock",
                        ),
                        marked(
                            spend_line("budget:claude:week", "Week", "40%"),
                            "clock",
                        ),
                    ],
                ),
                marked(
                    spend_line(
                        "budget:proxy",
                        "Your AI proxy token",
                        "$250 of $300 today",
                        "the default",
                        "warning",
                    ),
                    "agent",
                    rows=[
                        marked(
                            spend_line(
                                "budget:proxy:premium",
                                "Premium Day",
                                "$250 of $300",
                                role="warning",
                            ),
                            "clock",
                        ),
                        marked(
                            spend_line("budget:proxy:weekly", "Week", "$27,570 of $40,000"),
                            "clock",
                        ),
                    ],
                ),
                marked(
                    spend_line("budget:delta", "Delta team token", "$310 this month"),
                    "agent",
                ),
            ],
        ),
    ],
    "quiet": [spend_line("today", "Today", "spending")],
    # The keys listed, in the order spend-watch ranks them (collection.rs's ranked): with no
    # findings, how things stand; and the summary's, which leave today's out.
    "shown": ["today", "budgets"],
    "summaryRows": ["budgets"],
}


def allowance(
    name: str, label: str, standing: str, nearest: float, warn: float, **said: str
) -> Json:
    """Give one of a budget's windows as budgets.json has it, its facts in words."""
    unit = "dollars" if standing.startswith("$") else "share"
    window: Json = {"name": name, "label": label, "unit": unit, "warn": warn}
    window |= {"standing": standing, "nearest": nearest}
    keys = ("brief", "window", "warns", "resets", "ends", "changed", "read", "state")
    return window | {"said": {key: said.get(key, "") for key in keys}}


# How each of BUDGETED's budgets stands, as spend-watch's budgets.json says it, for their
# drawers: a plan's share windows, one past where it warns and one not read yet; the employer's
# money of two dollar windows, the nearer past where it warns; and a team's token of none.
BUDGETS = {
    "v": 1,
    "default": "proxy",
    "sessions": {},
    "budgets": [
        {
            "name": "claude",
            "label": "Claude Pro",
            "words": "your own plan",
            "providers": ["claude-code"],
            "standing": "85% of 5 h",
            "allowances": [
                allowance(
                    "five-hours",
                    "Five Hours",
                    "85% of 5 h",
                    0.85,
                    warn=0.8,
                    brief="85% \u00b7 resets 16:20",
                    window="5 hours",
                    warns="at 80%",
                    resets="16:20",
                    read="Claude at 14:20",
                ),
                allowance(
                    "week",
                    "Week",
                    "40% of the week",
                    0.4,
                    warn=0.8,
                    brief="40% \u00b7 resets Monday 09:00",
                    window="a week",
                    warns="at 80%",
                    resets="Monday 09:00",
                    read="Claude at 14:20",
                ),
            ],
            "said": {
                "what": "your own \u00b7 plan",
                "spent": "",
                "worth": "$84 today \u00b7 $1,120 this month",
                "reading": "reading fine",
                "providers": "Claude Code",
            },
        },
        {
            "name": "proxy",
            "label": "Your AI proxy token",
            "words": "the employer's money",
            "providers": [],
            "standing": "$250 of $300 today",
            "allowances": [
                allowance(
                    "premium",
                    "Premium Day",
                    "$250 of $300",
                    0.833,
                    warn=0.5,
                    window="a day",
                    warns="at $150",
                    resets="05:00 tomorrow",
                    read="the AI proxy at 14:05",
                ),
                allowance(
                    "weekly",
                    "Week",
                    "$27,570 of $40,000",
                    0.689,
                    warn=0.9,
                    window="a week",
                    warns="at $36,000",
                    read="the AI proxy at 14:05",
                ),
            ],
            "said": {
                "what": "the employer's \u00b7 money",
                "spent": "$402 today \u00b7 $3,060 in 7 days \u00b7 $20,144 this month",
                "worth": "",
                "reading": "reading fine",
            },
        },
        {
            "name": "delta",
            "label": "Delta team token",
            "words": "a team's token",
            "providers": [],
            "standing": "$310 this month",
            "allowances": [],
            "said": {
                "what": "a team's \u00b7 token",
                "spent": "$10 today \u00b7 $310 this month",
                "worth": "",
                "reading": "",
            },
        },
    ],
}


def stroke(window: Json) -> Json:
    """Give a window's meter as spend-watch's drawers.rs does: how near, warning from its warn."""
    near = window["nearest"]
    return {"shape": "stroke", "values": [min(max(near, 0.0), 1.0)], "warn": near >= window["warn"]}


def window_facts(window: Json, *, standing: bool, resets: bool) -> list[Json]:
    """Give a window's facts as drawers.rs's window_facts does, a share's without its standing."""
    said = window["said"]
    shown = (
        ("state", said["state"], True),
        ("standing", window["standing"], standing and window["unit"] != "share"),
        ("resets", said["resets"], resets),
        ("override", said["ends"], True),
        ("changed", said["changed"], True),
        ("read", said["read"], True),
        ("warns", said["warns"], True),
    )
    return [
        {"id": fact, "value": value} | ({"figure": stroke(window)} if fact == "standing" else {})
        for fact, value, shows in shown
        if shows and value
    ]


def budget_drawn(budget: Json) -> Json:
    """Give a budget's drawer as drawers.rs's budget does: its windows, then what it spent."""
    windows = budget["allowances"]
    facts = [
        {
            "id": "",
            "label": window["label"],
            "value": window["said"]["brief"] or window["standing"] or "not read yet",
            "figure": stroke(window),
        }
        for window in windows
    ]
    if len(windows) == 1:
        resets = not windows[0]["said"]["brief"]
        facts += window_facts(windows[0], standing=False, resets=resets)
    said = budget["said"]
    providers = said.get("providers") or " \u00b7 ".join(budget["providers"])
    more = (("spent", said["spent"]), ("worth", said["worth"]), ("reading", said["reading"]))
    facts += [
        {"id": fact, "value": value} for fact, value in (*more, ("providers", providers)) if value
    ]
    return {"eyebrow": said["what"] or budget["words"], "title": budget["label"], "facts": facts}


def window_drawn(budget: Json, window: Json) -> Json:
    """Give a window's drawer as drawers.rs's window does, named for the budget it's in."""
    return {
        "eyebrow": f"in {budget['label']} \u00b7 window",
        "title": window["label"],
        "names": True,
        "facts": window_facts(window, standing=True, resets=True),
    }


def drawn_by(line: Json, drawers: dict[str, Json]) -> Json:
    """Give a row, and the rows behind it, the drawer of the budget or window it names."""
    made = {**line}
    if "rows" in line:
        made["rows"] = [drawn_by(one, drawers) for one in line["rows"]]
    if line.get("budget") in drawers:
        made["drawn"] = drawers[line["budget"]]
    return made


def with_drawers(collection: Json, budgets: Json) -> Json:
    """Give a collection's budget, window and cap rows their drawers, as spend-watch lays them.

    Its collection.rs serves each drawer ready on its row from how the budget stands, as
    budgets.json says it, and a cap's is the window's it is.
    """
    drawers: dict[str, Json] = {}
    for budget in budgets["budgets"]:
        drawers[f"budget:{budget['name']}"] = budget_drawn(budget)
        for window in budget["allowances"]:
            drawers[f"budget:{budget['name']}:{window['name']}"] = window_drawn(budget, window)
    return collection | {"now": [drawn_by(line, drawers) for line in collection["now"]]}


def written(desktop: Desktop, name: str, what: Json) -> None:
    """Write one of spend-watch's files whole, as its daemon does, renaming it into place."""
    # Its words have apostrophes, so the JSON is quoted for the shell whole.
    desktop.as_account(
        f"mkdir -p $XDG_RUNTIME_DIR/spend-watch && echo {shlex.quote(json.dumps(what))}"
        f" > $XDG_RUNTIME_DIR/spend-watch/.{name}"
        f" && mv $XDG_RUNTIME_DIR/spend-watch/.{name} $XDG_RUNTIME_DIR/spend-watch/{name}"
    )


def drawer_shot(desktop: Desktop, title: str, picture: str) -> list[str]:
    """Shoot the drawer of the row selected once it's out with its title, and say its facts."""
    desktop.launcher_until(
        lambda shown: shown["drawer"]["out"] is True and shown["drawer"]["title"] == title
    )
    desktop.shoot(picture)
    return cast("list[str]", desktop.showing()["drawer"]["facts"])


def budgets_stand_in_agents(desktop: Desktop, named: str) -> None:
    """Shoot Agents' Budgets row, which is AI spend's: its rows, their order and drawers.

    It's the last of Agents' rows, beside AI Spend, which says today's dollars with their
    period, as All AI Spend does (VKCF3Z, 2026-11-08).
    """
    desktop.as_account("qs ipc call launcher collection agents")
    desktop.launcher_ready()
    desktop.launcher_until(lambda shown: "Budgets" in shown["rows"])
    rows = desktop.showing()["rows"]
    assert rows[-2:] == ["AI Spend", "Budgets"], rows
    desktop.down_to("AI Spend")
    assert desktop.showing()["selected"]["side"] == "$412 today"
    desktop.down_to("Budgets")
    assert desktop.showing()["selected"]["side"] == "$21,564 this month"
    desktop.shoot(f"launcher-agents-budgets-at-{named}")
    desktop.step_in()
    rows = desktop.showing()["rows"]
    assert rows[:3] == ["All AI Spend", "Claude Pro", "Your AI proxy token"], rows
    desktop.down_to("Claude Pro")
    facts = drawer_shot(desktop, "Claude Pro", f"launcher-agents-budgets-each-at-{named}")
    assert facts == ["Five Hours", "Week", "Worth at List", "Feed", "Providers"], facts
    desktop.close_launcher()


def test_the_budgets_stand_behind_one_row_at_each_scale(
    desktop: Desktop, at_each_scale: list[str]
) -> None:
    """Shoot AI spend's Budgets row, the budgets behind it, and their windows' drawers.

    One row stands for every budget, the month's whole beside it; behind it the whole first,
    then each budget nearest its allowance first, each standing in its own unit, and a budget
    steps into its windows (docs/design.md; VKCF3Z, 2026-11-06). A budget's drawer and a
    window's say what budgets.json holds of each (VKCF3Z, 2026-11-07): a plan's, the employer's
    money's, a share window's and a dollar window past where it warns. A running period's
    drawer says its days and what its cuts went to most, and a cap's is its window's. Agents
    shows the same Budgets row last, beside AI Spend. Each is kept for the design's look at
    Joel's scales.
    """
    written(desktop, "budgets.json", BUDGETS)
    written(desktop, "collection.json", with_drawers(BUDGETED, BUDGETS))
    try:
        for scale in at_each_scale:
            named = scale.replace(".", "-")
            at_scale(desktop, scale)
            desktop.as_account("qs ipc call launcher collection spend")
            desktop.launcher_ready()
            desktop.launcher_until(lambda shown: "Budgets" in shown["rows"])
            desktop.down_to("Today")
            facts = drawer_shot(desktop, "Today", f"launcher-spend-today-at-{named}")
            assert facts == [
                "Ends Near",
                "Read From",
                "Rebuilding Context",
                "Most by Quest",
                "Most by Model",
            ], facts
            desktop.step_in()
            desktop.down_to("Premium Day")
            facts = drawer_shot(desktop, "Premium Day", f"launcher-spend-cap-at-{named}")
            assert facts == ["Standing", "Resets", "Read By", "Warns"], facts
            desktop.step_out()
            desktop.down_to("Budgets")
            assert desktop.showing()["selected"]["side"] == "$21,564 this month"
            desktop.shoot(f"launcher-budgets-at-{named}")
            desktop.step_in()
            rows = desktop.showing()["rows"]
            assert rows[:4] == [
                "All AI Spend",
                "Claude Pro",
                "Your AI proxy token",
                "Delta team token",
            ], rows
            desktop.down_to("Your AI proxy token")
            facts = drawer_shot(
                desktop, "Your AI proxy token", f"launcher-budgets-proxy-at-{named}"
            )
            assert facts == ["Premium Day", "Week", "Spent", "Feed"], facts
            desktop.step_in()
            desktop.down_to("Premium Day")
            facts = drawer_shot(desktop, "Premium Day", f"launcher-budgets-dollars-at-{named}")
            assert facts == ["Standing", "Resets", "Read By", "Warns"], facts
            desktop.step_out()
            desktop.down_to("Claude Pro")
            facts = drawer_shot(desktop, "Claude Pro", f"launcher-budgets-each-at-{named}")
            assert facts == ["Five Hours", "Week", "Worth at List", "Feed", "Providers"], facts
            # Each window keeps its meter, under its value where it won't fit beside it, and
            # its eyebrow is our own words, in Label's capitals (VKCF3Z, 2026-11-09).
            drawer = desktop.showing()["drawer"]
            drawn = dict(zip(drawer["drawnFacts"], drawer["figures"], strict=True))
            assert drawn["Five Hours"] == drawn["Week"] == "stroke", f"at {scale}: {drawer}"
            assert drawer["eyebrowNames"] is False, f"at {scale}: {drawer}"
            desktop.step_in()
            rows = desktop.showing()["rows"]
            assert rows[:2] == ["Five Hours", "Week"], rows
            facts = drawer_shot(desktop, "Five Hours", f"launcher-budgets-windows-at-{named}")
            assert facts == ["Resets", "Read By", "Warns"], facts
            desktop.close_launcher()
            budgets_stand_in_agents(desktop, named)
    finally:
        desktop.as_account(
            "rm -f $XDG_RUNTIME_DIR/spend-watch/collection.json"
            " $XDG_RUNTIME_DIR/spend-watch/budgets.json"
        )


# What's typed for the meaning daemon's stand-in to find by its meaning, the words in the keys
# of the things it finds for it, among those the launcher told it of, and the place it finds,
# Screens, whose Brightness stands in its stead.
MEANT = "too bright to read"
MEANT_KEYS = ("bright", "night")
MEANT_PLACE = "collection:displays"
# The most rows meaning adds below what typing found (VKCF3Z, 2026-11-07).
MEANT_MOST = 3


def test_what_is_found_by_its_meaning_sits_below_what_typing_found_at_each_scale(
    desktop: Desktop, at_each_scale: list[str]
) -> None:
    """Find a thing in the launcher, then a screenshot in Recent, by their meaning.

    One subtest holds both, since both prove the one wiring to the meaning daemon, its things
    and its pictures.
    """
    a_thing_found_by_its_meaning(desktop, at_each_scale)
    a_screenshot_found_by_what_it_shows(desktop, at_each_scale)


def a_thing_found_by_its_meaning(desktop: Desktop, at_each_scale: list[str]) -> None:
    """Find things by their meaning, below what typing found, and shoot them.

    The launcher tells the meaning daemon its things as it opens and asks it what's typed, on
    a stand-in that loads no model (tests/desktop/stand-in-meaning.py): at most three of what
    it finds are added below every row typing found, none of them twice, each saying it was
    found by its meaning (VKCF3Z, 2026-11-07). Which it adds, and that nothing moves when they
    arrive late, is tests/meaning.qml's to prove; this is the wiring, and the look at Joel's
    scales.
    """
    desktop.as_account("rm -f $XDG_RUNTIME_DIR/meaning-means.json $XDG_RUNTIME_DIR/meaning-asked")
    desktop.answer(MEANT)
    # The spots are told a moment after the things, once the launcher makes them, so it waits
    # for the part the shot lands on rather than for the first things told.
    desktop.machine.wait_until_succeeds(
        desktop.in_session("grep -q '^spot:.*brightness$' $XDG_RUNTIME_DIR/meaning-known"),
        timeout=timedelta(seconds=10),
    )
    typed = [row["is"] for row in desktop.showing()["things"]]
    desktop.close_launcher()
    known = desktop.as_account("cat $XDG_RUNTIME_DIR/meaning-known").split()
    meant = [key for key in known if key == MEANT_PLACE or any(word in key for word in MEANT_KEYS)]
    assert meant, f"the launcher told of nothing a test can mean: {known}"

    # Two keys can name one thing, a spot and its collection, so what's added is seen by
    # identity: rows after the typed ones, and none of them twice.
    def added(shown: dict[str, Any]) -> list[str]:
        found = [row["is"] for row in shown["things"]]
        return found[len(typed) :] if found[: len(typed)] == typed else []

    desktop.as_account(
        f"echo {shlex.quote(json.dumps({MEANT: meant}))} > $XDG_RUNTIME_DIR/meaning-means.json"
    )
    try:
        for scale in at_each_scale:
            at_scale(desktop, scale)
            desktop.answer(MEANT)
            desktop.launcher_until(lambda shown: added(shown) != [])
            meanings = added(desktop.showing())
            assert len(meanings) <= MEANT_MOST, f"meaning added too many: {meanings}"
            assert len(set(meanings)) == len(meanings), f"a meaning came twice: {meanings}"
            assert not set(meanings) & set(typed), f"a meaning repeats a typed row: {meanings}"
            assert MEANT_PLACE not in meanings, f"Screens stood beside its Brightness: {meanings}"
            desktop.down_to(desktop.showing()["rows"][len(typed)])
            note = desktop.showing()["selected"]["note"]
            assert note == "found by its meaning", f"a meant row says {note!r}"
            desktop.shoot(f"launcher-found-by-meaning-at-{scale.replace('.', '-')}")
            desktop.close_launcher()
    finally:
        desktop.as_account("rm -f $XDG_RUNTIME_DIR/meaning-means.json")


def a_screenshot_found_by_what_it_shows(desktop: Desktop, at_each_scale: list[str]) -> None:
    """In Capture's Recent, a screenshot found by its look sits below those found by name and words.

    Typing in Recent finds the screenshot named for the words, then the one whose words hold
    them, its line below it, then the one the meaning daemon says looks like them, on a stand-in
    that loads no model (tests/desktop/stand-in-meaning.py), saying in the foot that it was
    found by its meaning and nothing under it (VKCF3Z, 2026-11-07). Which it adds is
    tests/meaning.qml's to prove; this is the wiring, and the look at Joel's scales.
    """
    shots = desktop.pictures
    named, said, seen = (
        f"{shots}/wombat plan.png",
        f"{shots}/2026-10-01 09-00-00.png",
        f"{shots}/2026-09-15 08-00-00.png",
    )
    texts = {said: ["Standup", "The wombat review moved to Friday"], named: ["Wombat"]}
    pictures = {named: drawn("square", "steelblue"), said: drawn("square", "grey")}
    pictures[seen] = drawn("circle", "sienna")
    desktop.as_account(
        f"mkdir -p {shlex.quote(shots)}"
        + "".join(
            f" && printf %s {picture} | base64 -d > {shlex.quote(path)}"
            for path, picture in pictures.items()
        )
        + f" && printf %s {shlex.quote(json.dumps(texts))}"
        " > $XDG_RUNTIME_DIR/screen-text-texts.json"
        + f" && printf %s {shlex.quote(json.dumps({'wombat': [seen]}))}"
        " > $XDG_RUNTIME_DIR/meaning-means.json"
    )
    found = ["wombat plan.png", "2026-10-01 09-00-00.png", "2026-09-15 08-00-00.png"]
    try:
        for scale in at_each_scale:
            at_scale(desktop, scale)
            desktop.answer("wombat")
            desktop.launcher_until(
                lambda shown: (shown["selected"] or {}).get("name") == "wombat plan.png"
            )
            desktop.step_in()
            desktop.launcher_until(
                lambda shown: shown["held"][-1:] == ["Recent"] and shown["rows"] == found
            )
            desktop.down_to(found[1])
            desktop.launcher_until(
                lambda shown: shown["selected"]["note"] == "The wombat review moved to Friday"
            )
            desktop.down_to(found[2])
            note = desktop.showing()["selected"]["note"]
            assert note == "found by its meaning", f"a looked-at screenshot says {note!r}"
            desktop.shoot(f"capture-recent-found-by-meaning-at-{scale.replace('.', '-')}")
            desktop.close_launcher()
        asked = desktop.as_account("cat $XDG_RUNTIME_DIR/meaning-asked").split("\n")
        assert "look wombat" in asked, f"the pictures were never asked of: {asked}"
    finally:
        desktop.as_account(
            "rm -f $XDG_RUNTIME_DIR/meaning-means.json $XDG_RUNTIME_DIR/screen-text-texts.json "
            + " ".join(shlex.quote(path) for path in pictures)
        )


# The ErgoDox as the kernel names it, the key the layout finds only on it, and what its watcher
# says while it's locked on its keypad's layer.
ERGODOX = "ZSA Technology Labs ErgoDox EZ"
ITS_KEY = "7 keypad 7"
LOCKED = "Numb locked"
# The laptop's own layout's one layer, as keyd's module describes it (modules/keyd/layout.nix).
LAPTOPS = "Colemak"
# How far, as the mean of every pixel's change in light, the screen moves once the layout's
# card stands on it: measured at 2, the card moves it by ten, and a still screen by nothing.
LAYOUT_CHANGES = 3.0
# How far it moves as one keyboard's card turns to the other's in place, which is less, since
# the two cards overlap.
CARD_TURNS = 1.0


def plug_in_the_ergodox(desktop: Desktop) -> None:
    """Plug the stand-in ErgoDox in, and say its layer as its watcher does once it's there.

    keyboard-lights hears it come, so the shell lists it, and locked, the layer's mark stands
    in the margin, where a mode Joel can forget is held.
    """
    machine = desktop.machine
    since = desktop.mark()
    desktop.as_account(f"echo {shlex.quote(LOCKED)} > $XDG_RUNTIME_DIR/keyboard-layer")
    machine.succeed(f"systemd-run --unit=ergodox {desktop.values['ergodox']}")
    machine.wait_for_file("/run/ergodox.device", timeout=timedelta(seconds=30))
    desktop.reported("holds margin ", lambda rest: "layer" in rest.split(), since)


def unplug_the_ergodox(desktop: Desktop) -> None:
    """Unplug the stand-in ErgoDox, its layer's mark going with it.

    Its watcher's last word is left standing, so only the keyboards connected now can say it's
    gone, as keyboard-lights hears it go.
    """
    since = desktop.mark()
    desktop.machine.succeed("systemctl stop ergodox")
    desktop.reported("holds margin ", lambda rest: "layer" not in rest.split(), since)


def the_card_showing(desktop: Desktop, layer: str, since: str) -> int:
    """Wait for the map to say it shows the layer, and give how tall its card stands."""
    heights: list[int] = []

    def showing(rest: str) -> bool:
        shown, _, height = rest.rpartition(" ")
        if shown == layer:
            heights.append(int(height))
        return shown == layer

    desktop.reported("keyboard layer ", showing, since)
    return heights[-1]


def the_layout_drawn(desktop: Desktop, layer: str) -> int:
    """Bring the layout up by Super+K's call, wait until its card is on the screen, its height.

    The shell says it's drawn as its surface is mapped, but in a guest drawing in software its
    76 keys reach the screen a second or more later, so its pixels are waited on instead.
    """
    bare = desktop.screen_now()
    since = desktop.mark()
    with desktop.told("drawn keyboard"):
        desktop.as_account("qs ipc call keyboard toggle")
    height = the_card_showing(desktop, layer, since)
    desktop_session.retry(
        lambda _: changed_from(bare, desktop.screen_now()) > LAYOUT_CHANGES,
        timeout=timedelta(seconds=30),
    )
    return height


def the_card_turned(desktop: Desktop, since: str, before: Image.Image, layer: str) -> int:
    """Wait for the open card to turn in place to another keyboard's layout, its height."""
    height = the_card_showing(desktop, layer, since)
    desktop_session.retry(
        lambda _: changed_from(before, desktop.screen_now()) > CARD_TURNS,
        timeout=timedelta(seconds=30),
    )
    return height


def keyboards_listed(desktop: Desktop, scale: str, plugged: str) -> list[str]:
    """Give the keyboards the launcher lists, once its search finds the ErgoDox's key or not.

    The keyboards are shot each time, for the design's look.
    """
    there = plugged != "unplugged"
    desktop.answer("keypad 7")
    desktop.launcher_until(lambda shown: (ITS_KEY in shown["rows"]) == there)
    desktop.close_launcher()
    desktop.answer("keyboards")
    desktop.step_in()
    desktop.launcher_until(lambda shown: "The Laptop's Keyboard" in shown["rows"])
    desktop.shoot(f"launcher-keyboards-{plugged}-at-{scale.replace('.', '-')}")
    rows = cast("list[str]", desktop.showing()["rows"])
    desktop.close_launcher()
    return rows


# A keyboard that isn't plugged in is shown nowhere (Joel, 2026-11-07: "we shouldn't be showing
# keyboard shortcuts for keyboards that are not plugged in"): unplugging the ErgoDox turns its
# open layout to the laptop's own in place, the margin lets its layer's mark go, and the
# launcher neither finds its keys nor lists it among the keyboards; Super+K then shows the
# laptop's, and plugging it back in turns the open card to the board's again (VKCF3Z,
# 2026-11-07). With it back, its drawer and every other new drawer are shot
# (each_drawer_drawn), here rather than in a subtest of their own, since the board is
# already plugged in.


def test_an_unplugged_ergodox_is_shown_nowhere_until_its_back(
    desktop: Desktop, at_each_scale: list[str]
) -> None:
    """Unplugging the ErgoDox shows the laptop's layout and leaves it out of the launcher."""
    scale = at_each_scale[-1]
    at_scale(desktop, scale)
    desktop.as_account(
        f"mkdir -p ~/.config/ergodox && cp {desktop.values['ergodox_layout']} ~/.config/ergodox/.l"
        " && mv ~/.config/ergodox/.l ~/.config/ergodox/layout.json"
    )
    named = scale.replace(".", "-")
    try:
        plug_in_the_ergodox(desktop)
        assert ERGODOX in keyboards_listed(desktop, scale, "plugged")
        boards = the_layout_drawn(desktop, "Numb")
        desktop.shoot(f"keyboard-layout-plugged-at-{named}")
        # The open card stays as the board goes, turned to the laptop's own.
        before = desktop.screen_now()
        since = desktop.mark()
        unplug_the_ergodox(desktop)
        turned = the_card_turned(desktop, since, before, LAPTOPS)
        desktop.shoot(f"keyboard-layout-unplugged-at-{named}")
        with desktop.told("gone keyboard"):
            desktop.as_account("qs ipc call keyboard toggle")
        rows = keyboards_listed(desktop, scale, "unplugged")
        assert ERGODOX not in rows, rows
        # The laptop's six rows stand shorter than the board's eight, with no band left of the
        # board's taller card, turned to or opened on.
        assert turned < boards, f"turned to the laptop's at {turned}, the board's at {boards}"
        # Super+K with the laptop alone shows the laptop's, never nothing.
        fresh = the_layout_drawn(desktop, LAPTOPS)
        assert fresh == turned, f"opened on the laptop's at {fresh}, turned to it at {turned}"
        before = desktop.screen_now()
        since = desktop.mark()
        plug_in_the_ergodox(desktop)
        the_card_turned(desktop, since, before, "Numb")
        with desktop.told("gone keyboard"):
            desktop.as_account("qs ipc call keyboard toggle")
        assert ERGODOX in keyboards_listed(desktop, scale, "back")
        each_drawer_drawn(desktop, at_each_scale)
    finally:
        desktop.machine.succeed("systemctl stop ergodox || true")
        desktop.as_account("rm -f ~/.config/ergodox/layout.json $XDG_RUNTIME_DIR/keyboard-layer")


# How long a wait lasts before Now lists Lag, as tests/desktop.nix sets it.
LAG_LASTS = 3600


def disk_filling(desktop: Desktop) -> None:
    """Write the machine's disk guard's status, /nix nearly full and filling, as the guard does."""
    now = time.time()
    status = {
        "v": 1,
        "heartbeat": now + 3600,
        "filesystems": [
            {
                "path": "/nix",
                "band": "nearly-full",
                "free": 18 * 2**30,
                "total": 900 * 2**30,
                "fill_per_hour": 3 * 2**30,
                "full_at": now + 6 * 3600,
                "says": "The disk is nearly full, with 18.0 GiB free.",
            }
        ],
        "frozen": [],
        "holders": [
            {"path": "/nix/store", "bytes": 610 * 2**30, "grew": {"bytes": 9 * 2**30}},
            {"path": "/var/log", "bytes": 2 * 2**30, "grew": {"bytes": 2**29}},
        ],
        "holders_at": now,
        "cleared": [
            {"at": now - 5400, "kind": "cache", "path": "/nix/var/cache", "bytes": 6 * 2**30}
        ],
    }
    desktop.machine.succeed(
        f"echo '{json.dumps(status)}' > /run/disk-guard/.status"
        " && mv /run/disk-guard/.status /run/disk-guard/status.json"
    )


def lag_lasting(desktop: Desktop) -> None:
    """Write a window as lag-watch does, a wait on memory begun longer ago than Lag waits.

    A window with no wait goes first, heard before the next, so a wait lag-watch itself saw
    earlier ends and the stand-in's is a run of its own. Lag hears a window lag-watch stopped
    rewriting as silent, so it's written just before its drawer is opened, each time.
    """
    now = int(desktop.machine.succeed("date +%s").strip())
    lagged = "$XDG_RUNTIME_DIR/lag-watch/status.json"

    def write(window: Json) -> None:
        desktop.as_account(
            f"mkdir -p $(dirname {lagged}) && echo '{json.dumps(window)}' > {lagged}.new"
            f" && mv {lagged}.new {lagged}"
        )

    with desktop.told("heard lag nothing"):
        write({"ended": now, "trouble": [], "worst": "", "who": "", "since": None})
    with desktop.told("heard lag memory"):
        write(lasting_window(now))


def lasting_window(now: int) -> Json:
    """Give lag-watch's window of a wait on memory begun longer ago than Lag waits.

    It ends five minutes on, since Lag hears a window 30 seconds past its end as lag-watch
    gone, and with lag-watch resting nothing writes the next: a walk to Lag at 2 outlasted it.
    """
    return {
        "ended": now + 300,
        "seconds": 10,
        "stalls": {
            "app": {"cpu_some": 2.4, "memory_some": 12.5, "io_some": 0.04},
            "session": {"cpu_some": 0.5, "memory_some": 3.1, "io_some": 0.0},
        },
        "memory": {"app": {"swap_mib": 1144}},
        "trouble": ["apps waited on memory"],
        "worst": "memory",
        "who": "apps",
        "since": now - LAG_LASTS - 10,
    }


def now_drawer(desktop: Desktop, row: str, eyebrow: str, shot: str) -> None:
    """Open Now by Super+O, move down to a piece's row, and shoot its drawer."""
    desktop.machine.send_key("meta_l-o")
    desktop.in_now("For You")
    desktop.launcher_until(lambda shown: row in shown["rows"])
    desktop.down_to(row)
    desktop.launcher_until(
        lambda shown: shown["drawer"]["out"] is True and shown["drawer"]["eyebrow"] == eyebrow
    )
    desktop.shoot(shot)
    desktop.close_launcher()


# Each drawer this desktop's integrations gained says everything its source holds (Joel,
# 2026-11-07: "every integration has an insanely high bar of nerdiness and integration"), where
# the thing is and what it is above, and each is shot at each of Joel's scales for the design's
# look: mpv's program row and its Media, playing and paused, the ErgoDox and the laptop's own
# board, a picture Capture took, Now's Disks and Lag, and its Keyboard Layer, which draws the
# ErgoDox's own drawer, and the Wi-Fi network connected.
def media_drawer(desktop: Desktop, shot: str, side: str, program: str = "") -> None:
    """Open mpv's Media, its side saying how it stands, and shoot its drawer.

    On the way, mpv's own row's drawer is shot too, where a shot is named for it.
    """
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("mpv", selecting="mpv Media Player")
    if program:
        desktop.launcher_until(
            lambda shown: (
                shown["drawer"]["out"] is True
                and shown["drawer"]["title"] == "mpv Media Player"
                and "Playing" in shown["drawer"]["facts"]
            )
        )
        desktop.shoot(program)
    desktop.into_program()
    desktop.type_in("media", selecting="Media")
    desktop.launcher_until(
        lambda shown: (
            shown["drawer"]["out"] is True
            and shown["drawer"]["title"] == "Bless The Child"
            and any(
                thing["reads"].startswith("Media") and side in thing["reads"]
                for thing in shown["things"]
            )
        )
    )
    drawer = desktop.showing()["drawer"]
    assert drawer["eyebrow"].endswith(" \u00b7 track"), drawer
    assert drawer["facts"][0] == "Position", drawer
    desktop.shoot(shot)
    desktop.close_launcher()


def board_drawer(desktop: Desktop, name: str, eyebrow: str, shot: str) -> None:
    """Open the keyboards on one, and shoot its drawer."""
    desktop.answer("keyboards")
    desktop.step_in()
    desktop.launcher_until(lambda shown: name in shown["rows"])
    desktop.down_to(name)
    desktop.launcher_until(
        lambda shown: shown["drawer"]["out"] is True and shown["drawer"]["title"] == name
    )
    drawer = desktop.showing()["drawer"]
    assert drawer["eyebrow"] == eyebrow, drawer
    desktop.shoot(shot)
    desktop.close_launcher()


def recent_drawer(desktop: Desktop, shot: str) -> None:
    """Open what Capture took lately on the newest, and shoot its drawer."""
    desktop.capture()
    desktop.type_in("recent", selecting="Recent")
    desktop.follow()
    desktop.launcher_until(
        lambda shown: (
            shown["drawer"]["out"] is True
            and shown["drawer"]["eyebrow"] == "in ~/Pictures/Screenshots \u00b7 picture"
            and "Dimensions" in shown["drawer"]["facts"]
        )
    )
    desktop.shoot(shot)
    desktop.close_launcher()


# wifid, as varlinkctl reaches it, and Lantern, the stand-in network every desktop machine hears
# (tests/desktop.nix), with the password its access point is given.
WIFID = "/run/wifid/io.jitsusama.Wifi io.jitsusama.Wifi"
LANTERN = '{"name":"Lantern","password":"glowing in the dark"}'
# Signing in at Lantern's front desk, which signs in whatever is posted to it (front-desk.py),
# over bash's own connection, since the machine has no curl.
SIGN_IN = (
    "bash -c 'exec 3<>/dev/tcp/10.42.0.1/80"
    ' && printf "POST /sign-in HTTP/1.0\\r\\nContent-Length: 0\\r\\n\\r\\n" >&3'
    " && cat <&3' | grep -q ' 200 '"
)


def lantern_joined(desktop: Desktop) -> None:
    """Join Lantern through wifid, and sign in at its desk, so it's online with no notice."""
    machine = desktop.machine
    machine.wait_until_succeeds(
        f"varlinkctl call {WIFID}.Scan '{{}}'"
        f" && varlinkctl call {WIFID}.Describe '{{}}' | grep -q '\"Lantern\"'",
        timeout=timedelta(seconds=60),
    )
    machine.succeed(f"varlinkctl call --more {WIFID}.Join '{LANTERN}'")
    machine.wait_until_succeeds(
        "ip -4 address show wlan1 | grep -q 'inet 10.42.0.'", timeout=timedelta(seconds=60)
    )
    machine.succeed(SIGN_IN)
    machine.succeed(f"varlinkctl call {WIFID}.Check '{{}}'")
    machine.wait_until_succeeds(
        f"varlinkctl --json=short call {WIFID}.Describe '{{}}' | grep -qF '\"reach\":\"online\"'",
        timeout=timedelta(seconds=60),
    )


def wifi_drawer(desktop: Desktop, shot: str) -> None:
    """Open Wi-Fi on Lantern, connected, and shoot its drawer.

    Its band says where it is, what moves leads, and its access point's hardware address is
    last, as wifid serves it (modules/wifid/daemon/src/parts.rs).
    """
    desktop.as_account("qs ipc call launcher collection wifi")
    desktop.launcher_ready()
    desktop.type_in("lant", selecting="Lantern")
    desktop.launcher_until(
        lambda shown: (
            shown["drawer"]["out"] is True
            and shown["drawer"]["eyebrow"] == "on 2.4 GHz \u00b7 network"
            and shown["drawer"]["facts"][:1] == ["Signal"]
            and shown["drawer"]["facts"][-1:] == ["Hardware Address"]
        )
    )
    desktop.shoot(shot)
    desktop.close_launcher()


def drawers_at(desktop: Desktop, scale: str) -> None:
    """Shoot each drawer at one of Joel's scales."""
    at_scale(desktop, scale)
    at = scale.replace(".", "-")
    media_drawer(
        desktop,
        f"launcher-media-drawer-playing-at-{at}",
        "playing",
        program=f"launcher-program-drawer-at-{at}",
    )
    desktop.as_account("qs ipc call music toggle")
    media_drawer(desktop, f"launcher-media-drawer-paused-at-{at}", "paused")
    desktop.as_account("qs ipc call music toggle")
    board_drawer(desktop, ERGODOX, "on USB \u00b7 keyboard", f"launcher-ergodox-drawer-at-{at}")
    board_drawer(
        desktop,
        "The Laptop's Keyboard",
        "built in \u00b7 keyboard",
        f"launcher-laptops-keyboard-drawer-at-{at}",
    )
    recent_drawer(desktop, f"launcher-capture-drawer-at-{at}")
    now_drawer(desktop, "Disks", "on this machine \u00b7 disk", f"launcher-disk-drawer-at-{at}")
    lag_lasting(desktop)
    now_drawer(desktop, "Lag", "on this machine \u00b7 lag", f"launcher-lag-drawer-at-{at}")
    now_drawer(
        desktop, "Keyboard Layer", "on USB \u00b7 keyboard", f"launcher-layer-drawer-at-{at}"
    )


# Called from the unplugged ErgoDox subtest once the board is back, so it's plugged in here and
# that subtest's own finally unplugs it.
def each_drawer_drawn(desktop: Desktop, scales: list[str]) -> None:
    """Shoot a program's, its Media's, each keyboard's, a capture's, Now's pieces' and Wi-Fi's."""
    machine = desktop.machine
    desktop.as_account(
        "systemd-run --user --unit=player mpv --ao=null --no-video --loop"
        f" {shlex.quote(desktop.values['track'])}"
    )
    named = "busctl --user list --acquired | grep -q '^org\\.mpris\\.MediaPlayer2\\.mpv'"
    machine.wait_until_succeeds(desktop.in_session(named), timeout=timedelta(seconds=30))
    # lag-watch would write its own window over the stand-in's, so it rests.
    desktop.as_account("systemctl --user stop lag-watch.service")
    try:
        desktop.capture()
        with desktop.captured():
            machine.send_key("ret")
        disk_filling(desktop)
        for scale in scales:
            drawers_at(desktop, scale)
        # Joined last, and left after, so no other drawer is shot on Lantern.
        lantern_joined(desktop)
        for scale in scales:
            at_scale(desktop, scale)
            wifi_drawer(desktop, f"launcher-wifi-drawer-at-{scale.replace('.', '-')}")
    finally:
        machine.succeed(f'varlinkctl call {WIFID}.Leave \'{{"name":"Lantern"}}\' || true')
        desktop.as_account("systemctl --user stop player || true")
        machine.succeed("rm -f /run/disk-guard/status.json")
        desktop.as_account(
            "rm -f $XDG_RUNTIME_DIR/lag-watch/status.json"
            " && systemctl --user start lag-watch.service"
        )


# The device Nearby hears: btvirt's second controller, named as BlueZ names a device by what it
# calls itself, and how strong btvirt says every device is heard (emulator/btdev.c's inquiry).
HEARD = "Buds"
HEARD_AT = "\u221260 dBm \u00b7 strong"
# BlueZ's adapters, both seen as Buds: the launcher looks from the first Quickshell is told of,
# which with BlueZ starting after the shell is either, and hears the other.
ADAPTERS = ("/org/bluez/hci0", "/org/bluez/hci1")


def bluez_set(desktop: Desktop, adapter: str, name: str, kind: str, value: str) -> None:
    """Set one of an adapter's properties over BlueZ's D-Bus, as bluetoothctl would."""
    desktop.machine.succeed(
        f"busctl set-property org.bluez {adapter} org.bluez.Adapter1 {name} {kind} {value}"
    )


def discovering(adapter: str) -> str:
    """Ask BlueZ whether an adapter is looking for devices, which busctl says as "b true"."""
    return f"busctl get-property org.bluez {adapter} org.bluez.Adapter1 Discovering"


def looking(desktop: Desktop) -> bool:
    """Say whether the launcher's adapter, whichever it is, is looking for devices."""
    return any(
        desktop.machine.succeed(discovering(adapter)).strip() == "b true" for adapter in ADAPTERS
    )


def radios_up(desktop: Desktop) -> None:
    """Power both of btvirt's adapters once BlueZ lists them."""
    for adapter in ADAPTERS:
        # An adapter BlueZ lists may still be setting up, and refuses power until it's done.
        desktop.machine.wait_until_succeeds(
            f"busctl set-property org.bluez {adapter} org.bluez.Adapter1 Powered b true",
            timeout=timedelta(seconds=30),
        )


def radios_seen(desktop: Desktop, seen: bool) -> None:
    """Make both adapters visible as Buds, for good, or hide them; BlueZ keeps which."""
    for adapter in ADAPTERS:
        bluez_set(desktop, adapter, "Alias", "s", HEARD)
        bluez_set(desktop, adapter, "DiscoverableTimeout", "u", "0")
        bluez_set(desktop, adapter, "Discoverable", "b", "true" if seen else "false")


def looks_soon(desktop: Desktop) -> None:
    """Wait for the launcher's adapter, whichever it is, to look for devices."""
    desktop.machine.wait_until_succeeds(
        " || ".join(f"{discovering(adapter)} | grep -qx 'b true'" for adapter in ADAPTERS),
        timeout=timedelta(seconds=30),
    )


def buds(shown: dict[str, Any]) -> list[bool]:
    """Say, for each row of Buds the launcher lists, whether it's gone."""
    return [thing["gone"] for thing in shown["things"] if thing["reads"].startswith(HEARD)]


# Nearby lists the devices the radio hears only while it's open (VKCF3Z, 2026-11-07): two
# controllers of btvirt's, each made visible as Buds, so the launcher's radio, whichever it is,
# looks once Nearby is stepped into, and the other is listed with its signal as Quickshell's
# patch says it (modules/quickshell/bluetooth-rssi-and-failures.patch), never paired. BlueZ
# starts here after the shell, as a restarted bluetoothd does, which the shell takes up
# (bluetooth-finds-bluez-late.patch). bluetoothd restarting, as a switch restarts it, says
# nothing of its devices as it stops, and comes back knowing none it hadn't paired, so the shell
# forgets them as it goes and keeps only what BlueZ has on return: with both radios hidden
# across the restart, Buds stays in Nearby, still open, gone, as Steady keeps a row, rather than
# heard on with its last signal, while Nearby looks again; shown once more, Buds is heard afresh,
# by whichever radio the shell now takes for its own, so as the same row or beside the one gone.
# A terminal's `launcher show` of Nearby, with the launcher closed, has the radio look too.
# Run last of the area's, since BlueZ starting moves what the launcher holds.


def nearby_heard(desktop: Desktop, at_each_scale: list[str], values: dict[str, Any]) -> None:
    """Hold Nearby to looking once open, listing Buds with its signal, no longer than BlueZ."""
    machine = desktop.machine
    machine.succeed("modprobe bluetooth && modprobe hci_vhci")
    machine.succeed(f"systemd-run --unit=btvirt {values['btvirt']} -l2")
    machine.succeed("systemctl start bluetooth.service")
    try:
        radios_up(desktop)
        radios_seen(desktop, seen=True)
        for scale in at_each_scale:
            named = scale.replace(".", "-")
            at_scale(desktop, scale)
            desktop.answer("bluetooth")
            # The find answers a moment after the letters, with nothing selected until then.
            desktop.launcher_until(
                lambda shown: (shown["selected"] or {}).get("name") == "Bluetooth"
            )
            desktop.step_in()
            desktop.launcher_until(lambda shown: "Nearby" in shown["rows"])
            # Not looking yet: only Nearby looks.
            assert not looking(desktop)
            desktop.down_to("Nearby")
            desktop.step_in()
            desktop.launcher_until(
                lambda shown: HEARD in shown["rows"], timeout=timedelta(seconds=60)
            )
            assert looking(desktop)
            desktop.down_to(HEARD)
            selected = desktop.showing()["selected"]
            assert selected["side"] == HEARD_AT, selected
            desktop.shoot(f"launcher-nearby-at-{named}")
            desktop.step_out()
            # Out of Nearby, the radio stops looking, and Bluetooth lists only what's paired.
            machine.wait_until_succeeds(
                " && ".join(f"{discovering(adapter)} | grep -qx 'b false'" for adapter in ADAPTERS),
                timeout=timedelta(seconds=30),
            )
            assert HEARD not in desktop.showing()["rows"]
            desktop.shoot(f"launcher-bluetooth-at-{named}")
            desktop.close_launcher()
        desktop.answer("bluetooth")
        desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Bluetooth")
        desktop.step_in()
        desktop.down_to("Nearby")
        desktop.step_in()
        desktop.launcher_until(lambda shown: buds(shown) == [False], timeout=timedelta(seconds=60))
        # Hidden at once, then stopped well before BlueZ would forget Buds for going unheard.
        radios_seen(desktop, seen=False)
        # Restarted, not stopped then started: a BlueZ client on the bus (WirePlumber's) may ask
        # for it the moment it's gone, which has the bus start it again and cancel a stop.
        machine.succeed("systemctl restart bluetooth.service")
        radios_up(desktop)
        desktop.launcher_until(lambda shown: buds(shown) == [True], timeout=timedelta(seconds=30))
        looks_soon(desktop)
        radios_seen(desktop, seen=True)
        desktop.launcher_until(lambda shown: False in buds(shown), timeout=timedelta(seconds=60))
        desktop.close_launcher()
        nearby_shown_to_a_terminal(desktop)
    finally:
        machine.succeed("systemctl stop bluetooth.service btvirt || true")


def nearby_shown_to_a_terminal(desktop: Desktop) -> None:
    """Have one terminal show of Nearby, launcher closed, hear Buds, then let the radio rest.

    BlueZ forgets what it heard first, so Buds is heard by the show's own look, within the one
    show's wait. A device heard is a thing of its own among Nearby's verbs, so it's said as a
    row, in words. The look ends with the terminal's, three seconds after it last asked, and a
    radio that's off is left off: its show finds no Nearby, and nothing turns it on to look.
    """
    machine = desktop.machine
    desktop.launcher_left_closed()
    # The step before's look is let end first, or it could hear Buds again once BlueZ forgets it.
    rested = " && ".join(f"{discovering(adapter)} | grep -qx 'b false'" for adapter in ADAPTERS)
    machine.wait_until_succeeds(rested, timeout=timedelta(seconds=10))
    for adapter in ADAPTERS:
        devices = f"grep -x '{adapter}/dev_[^/]*' || true"
        heard = machine.succeed(f"busctl tree org.bluez --list | {devices}")
        for device in heard.split():
            machine.succeed(
                f"busctl call org.bluez {adapter} org.bluez.Adapter1 RemoveDevice o {device}"
            )
    machine.succeed(desktop.in_session("launcher show machine bluetooth nearby | grep -q Buds"))
    # The look ends three seconds after the terminal's last ask, and the radio stops a moment
    # after, so five seconds holds it to that.
    machine.wait_until_succeeds(rested, timeout=timedelta(seconds=5))
    for adapter in ADAPTERS:
        bluez_set(desktop, adapter, "Powered", "b", "false")
    _, said = machine.execute(desktop.in_session("launcher show machine bluetooth nearby"))
    assert HEARD not in said, said
    for adapter in ADAPTERS:
        powered = f"busctl get-property org.bluez {adapter} org.bluez.Adapter1 Powered"
        assert machine.succeed(powered).strip() == "b false", adapter
        assert machine.succeed(discovering(adapter)).strip() == "b false", adapter


# What's around the machine, on its network and in the air, as the places that list it lay it
# out: one subtest, and last, for Nearby's sake.


def test_the_home_network_and_nearby_list_whats_around_at_each_scale(
    desktop: Desktop, at_each_scale: list[str], values: dict[str, Any]
) -> None:
    """The Home Network lays out the eero daemon's record, and Nearby what the radio hears."""
    home_network_laid_out(desktop, at_each_scale)
    nearby_heard(desktop, at_each_scale, values)
