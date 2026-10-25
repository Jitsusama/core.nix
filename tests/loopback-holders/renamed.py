r"""Take the name it's given, whatever bytes it holds, then hold a camera open.

tests/loopback-holders.nix installs this as renamed for tests/loopback-holders-vm-test.py,
which names itself as the relay, or by a name with a line break and a byte that isn't UTF-8,
to show how the holders' file names a program. The name is written with Python's escapes, as
`a\nb\xff`.
"""

import ctypes
import os
import sys
import time

# prctl's option for setting the calling thread's name, from linux/prctl.h.
PR_SET_NAME = 15


def main(name: str, device: str) -> None:
    """Take the escaped name as this process's, then hold the device open until stopped."""
    raw = name.encode().decode("unicode_escape").encode("latin-1")
    ctypes.CDLL(None).prctl(PR_SET_NAME, ctypes.c_char_p(raw), 0, 0, 0)
    held = os.open(device, os.O_RDONLY)
    try:
        while True:
            time.sleep(60)
    finally:
        os.close(held)


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
