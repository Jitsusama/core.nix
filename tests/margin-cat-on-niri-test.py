"""The margin's cat sits over the column the keys are in, on niri, and follows them.

modules/quickshell/checks/margin.nix runs this through pytest in tests/nested-niri.sh, with the
shell launcher_on_niri.py starts once a run, as the graphical account has it with the cat's
module (modules/margin-cat). Where over a column it sits, beside the margin's marks or nowhere,
is held in Qt's engine (modules/margin-cat/tests/margin-cat.qml), as are a walk's beats; here,
only that niri's real layout reaches it, that it rests two seconds before walking over a step
every frame with none dropped, and never for a tap along the columns, each walk's steps kept as
the check's result, that its layer is one taking no keys, that it goes in fullscreen, where the
gap it sits in is gone, that it dozes off after a minute with no keys and stretches as the
screen unlocks, that niri draws it in whole pixels at 1.5 and at 2, each screen kept as the
check's result, and that a move between workspaces has it hold its ground through niri's slide,
a second move cutting it short holding on from where the first got to, and that on a workspace
with no windows it sits on the floor until one opens there.
"""

from __future__ import annotations

import itertools
import json
import math
import os
import struct
import time
import zlib
from collections import Counter
from datetime import timedelta
from pathlib import Path
from typing import TYPE_CHECKING

import launcher_on_niri
import pytest
from launcher_on_niri import placed

if TYPE_CHECKING:
    from collections.abc import Callable, Iterator

    from launcher_on_niri import Json, Shell

# What the cat says each time it's placed (modules/margin-cat/MarginCatScreen.qml).
SITS = "sits cat "
# How long the cat's last frame, or its hearing the lock's word, may take to land.
LANDS = 2.0
# How long with no keys draws none of its frames, how long after the last key it may take to
# be heard asleep, its 45 s doze here (modules/quickshell/checks/margin.nix) and some, and the
# frames that curling up may draw.
IDLE, DOZES, POSED = 30.0, 60, 2
# How long the cat's stretch holds as the lock lets go: about half a second, and no less than
# a glance, nor so long that it's in the way of the keys pressed next.
STRETCHES = (0.3, 1.0)
# How long the keys rest on a column before the cat walks there
# (modules/margin-cat/MarginCatScreen.qml), and the longest the shell may take to sync and
# render one of its frames, a quarter of a frame at 60, so its own work never costs one.
RESTS = 2.0
# The shortest of a walk's beats, its crouch, in ms: a frame of the nested niri's longer than
# it may hide it.
SHORTEST_BEAT = 110
DRAWS = 4.0
WALKED = "walked cat "
# The fewest steps a second its walk may take, the nested screen drawing at 60.
SMOOTH = 50
# The farthest, in the cat's own size, a step of a move may take it between two frames
# the shell says, a builder's dropped frames included.
STEPS_ON = 2
# The frames of a walk niri may not yet have said it showed as the cat sits.
ON_THEIR_WAY = 3
# How long niri may take to say its layers have changed.
LAYERED = 10.0
POLL = 0.05
# How far each channel may be from the cat's hue for a pixel to be called near it, when
# looking for a cat drawn somewhere other than where it said.
NEAR = 24
# What a PNG's header says of niri's screenshot: eight bits a channel, in red, green and
# blue, with alpha or without.
BYTE, RGB, RGBA = 8, 2, 6


@pytest.fixture(scope="module")
def shell() -> Iterator[Shell]:
    """Give the account's whole shell once its cat has said where it sits, started once.

    Every case reads the cat's reports, so none starts before the cat's surface is loaded.
    Once they're done, nothing the cat did in any of them may have looped a binding or read
    a place left unworked: Joel's shell logged loops on `column` five times from 16:52 to
    16:59 on 2026-11-12, as a walk's handlers set where it sat while its column was said.
    """
    with launcher_on_niri.running() as started:
        heard(started, lambda _: True, 0)
        yield started
        faults = [
            line
            for line in started.log.read_text(errors="replace").splitlines()
            if ("Binding loop" in line or "TypeError" in line) and "MarginCat" in line
        ]
        assert not faults, f"the cat looped or read nothing: {faults}"


@pytest.fixture(autouse=True)
def awake(shell: Shell) -> None:
    """Wake the cat with a key before each case, since nobody types here and it dozes off."""
    since = shell.mark()
    shell.send_key("F24")
    if dozing(shell):
        heard(shell, lambda said: not asleep(said), since)


def dozing(shell: Shell) -> bool:
    """Say whether the cat's newest report has it asleep."""
    said, _ = shell.said_since(0)
    last = next((report for report in reversed(said) if report.startswith(SITS)), None)
    return last is not None and asleep(last[len(SITS) :])


def asleep(said: str) -> bool:
    """Say whether a report has the cat asleep where it sits."""
    at = placed(said)
    return at is not None and bool(at.get("asleep"))


def sits_over(shell: Shell, window: Json, since: int) -> None:
    """Wait until the cat says it sits on this window's column, over its middle.

    It sits a cell of air above the column's top, so its feet stand clear of the border. Each
    report is held to the window and column it names itself, never to where the windows are
    by the time it's read, since niri lays them out again as others open and close.
    """

    def over(said: str) -> bool:
        at = placed(said)
        if at is None or at.get("window") != window["id"]:
            return False
        column = json.loads(said.split(" over ", 1)[1])
        if column is None:
            return False
        left, top, width = column["left"], column["top"], column["width"]
        middle = at["x"] + at["size"] / 2
        return bool(left <= middle <= left + width and at["y"] + at["size"] + at["air"] == top)

    heard(shell, over, since)


def heard(shell: Shell, test: Callable[[str], bool], since: int) -> None:
    """Wait for the cat to report what passes the test, or fail saying all it reported."""
    try:
        shell.reported(SITS, test, since)
    except AssertionError:
        said, _ = shell.said_since(0)
        sat = "\n".join(report for report in said if report.startswith(SITS)) or "nothing"
        failed = f"the cat never sat as wanted; it said, since the shell started:\n{sat}"
        raise AssertionError(failed) from None


def cat_layers(shell: Shell) -> list[Json]:
    """Give each layer surface niri lists as the cat's."""
    layers = json.loads(shell.as_account("niri msg --json layers"))
    return [layer for layer in layers if layer["namespace"] == "cat"]


def layered(shell: Shell, wanted: int) -> list[Json]:
    """Wait until niri lists this many of the cat's layers, and give them."""
    deadline = time.monotonic() + LAYERED
    shown = cat_layers(shell)
    while len(shown) != wanted:
        assert time.monotonic() < deadline, f"niri lists {len(shown)} of the cat, not {wanted}"
        time.sleep(POLL)
        shown = cat_layers(shell)
    return shown


def steps(shell: Shell, since: int) -> list[Json]:
    """Give each step of a walk the cat said since the mark, in the order it took them."""
    said, _ = shell.said_since(since)
    reports = [placed(report[len(SITS) :]) for report in said if report.startswith(SITS)]
    return [at for at in reports if at is not None and at.get("walk")]


def unseen_only_between_frames(walked: list[Json], beats: list[str]) -> list[str]:
    """Give the beats a walk missed of its own, out of order or with no frame to hide them.

    The nested niri draws in software, and one frame in a walk can take longer than a short
    beat lasts: the crouch's 110 ms fell between two frames 121 ms apart (2026-11-12), so a
    beat may go unseen only where the steps either side of it are farther apart than that.
    """
    took: list[tuple[str, float, float]] = []
    for step in walked:
        beat, t = step["walk"]["beat"], step["walk"]["t"]
        if took and took[-1][0] == beat:
            took[-1] = (beat, took[-1][1], t)
        else:
            took.append((beat, t, t))
    wrong: list[str] = []
    at = 0
    for seen, (beat, starts, _) in enumerate(took):
        if beat not in beats[at:]:
            return [f"{beat} out of order"]
        skipped = beats[at : beats.index(beat, at)]
        gap = starts - took[seen - 1][2] if seen else 0
        wrong += [f"{missed} unseen in {gap:.0f} ms" for missed in skipped if gap < SHORTEST_BEAT]
        at = beats.index(beat, at) + 1
    return wrong + [f"{missed} never" for missed in beats[at:]]


def walking(said: str) -> bool:
    """Say whether a report is a step of a walk."""
    at = placed(said)
    return at is not None and bool(at.get("walk"))


def test_the_cat_walks_over_a_frame_at_a_time_once_the_keys_rest(shell: Shell) -> None:
    """Two seconds after the keys move to another column, it walks there, a step each frame.

    It steps at the screen's rate, and its own work never costs a frame: each takes the shell
    a quarter of one to draw, and niri shows every one (Joel, 2026-11-12: "we need proper
    smooth animations with character to them"). How far apart niri shows them is the nested
    screen's, drawn in software under a second compositor, so it's kept, not judged; Joel's
    journal says it of his own screen. Its beats come in the order its way has them, the gap
    between the two columns leapt (Joel, 2026-11-13: "when crossing across gaps between
    windows, the kitty should jump"), and it goes in both its strides, never stepping back.
    Its steps and how niri showed them are kept as the check's result, for a strip of the walk.
    """
    since = shell.mark()
    first = shell.opened("a column to walk to")
    second = shell.opened("a column to walk from")
    sits_over(shell, second, since)
    since = shell.mark()
    moved = time.monotonic()
    shell.as_account("niri msg action focus-column-left")
    heard(shell, walking, since)
    rested = time.monotonic() - moved
    sits_over(shell, first, since)
    walked = steps(shell, since)
    said, _ = shell.said_since(since)
    told = [
        json.loads(line[len(WALKED) :].split(" ", 1)[1]) for line in said if line.startswith(WALKED)
    ]
    kept = Path(os.environ["out"])  # noqa: SIM112, the check's own result, as Nix names it
    kept.mkdir(exist_ok=True)
    (kept / "walk.json").write_text(json.dumps({"steps": walked, "presented": told}))
    assert rested >= RESTS, f"the cat set off {rested:.2f} s after the key, before resting"
    beats = [step["walk"]["beat"] for step in walked]
    way = walked[0]["walk"]["beats"]
    assert "leaps" in way, f"the cat's way leapt no gap: {way}"
    assert unseen_only_between_frames(walked, way) == [], f"the cat's beats came as {beats}"
    times = [step["walk"]["t"] for step in walked]
    gaps = sorted(after - before for before, after in itertools.pairwise(times))
    frame = gaps[len(gaps) // 2]
    assert frame <= 1000 / SMOOTH, f"the cat walked a step every {frame} ms: {times}"
    assert len(told) == 1, f"the cat said how its walk was shown {len(told)} times: {told}"
    (shown,) = told
    assert shown["frames"] > 0, f"niri showed none of the walk's frames: {shown}"
    assert shown["unseen"] == 0, f"niri never showed {shown['unseen']} of its frames: {shown}"
    # Each frame it draws asks niri when it's shown, so a frame asked after the next had begun
    # can't halve what the journal says of Joel's screen; the last few are still on their way
    # as it sits.
    answered = shown["frames"] + shown["unseen"]
    assert answered >= shown["begun"] - ON_THEIR_WAY, (
        f"niri was asked of {answered} of the {shown['begun']} frames it drew: {shown}"
    )
    assert shown["drawn"] <= DRAWS, f"a frame took the shell {shown['drawn']} ms: {shown}"
    # Each stride stepped by its ground is MarginCat.js's, proven frame by frame in Qt's engine
    # (margin-cat.qml); here a frame can come late enough to skip a stride, so this asks only
    # that both reach the screen as it goes, the way it faces.
    going = [step for step in walked if step["walk"]["beat"] == "goes"]
    poses = {step["walk"]["pose"] for step in going}
    assert poses == {"walkingA", "walkingB"}, f"the cat went in the poses {poses}"
    ahead = [
        (after["x"] - before["x"]) * after["walk"]["facing"]
        for before, after in itertools.pairwise(going)
    ]
    assert min(ahead) >= 0, f"the cat stepped back as it went: {[s['x'] for s in going]}"


def test_tapping_along_the_columns_never_sets_it_walking(shell: Shell) -> None:
    """Tapping h then l, back within the rest, leaves the cat sitting where it was."""
    since = shell.mark()
    shell.opened("a column tapped past")
    sat = shell.opened("a column the cat stays on")
    sits_over(shell, sat, since)
    since = shell.mark()
    shell.as_account("niri msg action focus-column-left")
    time.sleep(RESTS / 4)
    shell.as_account("niri msg action focus-column-right")
    time.sleep(RESTS + 1)
    walked = steps(shell, since)
    assert not walked, f"tapping set the cat walking: {walked[:3]}"


def test_the_cat_takes_no_keys_on_a_layer_of_its_own(shell: Shell) -> None:
    """It's on the top layer, taking no keys, so typing goes where it went before.

    The margin, the launcher and notices stay over it, on the overlay layer, so a mark's ink
    lies over its paws as it perches there, and it never hides one (VKCF3Z, 3d799798).
    """
    shell.opened("a column under the cat")
    (layer,) = layered(shell, 1)
    assert layer["layer"] == "Top", f"the cat is on the {layer['layer']} layer"
    over = json.loads(shell.as_account("niri msg --json layers"))
    under = [
        (one["namespace"], one["layer"])
        for one in over
        if one["namespace"] in {"margin", "launcher", "notifications"} and one["layer"] != "Overlay"
    ]
    assert not under, f"these are under the overlay layer, beside the cat: {under}"
    assert layer["keyboard_interactivity"] == "None", (
        f"the cat takes keys: {layer['keyboard_interactivity']}"
    )


def counting(shell: Shell) -> str:
    """Give the command that counts the shell's frames by window (Frames.qml)."""
    return f"qs -p {shell.folder} ipc call frames"


def cat_frames(shell: Shell) -> tuple[int, Json]:
    """Give the frames the cat's windows drew since the count started, and every window's."""
    drawn = json.loads(shell.as_account(f"{counting(shell)} said"))
    return sum(count for name, count in drawn.items() if name.startswith("cat ")), drawn


def test_the_cat_dozes_off_with_no_keys_drawing_nothing_more(shell: Shell) -> None:
    """With no keys it draws nothing, then curls up asleep in a frame, and a key wakes it.

    Thirty seconds with nothing typed draw none of its frames (VKCF3Z, 2026-11-12: "30 s idle
    draws 0 frames"), its doze has it asleep, and asleep it draws nothing new.
    """
    since = shell.mark()
    sits_over(shell, shell.opened("a column to doze over"), since)
    # Its last frame over the new column lands a moment after it says it sits there.
    time.sleep(LANDS)
    since = shell.mark()
    shell.send_key("F24")
    shell.as_account(f"{counting(shell)} start")
    time.sleep(IDLE)
    still, drawn = cat_frames(shell)
    said, _ = shell.said_since(since)
    sat = [report for report in said if report.startswith(SITS)]
    assert still == 0, (
        f"the cat drew {still} frames in {IDLE:.0f} s with no keys: {drawn}; it said {sat}"
    )
    shell.reported(SITS, asleep, since, timedelta(seconds=DOZES))
    time.sleep(LANDS)
    dozed, drawn = cat_frames(shell)
    assert dozed <= POSED, f"the cat drew {dozed} frames dozing off: {drawn}"
    since = shell.mark()
    shell.send_key("F24")
    heard(shell, lambda said: not asleep(said), since)


def stretching(said: str) -> bool:
    """Say whether a report has the cat stretching where it sits."""
    at = placed(said)
    return at is not None and bool(at.get("stretching"))


def test_the_cat_stretches_as_the_screen_unlocks(shell: Shell) -> None:
    """As the lock lets the screens go it stretches for a moment, in one pose, then sits.

    The lock's word is written as Lock.qml writes it, so no lock covers the nested screen, and
    a shell that starts unlocked never stretches, since nothing was let go.
    """
    since = shell.mark()
    sits_over(shell, shell.opened("a column to stretch over"), since)
    marker = Path(os.environ["XDG_RUNTIME_DIR"]) / "quickshell-locked"
    said, _ = shell.said_since(0)
    started = [report for report in said if report.startswith(SITS)]
    assert not any(stretching(report[len(SITS) :]) for report in started), (
        f"the cat stretched with nothing unlocked: {started}"
    )
    marker.write_text("locked\n")
    # The cat hears the lock's word by watching its file, a moment after it's written.
    time.sleep(LANDS)
    since = shell.mark()
    marker.write_text("unlocked\n")
    heard(shell, stretching, since)
    began, since = time.monotonic(), shell.mark()
    heard(shell, lambda said: placed(said) is not None and not stretching(said), since)
    held = time.monotonic() - began
    shortest, longest = STRETCHES
    assert shortest <= held <= longest, f"the cat stretched for {held:.2f} s"


def test_the_cat_goes_in_fullscreen(shell: Shell) -> None:
    """A window in fullscreen leaves no gap above it, so the cat goes, and comes back after."""
    since = shell.mark()
    filling = shell.opened("a column to fill the screen")
    sits_over(shell, filling, since)
    layered(shell, 1)
    since = shell.mark()
    shell.as_account("niri msg action fullscreen-window")
    heard(shell, lambda said: placed(said) is None, since)
    layered(shell, 0)
    shell.as_account("niri msg action fullscreen-window")
    layered(shell, 1)


def test_leaving_and_coming_back_at_1_5_works_its_place_out_once(shell: Shell) -> None:
    """At 1.5 the cat goes and comes back without its place depending on itself.

    Its window's scale and the screen's differ at a fractional scale, so showing or hiding the
    window moved the scale its place is worked out from: Joel's shell logged a binding loop on
    `place` 327 times from 15:12 to 15:57 on 2026-11-12, three around each launcher opening,
    and read a place left undefined meanwhile ("Cannot read property 'x' of undefined").
    """
    since = shell.mark()
    filling = shell.opened("a column to leave at 1.5")
    sits_over(shell, filling, since)
    began = shell.mark()
    shell.as_account("niri msg output winit scale 1.5")
    try:
        for _ in range(2):
            since = shell.mark()
            shell.as_account("niri msg action fullscreen-window")
            heard(shell, lambda said: placed(said) is None, since)
            since = shell.mark()
            shell.as_account("niri msg action fullscreen-window")
            heard(shell, lambda said: placed(said) is not None, since)
        with shell.log.open("rb") as log:
            log.seek(began)
            looped = [
                line
                for line in log.read().decode(errors="replace").splitlines()
                if ("Binding loop" in line or "TypeError" in line) and "MarginCat" in line
            ]
        assert not looped, f"the cat's place depended on itself, or went unread: {looped}"
    finally:
        shell.as_account("niri msg output winit scale 1")


def screen() -> Path:
    """Give where a screenshot taken only to be read goes."""
    return Path(os.environ["TMPDIR"]) / "screen.png"


def picture(shell: Shell, taken: Path, rows: int | None = None) -> tuple[int, bytes]:
    """Take the screen as niri draws it, in its own pixels, and give its width and its pixels.

    Only its first `rows` where given: unfiltering a whole screen in Python takes seconds on a
    loaded builder, which left a wait for the cat two looks at it (2026-11-12, 2 of 6 runs).

    niri's own screenshot rather than grim's: grim scales its picture by the output's pixels
    over its logical size, which niri rounds at 1.5 (1276 over 851), so it filters every
    edge by a hair and smears pixels the screen has whole.
    """
    taken.unlink(missing_ok=True)
    shell.as_account(f"niri msg action screenshot-screen --show-pointer false --path {taken}")
    deadline = time.monotonic() + LAYERED
    while not taken.exists() or taken.stat().st_size == 0:
        assert time.monotonic() < deadline, f"niri wrote no screenshot to {taken}"
        time.sleep(POLL)
    return decoded(taken.read_bytes(), rows)


def decoded(png: bytes, upto: int | None = None) -> tuple[int, bytes]:
    """Give a PNG's width and its pixels as red, green and blue bytes, as niri writes one."""
    assert png.startswith(b"\x89PNG\r\n\x1a\n"), "niri's screenshot isn't a PNG"
    at, width, height, channels, packed = 8, 0, 0, 0, b""
    while at < len(png):
        (length,) = struct.unpack(">I", png[at : at + 4])
        kind, data = png[at + 4 : at + 8], png[at + 8 : at + 8 + length]
        if kind == b"IHDR":
            width, height, depth, colour = struct.unpack(">IIBB", data[:10])
            assert depth == BYTE, f"niri's screenshot has {depth} bits a channel"
            channels = {RGB: 3, RGBA: 4}[colour]
        elif kind == b"IDAT":
            packed += data
        at += 12 + length
    raw, stride = zlib.decompress(packed), width * channels
    rows: list[bytearray] = []
    for row in range(height if upto is None else min(upto, height)):
        start = row * (stride + 1)
        line = bytearray(raw[start + 1 : start + 1 + stride])
        unfilter(raw[start], line, rows[-1] if rows else bytearray(stride), channels)
        rows.append(line)
    pixels = b"".join(rows)
    if channels == RGB:
        return width, pixels
    return width, b"".join(pixels[spot : spot + 3] for spot in range(0, len(pixels), 4))


def unfilter(kind: int, line: bytearray, above: bytearray, channels: int) -> None:
    """Undo a PNG row's filter in place, from the row above it (the PNG standard, 9.2)."""
    for spot, value in enumerate(line):
        left = line[spot - channels] if spot >= channels else 0
        up = above[spot]
        corner = above[spot - channels] if spot >= channels else 0
        guess = [0, left, up, (left + up) // 2, paeth(left, up, corner)][kind]
        line[spot] = (value + guess) & 0xFF


def paeth(left: int, up: int, corner: int) -> int:
    """Give whichever neighbour is nearest left + up - corner, as PNG's Paeth filter does."""
    guess = left + up - corner
    near = min((abs(guess - left), 0), (abs(guess - up), 1), (abs(guess - corner), 2))
    return (left, up, corner)[near[1]]


def anywhere(shell: Shell, hue: str) -> str:
    """Say where on the whole screen pixels near `hue` are, for a cat not found in its box."""
    width, pixels = picture(shell, screen())
    spots = [
        (spot % width, spot // width)
        for spot in range(len(pixels) // 3)
        if near(pixels[spot * 3 : spot * 3 + 3].hex(), hue)
    ]
    if not spots:
        return f"no pixel near it on the {width} pixels wide screen"
    columns, rows = [x for x, _ in spots], [y for _, y in spots]
    return (
        f"{len(spots)} pixels near it on the {width} pixels wide screen, from "
        f"({min(columns)}, {min(rows)}) to ({max(columns)}, {max(rows)})"
    )


def near(colour: str, hue: str) -> bool:
    """Say whether each of a colour's channels is within NEAR of the hue's."""
    pairs = zip(bytes.fromhex(colour[-6:]), bytes.fromhex(hue[-6:]), strict=True)
    return all(abs(a - b) <= NEAR for a, b in pairs)


def inked(colours: Counter[str], hue: str) -> int:
    """Count the pixels of the ink the cat's hue comes out as, its box's commonest near it.

    The software renderer gives a theme's colour back a step or two off in its channels, so the
    ink is the colour it drew rather than the hue it was asked for. A cell split across a
    pixel mixes with the gap behind it into colours the count leaves out.
    """
    ink = [count for colour, count in colours.most_common() if near(colour, hue)]
    return ink[0] if ink else 0


def drawn_as(shell: Shell, at: Json, scale: float) -> str:
    """Draw the cat's box as the screen has it: # its exact hue, ~ near it, . anything else."""
    width, pixels = picture(shell, screen())
    left, top = round(at["x"] * scale) - 1, max(0, round(at["y"] * scale) - 1)
    across = round(at["size"] * scale) + 2
    hue = at["hue"][-6:].lower()

    def mark(spot: int) -> str:
        colour = pixels[spot * 3 : spot * 3 + 3].hex()
        return "#" if colour == hue else "~" if near(colour, hue) else "."

    return "\n".join(
        "".join(mark(row * width + column) for column in range(left, left + across))
        for row in range(top, top + across)
    )


def boxed(shell: Shell, at: Json, scale: float) -> Counter[str]:
    """Count each colour of the screen's pixels in the cat's box and a pixel around it.

    Only those on the screen: the cat's first report at a new scale can still give its place
    at the old one, past the screen's edge at the new, until niri lays the columns out again.
    """
    left, top = round(at["x"] * scale) - 1, max(0, round(at["y"] * scale) - 1)
    across = round(at["size"] * scale) + 2
    width, pixels = picture(shell, screen(), top + across)
    tall = len(pixels) // (width * 3)
    return Counter(
        "#" + pixels[(row * width + column) * 3 : (row * width + column) * 3 + 3].hex()
        for row in range(top, min(top + across, tall))
        for column in range(max(0, left), min(left + across, width))
    )


@pytest.mark.parametrize("scale", [2, 1.5])
def test_the_cat_is_drawn_in_whole_pixels(shell: Shell, scale: float) -> None:
    """At 1.5 as at 2, every lit cell is whole device pixels of its hue, none blurred.

    Each lit cell drawn whole gives the cell's pixels squared of exactly the hue, and a cell
    blurred or split across a pixel gives fewer, so the count says it's sharp. The screen is
    kept as the check's result, for a look at it (WCJQSG, 2026-11-12: "Send VKCF3Z a scale-2
    shot of each before it reaches Joel").
    """
    since = shell.mark()
    sits_over(shell, shell.opened(f"a column at {scale}"), since)
    since = shell.mark()
    shell.as_account(f"{counting(shell)} start")
    shell.as_account(f"niri msg output winit scale {scale}")
    began = time.monotonic()
    heard: list[str] = []
    seen = 0
    try:
        # niri lays the columns out again at the new scale a moment after it takes it, so the
        # cat's first report there may still be over the old ones: each try takes its newest.
        at: Json | None = None
        drawn, wanted, colours = 0, -1, Counter[str]()
        deadline = time.monotonic() + LAYERED
        while drawn != wanted and time.monotonic() < deadline:
            said, _ = shell.said_since(since)
            heard += [
                f"{time.monotonic() - began:.1f}s {report}"
                for report in said[seen:]
                if report.startswith(SITS)
            ]
            seen = len(said)
            sits = [placed(report[len(SITS) :]) for report in said if report.startswith(SITS)]
            at = sits[-1] if sits else None
            if at is not None and round(at["size"] * scale) == at["cell"] * at["cells"]:
                wanted = at["lit"] * at["cell"] ** 2
                colours = boxed(shell, at, scale)
                drawn = inked(colours, at["hue"])
                heard.append(f"{time.monotonic() - began:.1f}s poll: {drawn} in its box")
            if drawn != wanted:
                time.sleep(POLL)
        assert at is not None, f"the cat said nowhere at {scale}"
        # Whether the shell drew the cat's window at all meanwhile, so a cat missing from the
        # screen says which side of the socket kept it.
        frames = cat_frames(shell) if drawn != wanted else (0, {})
        assert drawn == wanted, (
            f"the shell drew {frames[0]} of the cat's frames in the wait, of {frames[1]}; "
            f"at {scale}, {drawn} pixels are the cat's ink, not {at['lit']} cells of "
            f"{at['cell']} squared: {at}; its box holds {colours.most_common(6)}, and "
            f"there are {anywhere(shell, at['hue'])}; drawn:\n{drawn_as(shell, at, scale)}\n"
            f"heard:\n"
            + "\n".join(heard)
            + "\nniri's windows: "
            + shell.as_account("niri msg --json windows")
        )
        kept = Path(os.environ["out"])  # noqa: SIM112, the check's own result, as Nix names it
        kept.mkdir(exist_ok=True)
        picture(shell, kept / f"cat-at-{scale}.png")
    finally:
        shell.as_account("niri msg output winit scale 1")


def moved(shell: Shell, since: int) -> list[Json]:
    """Give each step of a move between workspaces the cat said since the mark, in order."""
    said, _ = shell.said_since(since)
    reports = [placed(report[len(SITS) :]) for report in said if report.startswith(SITS)]
    return [at for at in reports if at is not None and at.get("move")]


def moving(said: str) -> bool:
    """Say whether a report is a step of a move between workspaces."""
    at = placed(said)
    return at is not None and bool(at.get("move"))


def held(shell: Shell, since: int, way: int) -> list[Json]:
    """Give a move's steps once it sits, held still and on the screen while niri slides.

    Its hold is every step said before its first of another beat, at the one place it sat as
    the move began (Paths.moves). It's never off the screen's top or bottom, the layer's
    reach being the screen's on a move, nor rides back after it set off from its hold.
    """
    took = moved(shell, since)
    said = [step["move"]["beat"] for step in took]
    runs = [beat for at, beat in enumerate(said) if at == 0 or beat != said[at - 1]]
    assert runs[0] == "rides", f"the cat's move came as {runs}"
    assert "rides" not in runs[1:], f"the cat's move came as {runs}"
    holding = [(step["x"], step["y"]) for step in took if step["move"]["beat"] == "rides"]
    assert len(set(holding)) == 1, f"it moved as it held: {holding}"
    gone = [step for step in took if step["y"] < 0 or step["y"] + step["size"] > step["reach"]]
    assert not gone, f"it went off the screen: {gone[:3]}"
    assert {step["move"]["way"] for step in took} == {way}, f"it moved {took[:3]}"
    return took


def test_the_cat_holds_its_ground_through_a_slide_between_workspaces(shell: Shell) -> None:
    """Between workspaces, it holds where it sat as niri slides, then sits on the new one.

    Its ways are MarginCatPaths.js's, frame by frame in Qt's engine; here, only that niri's
    workspaces and their windows reach it, up one and down one, and that it's seen through
    the slide, never ridden off the screen to come back in somewhere else (VKCF3Z 4138e938,
    4d18ade4). Each move's steps are kept as the check's result, for a strip of it.
    """
    since = shell.mark()
    upper = shell.opened("a workspace to go up to")
    sits_over(shell, upper, since)
    shell.as_account("niri msg action focus-workspace-down")
    since = shell.mark()
    lower = shell.opened("a workspace to go down to")
    sits_over(shell, lower, since)
    since = shell.mark()
    shell.as_account("niri msg action focus-workspace-up")
    heard(shell, moving, since)
    sits_over(shell, upper, since)
    up = held(shell, since, -1)
    since = shell.mark()
    shell.as_account("niri msg action focus-workspace-down")
    heard(shell, moving, since)
    sits_over(shell, lower, since)
    down = held(shell, since, 1)
    kept = Path(os.environ["out"])  # noqa: SIM112, the check's own result, as Nix names it
    kept.mkdir(exist_ok=True)
    (kept / "moves.json").write_text(json.dumps({"up": up, "down": down}))


def on_the_floor(said: str) -> bool:
    """Say whether a report has the cat sitting on the screen's floor, still."""
    at = placed(said)
    return at is not None and not at.get("walk") and at["y"] + at["size"] == at["reach"]


def test_an_empty_workspace_has_it_fall_to_the_floor_and_climb_back(shell: Shell) -> None:
    """On a workspace with no windows it sits on the floor, and climbs to one opened there.

    Its fall and climb are MarginCatPaths.js's, frame by frame in Qt's engine; here, only that
    niri's empty workspace reaches it: it's never nowhere from the move on, and sits still at
    the screen's foot until a window comes (VKCF3Z, e5a3ee44).
    """
    since = shell.mark()
    above = shell.opened("a workspace to leave for an empty one")
    sits_over(shell, above, since)
    holding = {window["workspace_id"] for window in shell.windows()}
    spaces = json.loads(shell.as_account("niri msg --json workspaces"))
    empty = next(space for space in spaces if space["id"] not in holding)
    since = shell.mark()
    shell.as_account(f"niri msg action focus-workspace {empty['idx']}")
    heard(shell, on_the_floor, since)
    said, _ = shell.said_since(since)
    reports = [report[len(SITS) :] for report in said if report.startswith(SITS)]
    nowhere = [report for report in reports if placed(report) is None]
    assert not nowhere, f"the cat went from the screen: {nowhere[:2]}"
    since = shell.mark()
    there = shell.opened("a window come to an empty workspace")
    sits_over(shell, there, since)


def test_a_second_move_holds_on_from_where_the_first_got_to(shell: Shell) -> None:
    """A move between workspaces as the last still holds moves it on from where it got to.

    It's never put anywhere it didn't get to (WCJQSG, 2bd7c7c6): its first place said after
    the second move is made is a step from its last before, and the second move is one of its
    own, back the other way. A step of the first move said as the second is made may come
    after the mark, and is a step on too.
    """
    since = shell.mark()
    upper = shell.opened("a workspace moved back to")
    sits_over(shell, upper, since)
    shell.as_account("niri msg action focus-workspace-down")
    since = shell.mark()
    lower = shell.opened("a workspace moved back from")
    sits_over(shell, lower, since)
    since = shell.mark()
    shell.as_account("niri msg action focus-workspace-up")
    sits_over(shell, upper, since)
    first = shell.mark()
    shell.as_account("niri msg action focus-workspace-down")
    heard(shell, moving, first)
    before, since = shell.said_since(first)
    shell.as_account("niri msg action focus-workspace-up")
    sits_over(shell, upper, since)
    said, _ = shell.said_since(since)
    places = [placed(report[len(SITS) :]) for report in before + said if report.startswith(SITS)]
    # The first place the second move says, past any step of the first said after the mark.
    down = [((one or {}).get("move") or {}).get("way", 0) > 0 for one in places]
    assert any(down), "the first move never said where the cat was"
    turned = next((i for i in range(down.index(True), len(down)) if not down[i]), None)
    assert turned is not None, "the second move never said where the cat was"
    was, after = places[turned - 1], places[turned]
    assert was is not None, "the cat was nowhere as the first move went"
    assert after is not None, "the cat was nowhere as the second move began"
    gone = math.hypot(after["x"] - was["x"], after["y"] - was["y"])
    assert gone <= STEPS_ON * was["size"], f"the second move put it at {after}, from {was}"
    again = [step for step in moved(shell, since) if step["move"]["way"] < 0]
    assert again, "the second move set it on its seat, moving it no more"
