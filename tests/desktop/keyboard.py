"""Stand in for a keyboard, which presses one key each time it's told, and says when it did.

Each line written to /run/keyboard names a key by its evdev name without KEY_ ("down", "a"). The
keyboard presses and lets go of it, then appends the key and when it went down, in milliseconds
of the realtime clock, to /run/keyboard.log. That's the clock the shell's Date.now() reads, so a
key's time and the time of the frame that drew it are on one clock inside the machine. The
device's event node goes to /run/keyboard.device first, so a test can wait until the desktop has
it open before it presses.
"""

import os
import time
from pathlib import Path

from evdev import UInput
from evdev import ecodes as e

# Where it's told what to press, where it says what it pressed and when, and which device it is.
FIFO = "/run/keyboard"
LOG = "/run/keyboard.log"
DEVICE = "/run/keyboard.device"
# Every key from Esc to the arrows and Page Down, which is what udev looks for to call a device a
# keyboard (ID_INPUT_KEYBOARD) and every key the launcher is driven by.
KEYS = list(range(e.KEY_ESC, e.KEY_PAGEDOWN + 1))
# How long a key is held, in seconds: less than any repeat waits.
HELD = 0.02


def report(device: UInput) -> None:
    """Send what's been written as one moment's keys."""
    device.write(e.EV_SYN, e.SYN_REPORT, 0)


def press(device: UInput, key: int) -> float:
    """Press the key once and let it go, giving when it went down in milliseconds."""
    pressed = time.clock_gettime(time.CLOCK_REALTIME) * 1000
    device.write(e.EV_KEY, key, 1)
    report(device)
    time.sleep(HELD)
    device.write(e.EV_KEY, key, 0)
    report(device)
    return pressed


def main() -> None:
    """Make the keyboard, say where it is, and press each key it's told, for as long as it runs."""
    device = UInput({e.EV_KEY: KEYS}, name="stand-in keyboard")
    Path(DEVICE).write_text(f"{device.device.path}\n")
    os.mkfifo(FIFO)
    with Path(LOG).open("a") as said:
        while True:
            with Path(FIFO).open() as asked:
                for line in asked:
                    name = line.strip()
                    pressed = press(device, e.ecodes[f"KEY_{name.upper()}"])
                    said.write(f"{name} {pressed:.1f}\n")
                    said.flush()


if __name__ == "__main__":
    main()
