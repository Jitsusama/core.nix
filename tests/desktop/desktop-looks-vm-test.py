"""The desktop test's looks area.

Here the launcher is laid out at Joel's two scales, on a screen as large as the laptop's, which
the other areas' screen is too small to hold the card and its drawer at: 1.5, as the HP draws it,
and 2, as the laptop does. Its drawer is a plate whose every line lies on one of the card's
(docs/design.md, the drawer), measured from what the launcher reports of both to a hundredth of a
pixel, and photographed in the gallery for a person to measure too. tests/desktop.nix runs this
through pytest inside the driver (tests/lib/vm_pytest.py), and its opening comment says what the
test proves. Every test asks for the desktop, started once a run (desktop_session.py), and leaves
it as it found it.
"""

from __future__ import annotations

import json
from datetime import timedelta
from itertools import pairwise
from typing import TYPE_CHECKING, Any, cast

import desktop_session
import pytest
from desktop_session import LAUNCHER_OPEN

if TYPE_CHECKING:
    from collections.abc import Iterator

    from desktop_session import Desktop, Json
    from test_driver.machine import BaseMachine

# Joel's scales.
JOELS_SCALES = ("1.5", "2")
# The drawer takes what the screen leaves beside the centred card, less the gap between them
# and the margin at the screen's edge, up to its most (Launcher.qml's drawerRoom). On the
# laptop's 2880 that's 520 at 1.5 and 344 at 2; niri gives the VM's screen a few logical
# pixels less, so it's worked out from the screen niri says it has.
CARD = 680
GAP = 12
MARGIN = 24
MOST = 520
# The plate's pitch, half a row, and how near a line is to where it should be: a hundredth,
# and the half a hundredth each of two lengths the launcher reports rounded may take.
PITCH = 24
NEAR = 0.02


@pytest.fixture(scope="module")
def desktop(machine: BaseMachine, values: dict[str, Any]) -> Desktop:
    """Give the desktop, started once a run."""
    return desktop_session.started(machine, cast("desktop_session.Values", values))


@pytest.fixture
def at_each_scale(desktop: Desktop) -> Iterator[list[str]]:
    """Give Joel's scales to set in turn, and put the screen back at 1 afterwards."""
    yield list(JOELS_SCALES)
    # A test that failed with the launcher open leaves it closed for the next.
    if desktop.machine.execute(desktop.in_session(LAUNCHER_OPEN))[0] == 0:
        desktop.machine.send_key("esc")
    desktop.as_account("niri msg output winit scale 1")


def at_scale(desktop: Desktop, scale: str) -> None:
    """Set the screen's scale, with the launcher closed, so it opens laid out for it."""
    machine = desktop.machine
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))
    desktop.as_account(f"niri msg output winit scale {scale}")


def on_the_pitch(line: float, from_line: float) -> bool:
    """Say whether a line lies a whole number of the plate's lines below another."""
    steps = round((line - from_line) / PITCH)
    return steps >= 0 and abs(line - from_line - steps * PITCH) <= NEAR


def room_beside_the_card(desktop: Desktop) -> int:
    """Say how wide the drawer should be on the screen as niri lays it out now."""
    output = next(iter(json.loads(desktop.as_account("niri msg --json outputs")).values()))
    width = int(output["logical"]["width"])
    return round(min(MOST, (width - CARD) / 2 - GAP - MARGIN))


def plate_shown(desktop: Desktop, scale: str) -> Json:
    """Say what the drawer shows, once it's out, holding it to the card's lines and its width."""
    drawer = desktop.showing()["drawer"]
    room = room_beside_the_card(desktop)
    assert drawer["width"] == room, f"at {scale} the plate is {drawer['width']} wide, not {room}"
    assert drawer["height"] == drawer["cardHeight"], f"the plate isn't the card's height: {drawer}"
    lines, card = drawer["lines"], drawer["cardLines"]
    assert abs(lines["title"] - card["rows"][0]) <= NEAR, (
        f"at {scale} the title is off the first row's baseline: {drawer}"
    )
    return cast("Json", drawer)


def test_a_plates_title_eyebrow_and_words_lie_on_the_cards_lines(
    desktop: Desktop, at_each_scale: list[str]
) -> None:
    """A notice's plate has its eyebrow on the field's centre and its words on the pitch."""
    desktop.as_account(
        "notify-send 'Nightly build finished'"
        " 'All 412 checks passed on the laptop, in 38 minutes. The slowest was the desktop.'"
    )
    machine = desktop.machine
    machine.wait_until_fails(
        desktop.in_session("niri msg layers | grep -q notifications"),
        timeout=timedelta(seconds=60),
    )
    for scale in at_each_scale:
        at_scale(desktop, scale)
        desktop.as_account("qs ipc call launcher toggle")
        desktop.launcher_ready()
        desktop.type_in("nightly build")
        desktop.launcher_until(
            lambda shown: (
                shown["drawer"]["out"] is True
                and shown["drawer"]["title"] == "Nightly build finished"
            )
        )
        drawer = plate_shown(desktop, scale)
        lines, card = drawer["lines"], drawer["cardLines"]
        assert abs(lines["eyebrowMiddle"] - card["field"]) <= NEAR, (
            f"at {scale} the eyebrow is off the field's centre: {drawer}"
        )
        # One empty line below the title.
        assert abs(lines["words"] - (lines["title"] + 2 * PITCH)) <= NEAR, drawer
        desktop.photograph(f"launcher-plate-at-{scale.replace('.', '-')}")
        machine.send_key("esc")


def test_a_plates_facts_lie_on_the_rows_lines_and_halfway(
    desktop: Desktop, at_each_scale: list[str]
) -> None:
    """A time server's plate has its facts from one empty line below the title, on the pitch."""
    kept = "$XDG_RUNTIME_DIR/timekeeping.json"
    machine = desktop.machine
    machine.wait_until_succeeds(
        desktop.in_session("systemctl --user is-active timekeeping"),
        timeout=timedelta(seconds=60),
    )
    # An exchange made, written as timekeeping writes it, with it stopped so nothing writes
    # over it.
    desktop.as_account("systemctl --user stop timekeeping")
    last = int(machine.succeed("date +%s%3N").strip()) - 120_000
    said = {
        "version": 2, "synchronised": True, "frequency": 6.267,
        "poll": {"now": 2048, "least": 32, "most": 2048},
        "servers": {
            "system": [], "link": [], "runtime": [],
            "fallback": ["0.harbour.pool.ntp.org", "1.harbour.pool.ntp.org"],
        },
        "exchange": {
            "server": "0.harbour.pool.ntp.org", "address": "192.0.2.7", "stratum": 1,
            "reference": "GNSS", "offset": 388, "delay": 36112, "jitter": 11888,
            "rootDelay": 15, "rootDispersion": 457, "precision": -25, "leap": "normal",
            "ntpVersion": 4, "last": last, "packets": 13,
        },
    }  # fmt: skip
    try:
        desktop.as_account(f"echo '{json.dumps(said)}' > {kept}.new && mv {kept}.new {kept}")
        for scale in at_each_scale:
            at_scale(desktop, scale)
            desktop.answer("time")
            desktop.step_in()
            desktop.down_to("0.harbour.pool.ntp.org")
            desktop.launcher_until(
                lambda shown: (
                    shown["drawer"]["out"] is True
                    and shown["drawer"]["title"] == "0.harbour.pool.ntp.org"
                    and len(shown["drawer"]["lines"]["facts"]) > 1
                )
            )
            drawer = plate_shown(desktop, scale)
            title, facts = drawer["lines"]["title"], drawer["lines"]["facts"]
            assert abs(facts[0] - (title + 2 * PITCH)) <= NEAR, drawer
            assert all(
                later > earlier and on_the_pitch(later, earlier)
                for earlier, later in pairwise(facts)
            ), f"at {scale} a fact is off the pitch: {drawer}"
            desktop.photograph(f"launcher-plate-facts-at-{scale.replace('.', '-')}")
            machine.send_key("esc")
    finally:
        desktop.as_account("systemctl --user start timekeeping")
