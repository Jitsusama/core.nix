"""Stand in for the ErgoDox, plugged in for as long as it runs.

The kernel lists it as it lists the board: ZSA's vendor and the ErgoDox's product on USB, with
keys and the three lock lights, which is what the launcher's Keyboards place takes a keyboard to
be. Stopping it unplugs it. The device's event node goes to /run/ergodox.device once it's made, so
a test can wait until the board is there.
"""

import signal
from pathlib import Path

from evdev import UInput
from evdev import ecodes as e

# Where it says which device it is.
DEVICE = "/run/ergodox.device"
# The board as the kernel's list of input devices names it, and its numbers.
NAME = "ZSA Technology Labs ErgoDox EZ"
VENDOR = 0x3297
PRODUCT = 0x1307
# Where a board on the machine's first USB port sits.
PHYS = "usb-0000:00:14.0-2/input0"
# Every key from Esc to Page Down, which is what udev looks for to call a device a keyboard
# (ID_INPUT_KEYBOARD), and the lock lights, which the kernel gives a keyboard's handlers as leds.
KEYS = list(range(e.KEY_ESC, e.KEY_PAGEDOWN + 1))
LIGHTS = [e.LED_NUML, e.LED_CAPSL, e.LED_SCROLLL]


def main() -> None:
    """Plug the board in, say where it is, and keep it plugged in until stopped."""
    # Held for sigwait, so stopping lets it say it's gone; the device goes as it exits.
    stops = {signal.SIGTERM, signal.SIGINT}
    signal.pthread_sigmask(signal.SIG_BLOCK, stops)
    device = UInput(
        {e.EV_KEY: KEYS, e.EV_LED: LIGHTS},
        name=NAME,
        vendor=VENDOR,
        product=PRODUCT,
        bustype=e.BUS_USB,
        phys=PHYS,
    )
    Path(DEVICE).write_text(f"{device.device.path}\n")
    try:
        signal.sigwait(stops)
    finally:
        Path(DEVICE).unlink(missing_ok=True)


if __name__ == "__main__":
    main()
