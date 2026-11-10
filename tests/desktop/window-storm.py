"""Show windows through X11 and retitle them without rest, as a crowd of agents' windows does.

    window-storm COUNT PER_SECOND

Each of COUNT windows is titled "storm N M", N its number and M how many times it's been
retitled, and they're retitled in turn PER_SECOND times a second, so niri tells the shell of a
window changing that often. It stays until it's stopped.
"""

import sys
import time

from Xlib import X, display

# Each window's size, in pixels each way: small, since only their titles matter.
SIDE = 64


def main(argv: list[str]) -> None:
    """Map the windows, then retitle one after another on the beat until stopped."""
    count, per_second = int(argv[0]), float(argv[1])
    shown = display.Display()
    screen = shown.screen()
    name = shown.intern_atom("_NET_WM_NAME")
    text = shown.intern_atom("UTF8_STRING")
    windows = []
    for _ in range(count):
        window = screen.root.create_window(
            0,
            0,
            SIDE,
            SIDE,
            0,
            screen.root_depth,
            X.InputOutput,
            X.CopyFromParent,
            background_pixel=screen.black_pixel,
        )
        window.set_wm_class("storm", "Storm")
        window.map()
        windows.append(window)
    beat = 1 / per_second
    retitled = 0
    next_beat = time.monotonic()
    while True:
        window = windows[retitled % count]
        title = f"storm {retitled % count} {retitled // count}"
        window.set_wm_name(title)
        window.change_property(name, text, 8, title.encode())
        shown.flush()
        retitled += 1
        next_beat += beat
        # Waits out the rest of the beat, answering X meanwhile, so it keeps
        # its pace however long a retitle took.
        while shown.pending_events():
            shown.next_event()
        time.sleep(max(0.0, next_beat - time.monotonic()))


if __name__ == "__main__":
    main(sys.argv[1:])
