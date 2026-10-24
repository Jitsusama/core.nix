"""Stand in for a touchscreen as wide as the screen, which swipes or taps when told.

Each line written to /run/touchscreen is how many fingers and which way (RL is from right to
left, DU from down to up), then how far across the screen an up or down swipe goes, as a
fraction of it; or a count, "tap" and anything, for one finger's tap in the middle. Writing
waits until it's read, so a swipe has started once the write returns.
"""

import os
import time
from pathlib import Path
from typing import TYPE_CHECKING, cast

from evdev import AbsInfo, UInput
from evdev import ecodes as e

if TYPE_CHECKING:
    from collections.abc import Sequence

# The screen's size, which the touchscreen covers, in pixels.
WIDTH, HEIGHT = 1280, 800
# Where it's told what to do.
FIFO = "/run/touchscreen"
# How many fingers it can tell apart, and the most tracking ids it hands out.
SLOTS = 10
TRACKING_IDS = 65535
# The way each swipe goes, as a step along x and y.
WAYS = {"LR": (1, 0), "RL": (-1, 0), "UD": (0, 1), "DU": (0, -1)}
# A swipe moves in this many steps, this long apart, in seconds.
STEPS = 10
STEP = 0.02
# How far apart fingers land, across the way they swipe, in pixels.
FINGERS_APART = 40
# How long a tap's finger stays down, in seconds.
TAP = 0.05


def axis(most: int) -> AbsInfo:
    """Describe an axis from 0 to most."""
    return AbsInfo(0, 0, most, 0, 0, 10)


def touchscreen() -> UInput:
    """Make the device, a direct one, as a screen's own touch is."""
    # evdev takes an absolute axis as its code and what it spans, as its docstring says,
    # though its type allows codes alone.
    axes = cast(
        "Sequence[int]",
        [
            (e.ABS_X, axis(WIDTH - 1)),
            (e.ABS_Y, axis(HEIGHT - 1)),
            (e.ABS_MT_SLOT, axis(SLOTS - 1)),
            (e.ABS_MT_TRACKING_ID, axis(TRACKING_IDS)),
            (e.ABS_MT_POSITION_X, axis(WIDTH - 1)),
            (e.ABS_MT_POSITION_Y, axis(HEIGHT - 1)),
        ],
    )
    return UInput(
        {e.EV_KEY: [e.BTN_TOUCH], e.EV_ABS: axes},
        name="stand-in touchscreen",
        input_props=[e.INPUT_PROP_DIRECT],
    )


def report(device: UInput) -> None:
    """Send what's been written as one moment's touch, as evdev's untyped syn() does."""
    device.write(e.EV_SYN, e.SYN_REPORT, 0)


def tap(device: UInput) -> None:
    """Tap one finger in the middle of the screen."""
    device.write(e.EV_ABS, e.ABS_MT_SLOT, 0)
    device.write(e.EV_ABS, e.ABS_MT_TRACKING_ID, 1)
    device.write(e.EV_ABS, e.ABS_MT_POSITION_X, WIDTH // 2)
    device.write(e.EV_ABS, e.ABS_MT_POSITION_Y, HEIGHT // 2)
    device.write(e.EV_KEY, e.BTN_TOUCH, 1)
    report(device)
    time.sleep(TAP)
    device.write(e.EV_ABS, e.ABS_MT_TRACKING_ID, -1)
    device.write(e.EV_KEY, e.BTN_TOUCH, 0)
    report(device)


def swipe(device: UInput, fingers: int, way: str, across: float) -> None:
    """Swipe across most of the screen the way it's told, from 15% to 85%, as a person does.

    The view follows the fingers, so a short swipe would snap back.
    """
    dx, dy = WAYS[way]
    for step in range(STEPS + 1):
        along = 14 * step - 7 * STEPS
        for finger in range(fingers):
            x = int(WIDTH * across) + dx * WIDTH * along // (20 * STEPS)
            x += FINGERS_APART * finger * abs(dy)
            y = HEIGHT // 2 + dy * HEIGHT * along // (20 * STEPS)
            y += FINGERS_APART * finger * abs(dx)
            device.write(e.EV_ABS, e.ABS_MT_SLOT, finger)
            if step == 0:
                device.write(e.EV_ABS, e.ABS_MT_TRACKING_ID, finger + 1)
            device.write(e.EV_ABS, e.ABS_MT_POSITION_X, x)
            device.write(e.EV_ABS, e.ABS_MT_POSITION_Y, y)
        if step == 0:
            device.write(e.EV_KEY, e.BTN_TOUCH, 1)
        report(device)
        time.sleep(STEP)
    for finger in range(fingers):
        device.write(e.EV_ABS, e.ABS_MT_SLOT, finger)
        device.write(e.EV_ABS, e.ABS_MT_TRACKING_ID, -1)
    device.write(e.EV_KEY, e.BTN_TOUCH, 0)
    report(device)


def main() -> None:
    """Do each thing written to the fifo, for as long as it runs."""
    device = touchscreen()
    os.mkfifo(FIFO)
    while True:
        with Path(FIFO).open() as asked:
            for line in asked:
                fingers, way, across = line.split()
                if way == "tap":
                    tap(device)
                else:
                    swipe(device, int(fingers), way, float(across))


if __name__ == "__main__":
    main()
