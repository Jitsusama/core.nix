"""Stand in for a busy old shell that reads a chord late (tests/hands-over.sh).

It listens on the launcher's socket, as the old shell does, and sends a chord
down it as launcher-say would, then says it was sent. Only after LATE does it
read the chord, as a shell busy on a loaded machine does, saying the launcher
is open as the real one would once its line is read. A handover that took the
socket aside and waited a fixed moment had ended the old shell under that
opening (a gate, 2026-11-15).

Usage: late-chord.py SOCKET OPEN SENT
"""

import socket
import sys
import time
from pathlib import Path

# How long the chord waits unread: past the 50 ms the handover once waited,
# and within the second it now waits for a chord to be read.
LATE = 0.5


def main() -> None:
    """Listen, send a chord to it, and read it only once LATE has passed."""
    path, opened, sent = (Path(each) for each in sys.argv[1:])
    with socket.socket(socket.AF_UNIX) as listening:
        listening.bind(str(path))
        listening.listen()
        with socket.socket(socket.AF_UNIX) as chord:
            chord.connect(str(path))
            chord.sendall(b"toggle\n")
        sent.write_text("sent\n")
        time.sleep(LATE)
        heard, _ = listening.accept()
        # Open before the connection closes, as the real shell answers its IPC only once the
        # handler that read the line has opened it.
        with heard:
            heard.recv(4096)
            opened.write_text("true\n")
        # Listening on, as a shell does, until the case ends it.
        while True:
            time.sleep(1)


if __name__ == "__main__":
    main()
