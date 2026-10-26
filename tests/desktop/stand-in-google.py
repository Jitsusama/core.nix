"""Stand in for the google program, which would ask Google, on the socket it keeps.

It speaks the program's protocol (modules/google/src/state.rs): a shell that connects is told the
whole state as one line of JSON and kept, as the program keeps it for each change. The account is
ready, with one meeting, Standup, half an hour from when the stand-in starts, joined on Meet.
"""

import json
import socket
import sys
import threading
import time
from pathlib import Path

# Milliseconds in a minute, as the program counts times.
MINUTE = 60_000
# How far ahead the meeting starts, and how long it lasts.
AHEAD = 30 * MINUTE
LASTS = 30 * MINUTE


def state() -> bytes:
    """Give the state the program would say, one meeting on its calendar."""
    now = int(time.time() * 1000)
    meeting = {
        "id": "standup",
        "title": "Standup",
        "start": now + AHEAD,
        "end": now + AHEAD + LASTS,
        "allDay": False,
        "answer": "yes",
        "link": "https://meet.google.com/abc-defg-hij",
        "kind": "meet",
        "page": "https://calendar.google.com/calendar/event?eid=standup",
    }
    said = {"account": "ready", "meetings": [meeting], "heard": now}
    return f"{json.dumps(said)}\n".encode()


def keep(client: socket.socket, told: bytes) -> None:
    """Tell a shell the state, then hold it until it goes."""
    try:
        client.sendall(told)
        while client.recv(1024):
            pass
    except OSError:
        pass
    finally:
        client.close()


def main() -> None:
    """Listen on the socket the program is given, telling each shell that connects."""
    told = state()
    # One left by a stand-in before it would refuse the name.
    Path(sys.argv[1]).unlink(missing_ok=True)
    server = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    server.bind(sys.argv[1])
    server.listen()
    while True:
        client, _ = server.accept()
        threading.Thread(target=keep, args=(client, told), daemon=True).start()


if __name__ == "__main__":
    main()
