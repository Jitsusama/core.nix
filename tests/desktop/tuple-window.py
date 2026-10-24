"""Show a window through X11 named only as Tuple names its windows.

    tuple-window PART

Its part is the WM_CLASS instance, Tuple its class, and it has no name. It stays until it's
stopped.
"""

import sys

from Xlib import X, display

# Its size, in pixels each way.
SIDE = 450


def main(argv: list[str]) -> None:
    """Map the window, then answer X until stopped."""
    shown = display.Display()
    screen = shown.screen()
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
    window.set_wm_class(argv[0], "Tuple")
    window.map()
    shown.flush()
    while True:
        shown.next_event()


if __name__ == "__main__":
    main(sys.argv[1:])
