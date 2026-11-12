"""The margin's cat sits over the column the keys are in, on niri, and follows them.

modules/quickshell/checks/margin.nix runs this through pytest in tests/nested-niri.sh, with the
shell launcher_on_niri.py starts once a run, as the graphical account has it with the cat's
module (modules/margin-cat). Where over a column it sits, beside the margin's marks or nowhere,
is held in Qt's engine (modules/margin-cat/tests/margin-cat.qml), as are a walk's strides; here,
only that niri's real layout reaches it, that it rests two seconds before trotting over in a
few frames, and never for a tap along the columns, that its layer is one taking no keys, that
it goes in fullscreen, where the gap it sits in is gone, that it dozes off after a minute with
no keys and stretches as the screen unlocks, and that niri draws it in whole pixels at 1.5 and
at 2, each screen kept as the check's result.
"""

from __future__ import annotations

import json
import os
import struct
import time
import zlib
from collections import Counter
from datetime import timedelta
from pathlib import Path
from typing import TYPE_CHECKING, cast

import launcher_on_niri
import pytest

if TYPE_CHECKING:
    from collections.abc import Callable, Iterator

    from launcher_on_niri import Json, Shell

# What the cat says each time it's placed (modules/margin-cat/MarginCat.qml).
SITS = "sits cat "
# How long the keys rest on a column before the cat trots there, how many strides its walk
# takes, and the most frames it may draw for a focus move (modules/margin-cat/MarginCat.js).
RESTS, STRIDES, FOCUS_MOVE = 2.0, 9, 12
# How long with no keys draws none of its frames, how long after the last key it may take to
# be heard asleep, its 45 s doze here (modules/quickshell/checks/margin.nix) and some, and the
# frames that curling up may draw.
IDLE, DOZES, POSED = 30.0, 60, 2
# How long the cat's stretch holds as the lock lets go: about half a second, and no less than
# a glance, nor so long that it's in the way of the keys pressed next.
STRETCHES = (0.3, 1.0)
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
    """
    with launcher_on_niri.running() as started:
        heard(started, lambda _: True, 0)
        yield started


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


def placed(said: str) -> Json | None:
    """Give where a report says the cat sits, or None where it says nowhere.

    A report is the screen's name, where it sits or nowhere, and the column it saw under it.
    """
    _, rest = said.split(" ", 1)
    at, _ = rest.split(" over ", 1)
    return None if at == "nowhere" else cast("Json", json.loads(at))


def sits_over(shell: Shell, window: Json, since: int) -> None:
    """Wait until the cat says it sits on this window's column, over its middle, its walk done.

    It sits a cell of air above the column's top, so its feet stand clear of the border. Each
    report is held to the window and column it names itself, never to where the windows are
    by the time it's read, since niri lays them out again as others open and close.
    """

    def over(said: str) -> bool:
        at = placed(said)
        if at is None or at["walking"] or at.get("window") != window["id"]:
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


def test_the_cat_follows_the_keys_from_column_to_column(shell: Shell) -> None:
    """Two columns open; the cat sits over the one the keys are in, and moves as they do."""
    since = shell.mark()
    first = shell.opened("first column")
    second = shell.opened("second column")
    sits_over(shell, second, since)
    since = shell.mark()
    shell.as_account("niri msg action focus-column-left")
    sits_over(shell, first, since)


def test_the_cat_takes_no_keys_on_a_layer_of_its_own(shell: Shell) -> None:
    """It's on the overlay layer, taking no keys, so typing goes where it went before."""
    shell.opened("a column under the cat")
    (layer,) = layered(shell, 1)
    assert layer["layer"] == "Overlay", f"the cat is on the {layer['layer']} layer"
    assert layer["keyboard_interactivity"] == "None", (
        f"the cat takes keys: {layer['keyboard_interactivity']}"
    )


def walked(shell: Shell, since: int) -> int:
    """Give how many strides the cat has said it took since the mark.

    Each is a place it walked to; it says where it is again as the column under it changes too,
    so a stride said twice in the same place is one.
    """
    said, _ = shell.said_since(since)
    reports = [placed(report[len(SITS) :]) for report in said if report.startswith(SITS)]
    places = [(at["x"], at["y"]) for at in reports if at is not None and at["walking"]]
    return sum(1 for at, before in zip(places, [None, *places], strict=False) if at != before)


def counting(shell: Shell) -> str:
    """Give the command that counts the shell's frames by window (Frames.qml)."""
    return f"qs -p {shell.folder} ipc call frames"


def cat_frames(shell: Shell) -> tuple[int, Json]:
    """Give the frames the cat's windows drew since the count started, and every window's."""
    drawn = json.loads(shell.as_account(f"{counting(shell)} said"))
    return sum(count for name, count in drawn.items() if name.startswith("cat ")), drawn


def test_the_cat_trots_over_once_the_keys_rest(shell: Shell) -> None:
    """Two seconds after the keys move to another column, it trots there in a few frames.

    From the key to the cat sitting over its new column it draws at most 12 frames, nine of
    them its walk (VKCF3Z, 2026-11-12: "a focus move draws at most 12"), and it rests two
    seconds first, so a glance along the columns doesn't set it off.
    """
    since = shell.mark()
    first = shell.opened("a column to trot to")
    second = shell.opened("a column to trot from")
    sits_over(shell, second, since)
    shell.as_account(f"{counting(shell)} start")
    since = shell.mark()
    moved = time.monotonic()
    shell.as_account("niri msg action focus-column-left")
    sits_over(shell, first, since)
    rested = time.monotonic() - moved
    cat, drawn = cat_frames(shell)
    assert rested >= RESTS, f"the cat set off {rested:.2f} s after the key, before resting"
    assert walked(shell, since) == STRIDES, f"the cat took {walked(shell, since)} strides"
    assert cat <= FOCUS_MOVE, f"the cat drew {cat} frames for a focus move: {drawn}"


def test_tapping_along_the_columns_never_sets_it_running(shell: Shell) -> None:
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
    assert walked(shell, since) == 0, f"tapping set the cat off: {walked(shell, since)} strides"


def test_the_cat_dozes_off_with_no_keys_drawing_nothing_more(shell: Shell) -> None:
    """With no keys it draws nothing, then curls up asleep in a frame, and a key wakes it.

    Thirty seconds with nothing typed draw none of its frames (VKCF3Z, 2026-11-12: "30 s idle
    draws 0 frames"), its doze has it asleep, and asleep it draws nothing new.
    """
    since = shell.mark()
    sits_over(shell, shell.opened("a column to doze over"), since)
    # Its last frame over the new column lands a moment after it says it sits there.
    time.sleep(RESTS)
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
    time.sleep(RESTS)
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
    time.sleep(RESTS)
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


def screen() -> Path:
    """Give where a screenshot taken only to be read goes."""
    return Path(os.environ["TMPDIR"]) / "screen.png"


def picture(shell: Shell, taken: Path) -> tuple[int, bytes]:
    """Take the screen as niri draws it, in its own pixels, and give its width and its pixels.

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
    return decoded(taken.read_bytes())


def decoded(png: bytes) -> tuple[int, bytes]:
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
    for row in range(height):
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
    """Count each colour of the screen's pixels in the cat's box and a pixel around it."""
    width, pixels = picture(shell, screen())
    left, top = round(at["x"] * scale) - 1, max(0, round(at["y"] * scale) - 1)
    across = round(at["size"] * scale) + 2
    return Counter(
        "#" + pixels[(row * width + column) * 3 : (row * width + column) * 3 + 3].hex()
        for row in range(top, top + across)
        for column in range(left, left + across)
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
    shell.as_account(f"niri msg output winit scale {scale}")
    try:
        # niri lays the columns out again at the new scale a moment after it takes it, so the
        # cat's first report there may still be over the old ones: each try takes its newest.
        at: Json | None = None
        drawn, wanted, colours = 0, -1, Counter[str]()
        deadline = time.monotonic() + LAYERED
        while drawn != wanted and time.monotonic() < deadline:
            said, _ = shell.said_since(since)
            sits = [placed(report[len(SITS) :]) for report in said if report.startswith(SITS)]
            at = sits[-1] if sits else None
            if at is not None and round(at["size"] * scale) == at["cell"] * at["cells"]:
                wanted = at["lit"] * at["cell"] ** 2
                colours = boxed(shell, at, scale)
                drawn = inked(colours, at["hue"])
            if drawn != wanted:
                time.sleep(POLL)
        assert at is not None, f"the cat said nowhere at {scale}"
        assert drawn == wanted, (
            f"at {scale}, {drawn} pixels are the cat's ink, not {at['lit']} cells of "
            f"{at['cell']} squared: {at}; its box holds {colours.most_common(6)}, and "
            f"there are {anywhere(shell, at['hue'])}; drawn:\n{drawn_as(shell, at, scale)}"
        )
        kept = Path(os.environ["out"])  # noqa: SIM112, the check's own result, as Nix names it
        kept.mkdir(exist_ok=True)
        picture(shell, kept / f"cat-at-{scale}.png")
    finally:
        shell.as_account("niri msg output winit scale 1")
