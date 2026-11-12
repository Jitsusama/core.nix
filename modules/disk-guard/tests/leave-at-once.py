"""Connect to a Unix socket and leave at once, as fast as it can.

A stand-in for the many short-lived programs that hand Nix's garbage
collector a temporary root, for nix-collects-beside-leaving-clients.sh.
It runs until it's stopped, waiting for the socket while it isn't there,
and then writes how many times it connected to the file named, so the
script can tell a round the collector never served from one it survived.
"""

from __future__ import annotations

import signal
import socket
import sys
import time
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from types import FrameType

# How long to wait for a socket that isn't there yet, in seconds.
ABSENT = 0.0002


def main() -> None:
    """Connect to argv[1] and leave until stopped, then write the count to argv[2]."""
    path, counted = sys.argv[1], Path(sys.argv[2])
    connected = 0

    def stop(_signal: int, _frame: FrameType | None) -> None:
        counted.write_text(f"{connected}\n")
        sys.exit(0)

    signal.signal(signal.SIGTERM, stop)
    while True:
        client = socket.socket(socket.AF_UNIX)
        try:
            client.connect(path)
            connected += 1
        except OSError:
            time.sleep(ABSENT)
        finally:
            client.close()


if __name__ == "__main__":
    main()
