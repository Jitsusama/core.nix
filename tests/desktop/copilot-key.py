"""Stand in for a laptop keyboard's Copilot key, which presses when told.

The key sends Super, Shift and F23 at once, and lets them go in the other order, as the XPS 14's
does: within a millisecond of each other, with F23 held a tenth of a second. Each line written to
/run/copilot-key presses it once. The device's event node goes to /run/copilot-key.device first,
so a test can wait until niri has it open before it presses.
"""

import os
import time
from pathlib import Path

from evdev import UInput
from evdev import ecodes as e

# Where it's told to press, and where it says which device it is.
FIFO = "/run/copilot-key"
DEVICE = "/run/copilot-key.device"
# What the key sends, in the order it presses them.
CHORD = [e.KEY_LEFTMETA, e.KEY_LEFTSHIFT, e.KEY_F23]
# How long F23 is held, in seconds, as the laptop's key holds it.
HELD = 0.1


def report(device: UInput) -> None:
    """Send what's been written as one moment's keys."""
    device.write(e.EV_SYN, e.SYN_REPORT, 0)


def press(device: UInput) -> None:
    """Press the key once: every part down in order, then up in the other."""
    for key in CHORD:
        device.write(e.EV_KEY, key, 1)
        report(device)
    time.sleep(HELD)
    for key in reversed(CHORD):
        device.write(e.EV_KEY, key, 0)
        report(device)


def main() -> None:
    """Make the key, say where it is, and press it each time it's told, for as long as it runs."""
    device = UInput({e.EV_KEY: CHORD}, name="stand-in Copilot key")
    Path(DEVICE).write_text(f"{device.device.path}\n")
    os.mkfifo(FIFO)
    while True:
        with Path(FIFO).open() as asked:
            for _ in asked:
                press(device)


if __name__ == "__main__":
    main()
