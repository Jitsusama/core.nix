"""What the desktop test's looks areas share.

Joel's scales, the card's and the drawer's measures, the desktop and the scales as fixtures, and the
helpers more than one looks area measures with. tests/desktop.nix gives it to the looks areas'
driver beside desktop_session.py, and each area imports what it uses.
"""

from __future__ import annotations

import base64
import io
import json
from datetime import timedelta
from itertools import pairwise
from typing import TYPE_CHECKING, Any, cast

import desktop_session
import pytest
from desktop_session import LAUNCHER_OPEN
from PIL import Image, ImageChops, ImageDraw, ImageStat

if TYPE_CHECKING:
    from collections.abc import Iterator

    from desktop_session import Desktop, Json
    from test_driver.machine import BaseMachine

# Joel's scales.
JOELS_SCALES = ("1.5", "2")
# The drawer takes what the screen leaves beside the centred card, less the gap between them
# and the margin at the screen's edge, up to its most (Launcher.qml's drawerRoom): the frames
# rung and the screen rung. On the laptop's 2880 that's 520 at 1.5 and 300 at 2; niri gives
# the VM's screen a few logical pixels less, so it's worked out from the screen niri says it
# has.
CARD = 680
GAP = 32
MARGIN = 48
MOST = 520
# The drawer's line pitch, Figure's, two of the card's beats, and how near a line is to where
# it should be: a hundredth, and the half a hundredth each of two lengths the launcher reports
# rounded may take.
PITCH = 24
BEAT = 12
NEAR = 0.02
# A Title's pitch: how far the title sits under the eyebrow, and its lines apart when its name
# wraps, tighter than two of the plate's, so the name reads as one heading. One line's pitch,
# since a Title stands apart by weight rather than size (VKCF3Z, 2026-11-06).
TITLE_LEADING = 24
# How far the text keeps from the plate's frame on every side, its foot included: the inset rung,
# from the frame's inside edge, its hairline border left out, to the ink.
INSET = 24
BORDER = 1
# How long QEMU holds a key down.
HELD_MS = 20


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
        desktop.close_launcher()
    desktop.as_account("niri msg output winit scale 1")


def at_scale(desktop: Desktop, scale: str) -> None:
    """Set the screen's scale, with the launcher closed, so it opens laid out for it."""
    machine = desktop.machine
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))
    desktop.as_account(f"niri msg output winit scale {scale}")


def body_line(lines: Json, card: Json) -> float | None:
    """Say where the plate's body should start, or None where the title's lines are off.

    The title's lines sit its leading apart, and the body starts on the first of the card's
    beats, counted from its first row's baseline, two Title pitches or more under the title's
    last line, so the head reads apart from what follows and the rest lies on the card's beats.
    """
    title, last = float(lines["title"]), float(lines["titleLast"])
    later = round((last - title) / TITLE_LEADING)
    if later < 0 or abs(last - title - later * TITLE_LEADING) > NEAR:
        return None
    first = float(card["rows"][0])
    beats = -(-(last + 2 * TITLE_LEADING - first - NEAR) // BEAT)
    return first + beats * BEAT


def on_the_pitch(line: float, from_line: float) -> bool:
    """Say whether a line lies a whole number of the plate's lines below another."""
    steps = round((line - from_line) / PITCH)
    return steps >= 0 and abs(line - from_line - steps * PITCH) <= NEAR


def on_the_beat(line: float, from_line: float) -> bool:
    """Say whether a line lies a whole number of the card's beats below another."""
    steps = round((line - from_line) / BEAT)
    return steps >= 0 and abs(line - from_line - steps * BEAT) <= NEAR


def things_apart(firsts: list[float], *, wrapped: bool) -> bool:
    """Say whether a list's things lie apart as they should, given each one's first line.

    Its lines are on the pitch, and where any thing wraps, each thing stands a beat further
    from the one before, so a wrapped line sits nearer its own thing than the next.
    """
    apart = BEAT if wrapped else 0
    return all(
        later - earlier > apart and on_the_pitch(later - apart, earlier)
        for earlier, later in pairwise(firsts)
    )


def bottom_set(drawer: Json, lines: list[float]) -> bool:
    """Say whether the last of these lines shown keeps the inset above the frame's bottom.

    Its baseline stands at least the inset above the frame's inside edge, and where more waits
    below, the next thing, with its heading if it opens a group, would not have fit whole there.
    These are each thing's first baselines, so a thing's last stands a step above the next's
    first: a pitch, a beat more where any thing in the list wraps, and two pitches before a
    group's heading, a fact with no name. Past the last thing left, the test can't tell. Where
    it hides any, the line saying so takes the lowest beat the inset above the frame's inside
    edge, and the things end a step above it.
    """
    shown = [line for line in lines if line < drawer["foot"]]
    if not shown:
        return False
    headings = {at for at, name in enumerate(drawer["facts"]) if name == ""}
    step = PITCH + BEAT if drawer["factsWrapped"] else PITCH
    floor = drawer["height"] - BORDER - INSET
    if drawer["moreSays"]:
        if not floor - BEAT < drawer["moreBase"] <= floor + NEAR:
            return False
        floor = drawer["moreBase"] - step
    if shown[-1] > floor + NEAR:
        return False
    thing = len(shown) + (len(shown) in headings)
    if not drawer["more"] or thing + 1 >= len(lines):
        return True
    last = lines[thing + 1] - (2 * PITCH if thing + 1 in headings else step)
    return bool(last > floor + NEAR)


def room_beside_the_card(desktop: Desktop) -> int:
    """Say how wide the drawer should be on the screen as niri lays it out now."""
    output = next(iter(json.loads(desktop.as_account("niri msg --json outputs")).values()))
    width = int(output["logical"]["width"])
    return round(min(MOST, (width - CARD) / 2 - GAP - MARGIN))


def plate_shown(desktop: Desktop, scale: str, seen: Json | None = None) -> Json:
    """Say what the drawer shows, once it's out, holding it to the card's lines and its width.

    What it shows is what the launcher said last, or `seen`, the report a wait passed on.
    """
    drawer = (seen or desktop.showing())["drawer"]
    room = room_beside_the_card(desktop)
    assert drawer["width"] == room, f"at {scale} the plate is {drawer['width']} wide, not {room}"
    assert drawer["height"] == drawer["cardHeight"], f"the plate isn't the card's height: {drawer}"
    lines, card = drawer["lines"], drawer["cardLines"]
    # The head is one unit: the eyebrow on the field's baseline, the title a Title pitch under.
    if lines["eyebrow"] is not None:
        assert abs(lines["eyebrow"] - card["fieldBase"]) <= NEAR, (
            f"at {scale} the eyebrow is off the field's baseline: {drawer}"
        )
    assert abs(lines["title"] - card["fieldBase"] - TITLE_LEADING) <= NEAR, (
        f"at {scale} the title isn't a Title pitch under the eyebrow: {drawer}"
    )
    return cast("Json", drawer)


def changed_from(after: Image.Image, frame: Image.Image) -> float:
    """Say how far a picture is from another, as the mean of its differences' light."""
    return float(ImageStat.Stat(ImageChops.difference(frame, after).convert("L")).mean[0])


def chose(by: str, count: int, shown: Json) -> bool:
    """Say whether the drawer's visual has chosen so many, by letters or by lines."""
    chosen = shown["drawer"]["chosen"]
    return bool(chosen["by"] == by and chosen["count"] == count)


def pressed(desktop: Desktop, keys: str) -> None:
    """Press each key QEMU names, held a moment, the next following at once."""
    for key in keys.split():
        desktop.machine.send_monitor_command(f"sendkey {key} {HELD_MS}")


def drawn(shape: str, colour: str) -> str:
    """Give a picture of one shape on white, as base64, for the drawer to show."""
    picture = Image.new("RGB", (224, 224), "white")
    box = (40, 40, 184, 184)
    if shape == "circle":
        ImageDraw.Draw(picture).ellipse(box, fill=colour)
    else:
        ImageDraw.Draw(picture).rectangle(box, fill=colour)
    png = io.BytesIO()
    picture.save(png, "PNG")
    return base64.b64encode(png.getvalue()).decode()
