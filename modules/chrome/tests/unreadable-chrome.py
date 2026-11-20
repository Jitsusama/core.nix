"""A stand-in for one of Chrome's sandboxed processes.

Its own account can't read its maps: it marks itself not dumpable, as
Chrome's sandbox does, takes Chrome's name, and waits until the file it's
given exists.

Usage: unreadable-chrome.py QUIT_FILE
"""

import ctypes
import pathlib
import sys
import time

PR_SET_DUMPABLE = 4
PR_SET_NAME = 15


def main() -> None:
    """Take Chrome's name, hide the maps, and wait to be told to quit."""
    quit_file = pathlib.Path(sys.argv[1])
    libc = ctypes.CDLL(None, use_errno=True)
    # Hidden before it's named, so a test that waits for the name never
    # finds its maps still readable.
    for option, value in ((PR_SET_DUMPABLE, 0), (PR_SET_NAME, ctypes.c_char_p(b"chrome"))):
        if libc.prctl(option, value, 0, 0, 0) != 0:
            sys.exit(f"prctl {option} failed: errno {ctypes.get_errno()}")
    while not quit_file.exists():
        time.sleep(0.1)


if __name__ == "__main__":
    main()
