"""Hold a loopback camera open as the camera's relay does, writing nothing to it.

tests/loopback-holders.nix runs this as the stand-in camera's service, with the loopback it
made as its argument, for tests/loopback-holders-vm-test.py.
"""

import os
import sys
import time


def main(device: str) -> None:
    """Hold the device open, readable and writable, until stopped."""
    held = os.open(device, os.O_RDWR)
    try:
        while True:
            time.sleep(60)
    finally:
        os.close(held)


if __name__ == "__main__":
    main(sys.argv[1])
