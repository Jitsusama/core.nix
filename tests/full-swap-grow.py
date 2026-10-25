"""Take memory as a program's grows, up to the MiB it's told, and keep it.

tests/full-swap.nix installs this as grow for tests/full-swap-vm-test.py. It takes another
16 MiB every half second. Each page is half random and half zeros, so it compresses about two
to one, as a program's memory does: data that doesn't compress costs zram a page of memory for
each page it takes, and memory runs out with swap half used, before oomd may act.
"""

import os
import sys
import time

# What each step takes, in MiB, as 4,096 pages of 4 KiB.
STEP = 16
PAGES = 4096
# How long it waits between steps, in seconds.
EVERY = 0.5


def main(cap: int) -> None:
    """Take memory a step at a time up to cap MiB, then keep it until stopped."""
    held: list[bytes] = []
    while len(held) * STEP < cap:
        pages = (os.urandom(2048) + bytes(2048) for _ in range(PAGES))
        held.append(b"".join(pages))
        time.sleep(EVERY)
    while True:
        time.sleep(1)


if __name__ == "__main__":
    main(int(sys.argv[1]))
