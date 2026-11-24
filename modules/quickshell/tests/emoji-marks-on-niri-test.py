"""Super+E's groups have their emoji centred on their names, as niri shows the launcher.

modules/quickshell/checks/core.nix runs this through pytest in tests/lib/nested-niri.sh, with the
shell launcher_on_niri.py starts once a run and the clipboard's history serving Emoji's rows.
The shell draws here through Qt's GPU path, on Mesa's software GL, as Joel's draws on its GPU.
modules/quickshell/tests/emoji-marks.qml, which draws each mark anew and alone, stood each colour
emoji on its name's capitals, while Joel's screen at 1.5 drew every group's emoji about 7 device
pixels high (2026-11-19, 19:39): the launcher's rows kept a measure of each emoji with no ink.
So what's read here is the launcher's own rows as niri's screencopy has them: each row read, its
emoji the coloured ink left of its name, has its middle within a device pixel of its name's,
halfway from the top of its capitals to their baseline. A row is read only where its emoji is
coloured and its name runs well past it, which Emoji's group rows are; at least three must be.
"""

from __future__ import annotations

import subprocess
import time
from collections import Counter
from typing import TYPE_CHECKING

import launcher_on_niri
import pytest

if TYPE_CHECKING:
    from collections.abc import Iterator

    from launcher_on_niri import Shell

# Emoji's first group, which its place lists first.
FIRST_GROUP = "Smileys & Emotion"
# How far a mark's middle may stand from its name's, in device pixels (Joel, 2026-11-19).
WITHIN = 1.0
# How many rows must be read, of the groups a small screen shows, for the case to say anything.
READ_LEAST = 3
# A pixel of ink: brighter than the launcher's ground, selected or not, at its brightest.
INK = 110
# How far apart a pixel's brightest and dimmest channels are in an emoji's colour, at least,
# which the cream of a name's text never is.
COLOURED = 80
# How many blank pixels part one patch of colour from the next, across or down.
APART = 2
# How tall a patch of colour may be and still be a mark, in device pixels.
MARK_TALLEST = 80
# How short, so a fleck of colour isn't taken for one.
MARK_SHORTEST = 6
# How far right of its mark a row's name is read, in device pixels.
NAME_RUNS = 400
# How many times its mark's height a row's name runs across, at least.
NAME_LEAST = 5
# How many times its mark's height the gap before a row's name may be, at most.
NAME_NEAR = 2
# How much wider than tall, or taller than wide, a mark may be: an emoji is about square.
SQUARE = 1.6
# How far left edges may stand apart and still be one column of marks, in device pixels.
COLUMN = 8
# The share of a name's widest row of ink its baseline still holds, so a descender below it,
# a y's or a j's, is never taken for it.
BASELINE_HOLDS = 0.25
# How long the launcher has to stand still before it's read anyway, in seconds: its caret may
# blink, so two pictures in a row may never agree.
STILL_WITHIN = 5.0
# A PPM header's fields: its magic, width, height and greatest value.
PPM_FIELDS = 4

Pixels = tuple[int, int, bytes]


@pytest.fixture(scope="module")
def shell() -> Iterator[Shell]:
    """Give the shell, started once for every scale, and left at 1 as it began."""
    with launcher_on_niri.running() as started:
        yield started
        started.as_account("niri msg output winit scale 1")


def picture() -> Pixels:
    """Take niri's screen through its screencopy, giving its width, height and pixels."""
    ppm = subprocess.run(["grim", "-t", "ppm", "-"], check=True, capture_output=True).stdout
    fields: list[bytes] = []
    at = 0
    while len(fields) < PPM_FIELDS:
        assert at < len(ppm), f"grim's picture ends in its header: {fields!r}"
        while ppm[at : at + 1].isspace():
            at += 1
        start = at
        while at < len(ppm) and not ppm[at : at + 1].isspace():
            at += 1
        fields.append(ppm[start:at])
    assert fields[0] == b"P6", f"grim's picture isn't a PPM: {fields[0]!r}"
    assert fields[3] == b"255", f"grim's picture has {fields[3]!r} levels a channel"
    return int(fields[1]), int(fields[2]), ppm[at + 1 :]


def still_picture() -> Pixels:
    """Take niri's screen once two pictures in a row agree, or the last at STILL_WITHIN."""
    deadline = time.monotonic() + STILL_WITHIN
    last = picture()
    while time.monotonic() < deadline:
        time.sleep(0.1)
        now = picture()
        if now == last:
            break
        last = now
    return last


def colour(shown: Pixels, x: int, y: int) -> tuple[int, int, int]:
    """Give a pixel's red, green and blue."""
    width, _, pixels = shown
    at = (y * width + x) * 3
    return pixels[at], pixels[at + 1], pixels[at + 2]


def coloured(rgb: tuple[int, int, int]) -> bool:
    """Say whether a pixel is an emoji's colour."""
    return max(rgb) >= INK and max(rgb) - min(rgb) >= COLOURED


def patches(spots: set[tuple[int, int]]) -> list[list[tuple[int, int]]]:
    """Part pixels into patches, each pixel within APART of another of its own.

    The wallpaper's lit windows are coloured too, beside the launcher and level with its
    rows, so a mark is told from them by where it stands, not by its row alone.
    """
    left = set(spots)
    parted: list[list[tuple[int, int]]] = []
    reach = range(-APART, APART + 1)
    while left:
        start = left.pop()
        patch = [start]
        waiting = [start]
        while waiting:
            x, y = waiting.pop()
            for across in reach:
                for down in reach:
                    near = (x + across, y + down)
                    if near in left:
                        left.remove(near)
                        patch.append(near)
                        waiting.append(near)
        parted.append(patch)
    return parted


def middle(rows: list[int]) -> float:
    """Give the middle, down, of the pixel rows ink stands in."""
    return (min(rows) + max(rows)) / 2


def name_beside(shown: Pixels, left: int, top: int, bottom: int) -> tuple[float, int, int] | None:
    """Give a name's capitals' middle, top to baseline, where it starts and its width, or None.

    Its top is its highest ink, an ascender's or a dot's too where one stands past the capitals:
    a share of the widest row, as the baseline takes, put a round capital's thin top 3 pixels
    low (2026-11-19). Its baseline is the lowest row holding a share of its widest, so a
    descender below it is never taken for it.
    """
    width, height, _ = shown
    counts: dict[int, int] = {}
    across: set[int] = set()
    for y in range(max(0, top), min(height, bottom)):
        inked = 0
        for x in range(left, min(width, left + NAME_RUNS)):
            rgb = colour(shown, x, y)
            if max(rgb) >= INK and not coloured(rgb):
                inked += 1
                across.add(x)
        if inked:
            counts[y] = inked
    if not counts:
        return None
    widest = max(counts.values())
    baseline = max(y for y, inked in counts.items() if inked >= widest * BASELINE_HOLDS)
    return (min(counts) + baseline) / 2, min(across), max(across) - min(across) + 1


def marks_and_names(shown: Pixels) -> list[tuple[float, float]]:
    """Give each row's mark middle and its name's, down the screen, in device pixels."""
    width, height, _ = shown
    spots = {(x, y) for y in range(height) for x in range(width) if coloured(colour(shown, x, y))}
    named: list[tuple[int, float, float]] = []
    for patch in patches(spots):
        rows = [y for _, y in patch]
        tall = max(rows) - min(rows) + 1
        if not MARK_SHORTEST <= tall < MARK_TALLEST:
            continue
        # A row's name runs well past its mark, which a lit window's neighbours never do.
        name = name_beside(shown, max(x for x, _ in patch) + 3, min(rows) - tall, max(rows) + tall)
        wide = max(x for x, _ in patch) - min(x for x, _ in patch) + 1
        if (
            name is not None
            and 1 / SQUARE <= wide / tall <= SQUARE
            and name[1] - max(x for x, _ in patch) <= NAME_NEAR * tall
            and name[2] >= NAME_LEAST * tall
        ):
            named.append((min(x for x, _ in patch), middle(rows), name[0]))
    # The marks stand on one column, which a drawer's or a frame's colours don't: the left
    # edge most others stand near.
    lefts = [left for left, _, _ in named]
    near = Counter({left: sum(abs(left - other) <= COLUMN for other in lefts) for left in lefts})
    column = near.most_common(1)[0][0] if near else 0
    return sorted((mark, name) for left, mark, name in named if abs(left - column) <= COLUMN)


@pytest.mark.parametrize("scale", [1.5, 2])
def test_each_groups_emoji_stands_centred_on_its_name(shell: Shell, scale: float) -> None:
    """Super+E's groups read have their emoji's middle within a device pixel of their name's.

    Joel, 2026-11-19 19:39, at 1.5: "the alignment for emojis is better, but still not right".
    """
    # Emoji's rows are the clipboard history's, which answers once it has started.
    launcher_on_niri.warmed(shell)
    shell.as_account(f"niri msg output winit scale {scale}")
    try:
        shell.as_account(shell.ipc("collection", "emoji"))
        shell.launcher_ready()
        shell.launcher_until(lambda shown: shown["rows"][:1] == [FIRST_GROUP])
        read = marks_and_names(still_picture())
    finally:
        # Shut whatever happened, so the next scale's case opens it afresh.
        if shell.showing()["open"]:
            shell.close_launcher()
        shell.as_account("niri msg output winit scale 1")
    said = ", ".join(f"{mark:.1f} against {name:.1f}" for mark, name in read)
    assert len(read) >= READ_LEAST, f"read {len(read)} of Emoji's rows at {scale}: {said}"
    assert all(abs(mark - name) <= WITHIN for mark, name in read), (
        f"at {scale}, each row's emoji middle against its name's, down: {said}"
    )
